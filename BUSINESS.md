# ZapFast Business for Windows

Independent fork of [Carmine Paolino's ZapFast](https://github.com/crmne/zapfast).
Base: upstream 0.17.0, commit `76d0789`.
Branch: `business/windows-isolation`. Keep `main` as an upstream reference.

This variant also exposes WhatsApp Business labels and quick replies already
supported by the pinned protocol library. They synchronize through WhatsApp's
regular app-state collection. The first 0.17 Business start requests one full
snapshot so labels and quick replies created before the upgrade are populated.
The local database is only a Business-profile cache and never reads another
ZapFast profile.

## Isolation audit

| Surface | Business implementation |
| --- | --- |
| Executable | `zapfast-business.exe`; library name remains `zapfast` to minimize churn |
| Installer | Separate AppId `{B52DF836-982D-48F8-91B0-FE4D9E58B7F2}`; per-user `Programs\ZapFast Business` |
| Settings, session, archive, cache, logs | `ProjectDirs::from("me", "paolino", "zapfast-business")`; distinct fallback directories |
| Window persistence | Explicit `window.ron` under the Business state directory |
| Old profiles | Migration implementation and startup call removed; no old profile is opened, renamed or copied |
| Single instance | Lock and token file under Business runtime directory; `zapfast-business:` protocol; no legacy fixed-port probe/listener |
| Archive key | Windows Credential Manager service `io.github.DLangkamer.ZapFastBusiness.archive`, account derived from archive path; no legacy lookup |
| Device credentials | Fresh `session.db` in Business state directory, owned by whatsapp-rust |
| Window, taskbar, shortcuts, toasts | `ZapFast Business`, AppUserModelID `io.github.DLangkamer.ZapFastBusiness` |
| Tray and linked-device name | `ZapFast Business` |
| Login startup | Separate current-user Run value `ZapFast Business`, pointing to its own executable |
| GIPHY build key | Only `ZAPFAST_BUSINESS_GIPHY_KEY`; no original build-variable fallback |
| Updates | Signed releases from `DLangkamer/zapfast-business` only; distinct slug, marker files and embedded Ed25519 public key |
| Uninstall | Own executable/shortcuts/registration only; no legacy shortcut deletion, no shared Run-key deletion |

The installer refuses a destination containing `zapfast.exe`, `fastsapp.exe` or
`fastwhatsapp.exe`, and its close-applications filter targets only the Business
executable. Profile data and archive credentials are deliberately retained on
uninstall, so reinstalling does not destroy the archive.

On Windows the standard locations resolve to:

- `%APPDATA%\paolino\zapfast-business\config`: settings and custom themes.
- `%LOCALAPPDATA%\paolino\zapfast-business\data`: session, archive, stickers,
  `window.ron`, `zapfast-business.log` and `panic.log`.
- `%LOCALAPPDATA%\paolino\zapfast-business\cache`: downloaded media and avatars.
- `%LOCALAPPDATA%\paolino\zapfast-business\data.run`: instance lock and token.

Isolation here means independent application identities and storage, not a
security boundary between programs running under the same Windows user. As in
upstream, only the archive is SQLCipher-encrypted. Device session credentials,
settings and cached attachments are ordinary files in the user's profile.

## Build and verify

Install the Rust toolchain pinned by `rust-toolchain.toml`, Visual Studio C++
Build Tools with Windows SDK, CMake, Perl and Inno Setup 6.3 or newer. Use an
x64 developer shell. A short checkout and Cargo home avoid Windows path limits.

```powershell
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --all-features
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --locked --all-features --no-deps
cargo build --locked --release
iscc /DVersion=0.17.3 /DNumericVersion=0.17.3 /DArch=x86_64 /DBinary="$PWD\target\release\zapfast-business.exe" /DOutputDir="$PWD\dist" packaging\windows\zapfast.iss
```

Do not run the original multi-platform release workflow to publish this fork.
Its filenames and downstream packaging remain upstream references. Only the
Windows recipe above is adapted and supported by this branch.

Tests use synthetic files and mock credentials. They check separate directory
names, simultaneous instance guards, separate credential services, installer
identity and Business-only update-helper handling. They do not read real chats.

## Open, update and remove

Open **ZapFast Business** from Start. Link with the QR code using **WhatsApp
Business > Linked devices > Link a device** on your phone. Keep the personal
ZapFast running as usual. Linking, history synchronization and notification
delivery for the Business account require your phone and are not verified by
offline tests.

Open **Quick replies** from the `+` menu beside the composer. Typing `/` in the
composer searches the synchronized shortcuts; Enter or Tab inserts the selected
reply for review before sending. Labels are managed from the existing label
controls and can be assigned to direct chats or groups. Creating, editing or
deleting either feature synchronizes the change with WhatsApp Business.

In **Settings > Privacy**, turn off **Mark chats as read when opened** to
inspect and reply in direct chats or groups without clearing their unread
state. Recording and sending voice messages also preserve that state. Use the
chat's existing **Mark as read** action when you want to clear it.

Text messages can be scheduled from the `+` menu beside the composer. Scheduled
text is stored in the encrypted Business archive and uses the ordinary send
path when its time arrives. ZapFast Business must be running and connected at
that time. This first scheduling version supports one destination at a time;
management, retry controls and opt-in broadcast lists remain future work.

Stable releases are published at `DLangkamer/zapfast-business`. Version 0.17.2
is the trusted bootstrap: it embeds the Business public key and accepts only a
manifest signed by the corresponding GitHub Actions secret. The updater's slug,
asset names and marker files are `zapfast-business`; upstream packages and old
ZapFast/FastsApp aliases are never accepted. Users may enable daily checks and
automatic downloads in Settings and still choose when to restart.

Uninstall **ZapFast Business** from Windows Settings > Apps, or run its
`unins000.exe`. This leaves personal ZapFast untouched and retains Business
data. Unlink the Business device on your phone separately if desired.

`LICENSE`, `THIRD-PARTY-NOTICES.md` and bundled asset licenses remain unchanged.
