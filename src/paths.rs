//! Where ZapFast keeps its files.
//!
//! Configuration, session state, and caches use separate standard platform
//! directories. Clearing a cache does not remove device keys.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct AppDirs {
    pub config: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    /// Per-user directory for the single-instance lock and control socket.
    pub runtime: PathBuf,
}

impl AppDirs {
    pub fn discover() -> Self {
        match Self::of(crate::identity::SLUG) {
            Some(dirs) => dirs,
            None => {
                let fallback = std::env::current_dir().unwrap_or_default();
                Self {
                    config: fallback.join("zapfast-business-config"),
                    state: fallback.join("zapfast-business-state"),
                    cache: fallback.join("zapfast-business-cache"),
                    runtime: fallback.join("zapfast-business-run"),
                }
            }
        }
    }

    /// Standard platform directories for the app.
    fn of(name: &str) -> Option<Self> {
        let project = ProjectDirs::from("me", "paolino", name)?;
        let state = project
            .state_dir()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| project.data_local_dir().to_path_buf());
        Some(Self {
            config: project.config_dir().to_path_buf(),
            runtime: runtime_dir(&project, &state),
            state,
            cache: project.cache_dir().to_path_buf(),
        })
    }

    /// Places all data under one directory for tests and temporary runs.
    pub fn under(root: &std::path::Path) -> Self {
        Self {
            config: root.join("config"),
            state: root.join("state"),
            cache: root.join("cache"),
            runtime: root.join("run"),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    /// whatsapp-rust device identity, Signal sessions, and state keys.
    /// Deleting this database unlinks the computer.
    pub fn session_db(&self) -> PathBuf {
        self.state.join("session.db")
    }

    /// Local message archive.
    pub fn archive_db(&self) -> PathBuf {
        self.state.join("archive.db")
    }

    /// Current-run log, replaced at startup.
    pub fn log_file(&self) -> PathBuf {
        self.state.join("zapfast-business.log")
    }

    /// Panic log written before process exit.
    pub fn panic_log(&self) -> PathBuf {
        self.state.join("panic.log")
    }

    /// Downloaded attachments keyed by message id.
    pub fn media_cache_dir(&self) -> PathBuf {
        self.cache.join("media")
    }

    /// Profile pictures keyed by chat.
    pub fn avatar_cache_dir(&self) -> PathBuf {
        self.cache.join("avatars")
    }

    /// Recent phone stickers keyed by file hash.
    pub fn sticker_cache_dir(&self) -> PathBuf {
        self.cache.join("stickers")
    }

    /// Saved stickers keyed by content hash. These are user data, not cache.
    pub fn saved_sticker_dir(&self) -> PathBuf {
        self.state.join("stickers")
    }

    /// ZapFast's copy of the chosen chat wallpaper image. User data, so it
    /// sits beside saved stickers rather than in the cache.
    pub fn wallpaper_file(&self, extension: &str) -> PathBuf {
        self.state.join(format!("wallpaper.{extension}"))
    }

    /// Cached profile-picture path. `full` selects the info-dialog size.
    pub fn avatar_file(&self, id: &str, full: bool) -> PathBuf {
        let stem: String = id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        self.avatar_cache_dir()
            .join(format!("{stem}{}.jpg", if full { "-full" } else { "" }))
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [&self.config, &self.state, &self.cache] {
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true);
            // Create new directories privately, even with a permissive umask.
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(dir)?;
            restrict_directory(dir)?;
        }
        Ok(())
    }
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Private directory for the instance lock and control channel. Uses
/// `$XDG_RUNTIME_DIR` where there is one. Elsewhere the directory sits
/// beside the state directory rather than inside it, because the state
/// directory must not exist before an earlier name's data is adopted.
fn runtime_dir(project: &ProjectDirs, state: &Path) -> PathBuf {
    // Flatpak gives each sandbox a private runtime directory and shares only
    // this one between instances of the app.
    #[cfg(target_os = "linux")]
    if let (Some(runtime), Some(id)) = (
        project.runtime_dir().and_then(Path::parent),
        std::env::var_os("FLATPAK_ID"),
    ) {
        return runtime.join("app").join(id);
    }
    if let Some(runtime) = project.runtime_dir() {
        return runtime.to_path_buf();
    }
    let mut name = state.file_name().unwrap_or_default().to_os_string();
    name.push(".run");
    state.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("zapfast-paths-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[cfg(unix)]
    #[test]
    fn ensure_restricts_base_directories() {
        use std::os::unix::fs::PermissionsExt;

        let root = root("permissions");
        let dirs = AppDirs::under(&root);
        dirs.ensure().unwrap();
        for path in [&dirs.config, &dirs.state, &dirs.cache] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ensure_repairs_existing_directory_permissions_without_changing_data() {
        use std::os::unix::fs::PermissionsExt;

        let root = root("existing-permissions");
        let dirs = AppDirs::under(&root);
        for path in [&dirs.config, &dirs.state, &dirs.cache] {
            std::fs::create_dir_all(path).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::write(path.join("fixture"), b"preserved").unwrap();
        }
        dirs.ensure().unwrap();
        for path in [&dirs.config, &dirs.state, &dirs.cache] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(std::fs::read(path.join("fixture")).unwrap(), b"preserved");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ensure_stops_when_an_application_directory_cannot_be_created() {
        let root = root("blocked-directory");
        let dirs = AppDirs::under(&root);
        std::fs::write(&dirs.state, b"existing file").unwrap();
        assert!(dirs.ensure().is_err());
        assert!(!dirs.cache.exists());
        assert_eq!(std::fs::read(&dirs.state).unwrap(), b"existing file");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn business_directories_are_disjoint_from_all_previous_names() {
        let business = AppDirs::discover();
        for name in ["zapfast", "fastsapp", "fastwhatsapp"] {
            let old = AppDirs::of(name).unwrap();
            for ours in [
                &business.config,
                &business.state,
                &business.cache,
                &business.runtime,
            ] {
                for theirs in [&old.config, &old.state, &old.cache, &old.runtime] {
                    assert!(!ours.starts_with(theirs));
                    assert!(!theirs.starts_with(ours));
                }
            }
        }
    }
}
