//! One running ZapFast per user, through fastframe-instance.
//!
//! The crate holds the lock in the per-user runtime directory and serves the
//! private channel a later launch hands its request over (a socket only the
//! user can open, or a token-checked loopback port on Windows). ZapFast keeps
//! Its slot and wire prefix are exclusive to ZapFast Business, so the original
//! application can remain open at the same time.

use std::path::Path;
use std::sync::{Arc, Mutex};

/// Stable wire identity used only by Business releases.
const NAME: &str = crate::identity::SLUG;
/// What ZapFast answers a request it takes: `zapfast-business:ok` on the wire.
const OK: &str = "ok";

pub enum Outcome {
    /// This process owns the instance guard.
    Only(Guard),
    /// Another instance is running and received the request.
    Surfaced,
    /// Another instance holds the lock but did not answer.
    Unanswered,
}

/// Request from another launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlCommand {
    /// Shows or creates the window.
    Show,
    /// Reload local theme files without opening the window.
    ReloadThemes,
    /// Confirms an instance is running and changes nothing.
    Ping,
}

type Queue = Arc<Mutex<Vec<ControlCommand>>>;

/// Marks this process as the running instance until it exits.
pub struct Guard {
    /// Requests queued by later launches.
    commands: Queue,
    /// Held until the process exits.
    _claim: fastframe_instance::Guard,
}

impl Guard {
    /// Shared request queue drained by the app.
    pub fn commands(&self) -> Queue {
        Arc::clone(&self.commands)
    }
}

/// ZapFast's slot: its runtime directory, with the prefix older copies use.
fn slot(dir: &Path) -> fastframe_instance::Slot {
    fastframe_instance::Slot::at(dir, NAME)
}

/// Becomes the running instance, or hands `verb` to the one already running.
pub fn acquire(dir: &Path, waker: &crate::backend::Waker, verb: &str) -> Outcome {
    let commands = Queue::default();
    let claim = match claim(dir, verb, &commands, waker) {
        fastframe_instance::Claim::First(claim) => claim,
        fastframe_instance::Claim::Running(_) => return Outcome::Surfaced,
        // A copy that refuses the verb is reported as before, when it
        // simply did not answer.
        fastframe_instance::Claim::Unanswered | fastframe_instance::Claim::Declined => {
            return Outcome::Unanswered;
        }
    };
    // The Business fork must never probe or bind the legacy ZapFast port: the
    // original application may be running at the same time.
    Outcome::Only(Guard {
        commands,
        _claim: claim,
    })
}

/// Takes the slot, queueing what later launches ask for, or hands `verb` to
/// the copy that holds it.
fn claim(
    dir: &Path,
    verb: &str,
    commands: &Queue,
    waker: &crate::backend::Waker,
) -> fastframe_instance::Claim {
    let (commands, waker) = (Arc::clone(commands), waker.clone());
    slot(dir).claim(verb, move |request| {
        let command = parse(request)?;
        commands
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(command);
        waker.wake();
        Some(OK.to_owned())
    })
}

/// Sends one request to the running instance. An error of kind `NotFound`
/// or `ConnectionRefused` means none is running.
pub fn send(dir: &Path, verb: &str) -> std::io::Result<()> {
    slot(dir).send(verb).map(drop)
}

/// The verbs another launch may send. Anything else is declined.
fn parse(verb: &str) -> Option<ControlCommand> {
    match verb {
        "show" => Some(ControlCommand::Show),
        "reload-themes" => Some(ControlCommand::ReloadThemes),
        "ping" => Some(ControlCommand::Ping),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, TcpStream};

    #[test]
    fn only_our_own_verbs_are_understood() {
        assert_eq!(parse("show"), Some(ControlCommand::Show));
        assert_eq!(parse("ping"), Some(ControlCommand::Ping));
        assert_eq!(parse("reload-themes"), Some(ControlCommand::ReloadThemes));
        assert_eq!(parse("GET / HTTP/1.1"), None);
        assert_eq!(parse("frobnicate"), None);
        assert_eq!(parse(""), None);
    }

    fn connect(port: u16) -> TcpStream {
        TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("a connection")
    }

    /// Sends `line` the way a copy from before fastframe-instance does, with
    /// `token` first on Windows, and returns the raw reply.
    fn raw_request(dir: &Path, line: &str, token: Option<&str>) -> String {
        #[cfg(unix)]
        let mut stream = {
            let _ = token;
            std::os::unix::net::UnixStream::connect(dir.join("instance.sock")).unwrap()
        };
        #[cfg(not(unix))]
        let mut stream = {
            let key = std::fs::read_to_string(dir.join("instance.key")).unwrap();
            let mut key = key.lines();
            let port: u16 = key.next().unwrap().parse().unwrap();
            let written = key.next().unwrap().to_owned();
            let mut stream = connect(port);
            let token = token.map_or(written, str::to_owned);
            stream.write_all(format!("{token}\n").as_bytes()).unwrap();
            stream
        };
        stream.write_all(format!("{line}\n").as_bytes()).unwrap();
        let mut reply = String::new();
        let _ = stream.read_to_string(&mut reply);
        reply
    }

    /// A second launch reaches the first, whether it is this version or one
    /// from before fastframe-instance, which writes and expects the same
    /// lines: `zapfast-business:show` in, `zapfast-business:ok` out. Themes reload without a
    /// window. Unknown verbs are declined, and on Windows a wrong token gets
    /// no reply.
    #[test]
    fn a_second_launch_reaches_the_queue_on_the_old_wire() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join("runtime");
        let waker = crate::backend::Waker::default();
        let commands = Queue::default();
        let fastframe_instance::Claim::First(first) = claim(&dir, "show", &commands, &waker) else {
            panic!("the first launch takes the slot");
        };

        // A launch of this version.
        let second = claim(&dir, "show", &Queue::default(), &waker);
        assert!(matches!(second, fastframe_instance::Claim::Running(reply) if reply == OK));
        send(&dir, "reload-themes").expect("themes reload without a window");
        let declined = send(&dir, "frobnicate").unwrap_err();
        assert_eq!(declined.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(matches!(
            claim(&dir, "frobnicate", &Queue::default(), &waker),
            fastframe_instance::Claim::Declined
        ));

        // A launch of an older version.
        assert_eq!(
            raw_request(&dir, "zapfast-business:show", None),
            "zapfast-business:ok\n"
        );
        assert_eq!(
            raw_request(&dir, "zapfast-business:frobnicate", None),
            "zapfast-business!declined\n",
            "which an older copy reads as no answer"
        );
        #[cfg(not(unix))]
        assert_eq!(
            raw_request(&dir, "zapfast-business:show", Some(&"0".repeat(64))),
            "",
            "a wrong token is refused"
        );

        assert_eq!(
            *commands.lock().unwrap(),
            vec![
                ControlCommand::Show,
                ControlCommand::ReloadThemes,
                ControlCommand::Show,
            ]
        );
        drop(first);
    }
}
