# SSH Manager

A tiny command-palette style launcher for your SSH sessions. macOS and Windows.

Type to search, press **Enter**, and the terminal you chose opens with the right `ssh` command.
No passwords are ever stored: sessions point at a key file (or rely on your agent / `~/.ssh/config`), and the terminal handles the rest.

## Features

- Instant search over session name, host and user; arrow keys + Enter to connect
- Per-session terminal:
  - macOS: Terminal, iTerm2, Warp, Alacritty\*, Ghostty\*
  - Windows: Windows Terminal, PowerShell, Command Prompt
- Copy the `ssh` command to the clipboard
- Sessions live in a plain JSON file you can edit, back up or sync yourself
- Checks for new versions on launch and updates itself in one click (signed updates)

\* not yet verified on a real install — reports welcome.

## Install

Grab the latest build from [Releases](../../releases).

- **macOS**: builds are ad-hoc signed, not notarized. If macOS says the app is damaged, run once:
  `xattr -dr com.apple.quarantine "/Applications/SSH Manager.app"`.
  The first time you connect, macOS asks for permission to control Terminal / iTerm2 (Automation); because the signature is ad-hoc, it asks again after each update. Warp is driven through a launch configuration, no permission needed.
- **Windows**: `.msi` or `.exe` installer. OpenSSH client must be installed (it is by default on Windows 10/11).

## Updates

On launch the app checks `releases/latest/download/latest.json` on this repository. When a newer version exists, a bar at the bottom offers **Update & restart**: the installer is downloaded, its minisign signature verified against the public key embedded in the app, and the app restarts. No telemetry, nothing else is sent.

## Shortcuts

| Keys | Action |
| --- | --- |
| `⌘K` / `Ctrl+K` | Focus search |
| `↑` `↓` | Move selection |
| `Enter` | Connect |
| `⌘N` / `Ctrl+N` | New session |
| `Esc` | Cancel edit |

## Data

Sessions are stored as JSON:

- macOS: `~/Library/Application Support/com.joandevandtech.sshmanager/sessions.json`
- Windows: `%APPDATA%\com.joandevandtech.sshmanager\sessions.json`

```json
[
  {
    "id": "7f1c...",
    "name": "aws-prod-gateway",
    "host": "54.210.12.89",
    "user": "ubuntu",
    "port": 22,
    "keyPath": "~/.ssh/krom_prod",
    "terminal": "iterm2"
  }
]
```

`terminal` ids: `terminal`, `iterm2`, `warp`, `alacritty`, `ghostty` (macOS) · `wt`, `powershell`, `cmd` (Windows).

## Development

Prerequisites: Node 20+, Rust stable and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run tauri dev     # runs the app
npm test              # JS unit tests
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build   # .dmg / .msi in src-tauri/target/release/bundle
```

Stack: Tauri 2, vanilla JS, Tailwind 4 (compiled by `npm run css`). No frontend bundler.

`npm run tauri build` needs the updater signing key in the environment (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`); `npm run tauri dev` does not.

### Releasing

```bash
git tag v0.2.0 && git push origin v0.2.0
```

GitHub Actions (`release.yml`) builds macOS arm64 + x64 and Windows, sets the app version from the tag, signs the updater artifacts, uploads everything plus `latest.json` to a draft release and publishes it with generated notes once all three builds are in. The *Release* workflow can also be run by hand with a tag name. Tags must be `vX.Y.Z` (Windows installers need numeric versions).

Secrets required in the repository: `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (generated with `npm run tauri signer generate`). The public key lives in `src-tauri/tauri.conf.json`. Keep the private key safe: without it, installed apps cannot be updated.

## License

MIT
