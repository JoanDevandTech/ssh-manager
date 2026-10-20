# SSH Manager

Command-palette style launcher for your SSH sessions on macOS and Windows: type, Enter, your terminal opens connected.

## Download

Get the installer for your platform from the [latest release](https://github.com/JoanDevandTech/ssh-manager/releases/latest):

| Platform | File |
| --- | --- |
| macOS, Apple Silicon (M1 or later) | `SSH.Manager_<version>_aarch64.dmg` |
| macOS, Intel | `SSH.Manager_<version>_x64.dmg` |
| Windows | `SSH.Manager_<version>_x64-setup.exe` (or `SSH.Manager_<version>_x64_en-US.msi`) |

Ignore the `.app.tar.gz`, `.sig` and `latest.json` files: they are for the in-app updater.

### macOS

Open the `.dmg` and drag **SSH Manager** to Applications.

The app is ad-hoc signed but not notarized, so macOS blocks the first launch. Right-click the app → **Open** (on macOS 15 or later: try to open it once, then **System Settings → Privacy & Security → Open Anyway**), or remove the quarantine flag, which also works when macOS says the app is damaged:

```bash
xattr -dr com.apple.quarantine "/Applications/SSH Manager.app"
```

On the first **Connect**, macOS asks to allow SSH Manager to control Terminal / iTerm2 (Automation): allow it. Because the signature is ad-hoc, it asks again after each update. Warp needs nothing (it is opened through a launch configuration).

### Windows

Run the installer. SmartScreen may show "Windows protected your PC" because the build is unsigned: **More info → Run anyway**. You need the OpenSSH client, which is built into Windows 10/11.

## Use it in 30 seconds

1. **+ New** (or `⌘N` / `Ctrl+N`) → fill in Name, Host (both required), User, Port, Key Path and Terminal → **Save**. **Browse** opens the file picker in `~/.ssh`; leave Key Path empty to use your default key, your agent or `~/.ssh/config`.
2. Type to search, `↑` `↓` to pick, **Enter** (or **Connect Session**): the chosen terminal opens running `ssh -i <key> -p <port> user@host` (`-i` only with a key path, `-p` only for a port other than 22).
3. **Copy** puts that command on the clipboard.
4. **Edit** and **Delete** are in the detail panel on the right; its **Terminal** dropdown changes the terminal for that session.

## Features

- Instant search over name, host and user
- Per-session terminal:
  - macOS: Terminal, iTerm2, Warp, Alacritty\*, Ghostty\*
  - Windows: Windows Terminal, PowerShell, Command Prompt
- **Browse** button to pick the key file
- Copy the `ssh` command to the clipboard
- Sessions encrypted at rest, key in the OS keychain
- Signed in-app updates

\* Untested on a real install; reports welcome.

## Shortcuts

| Keys | Action |
| --- | --- |
| `⌘K` / `Ctrl+K` | Focus search |
| `↑` `↓` | Move selection |
| `Enter` | Connect |
| `⌘N` / `Ctrl+N` | New session |
| `Esc` | Cancel edit |

## Updates

On launch the app checks `releases/latest/download/latest.json` in this repository. When a newer version exists, a bar at the bottom offers **Update & restart**: the download is verified with its minisign signature against the public key embedded in the app, then installed, and the app restarts. No telemetry; nothing else is sent.

## Data and security

The app never stores passwords or private key contents. A session holds only its name, host, user, port, key path and terminal (plus a random id). Keys stay where they are (`~/.ssh`) and `ssh` itself reads them.

Sessions are stored in:

- macOS: `~/Library/Application Support/com.joandevandtech.sshmanager/sessions.json`
- Windows: `%APPDATA%\com.joandevandtech.sshmanager\sessions.json`

Since v0.1.2 the file is encrypted at rest with AES-256-GCM. The 256-bit key is generated on first use and kept in the OS keychain (macOS Keychain, Windows Credential Manager) under `com.joandevandtech.sshmanager` / `sessions-key`; the file and its folder are owner-only (`0600` / `0700`). A `sessions.json` written by an older version is encrypted automatically the first time the new version reads it; after that, an unencrypted file is refused.

Decrypted, a session looks like this:

```json
[
  {
    "id": "7f1c...",
    "name": "prod-gateway",
    "host": "203.0.113.10",
    "user": "ubuntu",
    "port": 22,
    "keyPath": "~/.ssh/id_ed25519",
    "terminal": "iterm2"
  }
]
```

`terminal` ids: `terminal`, `iterm2`, `warp`, `alacritty`, `ghostty` (macOS) · `wt`, `powershell`, `cmd` (Windows).

For Warp, the app writes a launch configuration to `~/.warp/launch_configurations/ssh-manager.yaml` with the `ssh` command of the session being opened; the file is owner-only and deleted about ten seconds after Warp is launched.

### What this does and does not protect

It protects the session list from other users and processes that cannot read your keychain, and from backups or sync tools copying the file in clear. It does not protect against code already running as your user, which can read the keychain and, more importantly, `~/.ssh`. For that, put a passphrase on your keys (`ssh-keygen -p -f ~/.ssh/<key>`) and keep full-disk encryption (FileVault / BitLocker) on. On macOS, because the app is ad-hoc signed, the keychain may ask to allow access again after an update.

## Development

Prerequisites: Node 20+, Rust stable and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run tauri dev     # runs the app
npm test              # JS unit tests
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build   # .dmg / .msi / .exe in src-tauri/target/release/bundle
```

`npm run tauri build` needs the updater signing key in the environment (`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`); `npm run tauri dev` does not.

Stack: Tauri 2, vanilla JS, Tailwind 4 compiled by `npm run css` (`tauri dev` and `tauri build` run it first). No frontend bundler.

## Releasing

```bash
git tag vX.Y.Z && git push origin vX.Y.Z
```

GitHub Actions (`release.yml`) builds macOS arm64 and x64 plus Windows, sets the app version from the tag, signs the updater artifacts, uploads everything (including `latest.json`) to a draft release, and publishes it with generated notes once all builds are in. The **Release** workflow can also be run by hand with a tag name (and, optionally, the branch or commit to build).

- Tags must be numeric `vX.Y.Z` (Windows installers need numeric versions).
- Repository secrets: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, generated with `npm run tauri signer generate`. The public key lives in `src-tauri/tauri.conf.json`.
- Keep the private key safe: without it, installed apps cannot be updated.

## License

MIT
