# ZapFast Business for Windows

Independent fork of [Carmine Paolino's ZapFast](https://github.com/crmne/zapfast).
Base: upstream 0.16.5, commit `3f958708b5e6b2381a768fad332f97376fa064e5`.
Branch: `business/windows-isolation`. Keep `main` as an upstream reference.
No protocol changes or extra WhatsApp Business features are included.

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
| Updates | Disabled at settings/UI, scheduler and updater factory; CLI rejects all upstream helper flags before touching state |
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
iscc /DVersion=0.16.5 /DNumericVersion=0.16.5 /DArch=x86_64 /DBinary="$PWD\target\release\zapfast-business.exe" /DOutputDir="$PWD\dist" packaging\windows\zapfast.iss
```

Do not run the original multi-platform release workflow to publish this fork.
Its filenames and downstream packaging remain upstream references. Only the
Windows recipe above is adapted and supported by this branch.

Tests use synthetic files and mock credentials. They check separate directory
names, simultaneous instance guards, separate credential services, installer
identity and rejection of update-helper flags. They do not read real chats.

## Open, update and remove

Open **ZapFast Business** from Start. Link with the QR code using **WhatsApp
Business > Linked devices > Link a device** on your phone. Keep the personal
ZapFast running as usual. Linking, history synchronization and notification
delivery for the Business account require your phone and are not verified by
offline tests.

Updates are manual: fetch upstream, merge or rebase changes onto this branch,
review the isolation table and tests, rebuild, then run the Business installer.
Preserve the Business AppId, paths and credential service across upgrades.
Never install an upstream binary over the Business executable. Do not enable
self-update until a separate signing key and release channel exist.

Uninstall **ZapFast Business** from Windows Settings > Apps, or run its
`unins000.exe`. This leaves personal ZapFast untouched and retains Business
data. Unlink the Business device on your phone separately if desired.

`LICENSE`, `THIRD-PARTY-NOTICES.md` and bundled asset licenses remain unchanged.
