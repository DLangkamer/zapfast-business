# ZapFast Business for Windows

Independent fork of [Carmine Paolino's ZapFast](https://github.com/crmne/zapfast).
Base: upstream 0.19.0.
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
iscc /DVersion=0.19.5 /DNumericVersion=0.19.5 /DArch=x86_64 /DBinary="$PWD\target\release\zapfast-business.exe" /DOutputDir="$PWD\dist" packaging\windows\zapfast.iss
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

### Scheduled messages and visual calendar

Text and voice messages can be scheduled from the `+` menu beside the composer
or by clicking the calendar icon during audio recording. The visual calendar
dialog lets you pick the day, hour, and minute, with quick presets (+15m, +30m,
+1h, +3h, Tomorrow 09:00). Chats with scheduled messages display a visible
calendar badge and count directly in the chat list and header. The **Scheduled
messages** management center allows editing text/time or cancelling pending
items per account. Scheduled data stays in the SQLCipher-encrypted archive.

### Mass dispatch and broadcast lists

Open **Bulk dispatch** from the broadcast button in the chat list header,
from the `+` menu, or via right-click on chats ("Add to broadcast list..."):
- **Phone number paste**: Paste raw phone numbers separated by newlines, commas,
  or semicolons. Numbers are sanitized and Brazilian numbers automatically
  receive country code 55.
- **Segmented broadcast lists**: Create and maintain custom lists of contacts
  and groups stored in `business_broadcast_lists`.
- **Authentic PTT voice notes**: Transmit audio files (.mp3, .ogg, .wav, .m4a)
  converted natively to mono 48 kHz with WhatsApp 64-bar waveforms.
- **Intercalated delivery**: Adjustable delays (5s to 2 min) protect against
  rate limits, with total delivery time estimates. Dispatches can be started
  immediately or scheduled for a specific date and time.

### Quick replies with PTT voice notes and dynamic variables

Manage canned responses synced with WhatsApp Business with rich native features:
- **Recorded PTT voice notes in quick replies**: Store audio notes attached to quick replies
  (`/shortcut`). When invoked, they are dispatched as live voice notes (`ptt: true`, green mic,
  64-bar waveforms) indistinguishable from live recordings.
- **Dynamic variables**: Use `{{primeiro_nome}}`, `{{saudacao}}` (automatic morning/afternoon/evening
  greeting), `{{nome}}`, `{{data}}`, `{{hora}}`, and `{{telefone}}` in canned responses, composer,
  and mass dispatches. Each recipient automatically receives their personalized content.
- **Audio + text combination**: Send a PTT audio note and its accompanying personalized text message
  in a single action.


Stable releases are published at `DLangkamer/zapfast-business`. Version 0.17.2
is the trusted bootstrap: it embeds the Business public key and accepts only a
manifest signed by the corresponding GitHub Actions secret. The updater's slug,
asset names and marker files are `zapfast-business`; upstream packages and old
ZapFast/FastsApp aliases are never accepted. Users may enable daily checks and
automatic downloads in Settings and still choose when to restart.
Use **Settings > Check for updates now** to force an immediate GitHub check;
manual checks report both the up-to-date result and network errors.

Uninstall **ZapFast Business** from Windows Settings > Apps, or run its
`unins000.exe`. This leaves personal ZapFast untouched and retains Business
data. Unlink the Business device on your phone separately if desired.

`LICENSE`, `THIRD-PARTY-NOTICES.md` and bundled asset licenses remain unchanged.
