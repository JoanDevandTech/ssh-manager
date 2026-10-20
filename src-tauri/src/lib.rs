use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use tauri::Manager;

const KEYRING_SERVICE: &str = "com.joandevandtech.sshmanager";
const KEYRING_USER: &str = "sessions-key";

#[derive(Serialize, Deserialize)]
struct Envelope {
    v: u8,
    nonce: String,
    data: String,
}

// Random 256-bit key, created once and kept in the OS keychain.
// Also reports whether this call created it.
fn cipher() -> Result<(Aes256Gcm, bool), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())?;
    let (key_b64, created) = match entry.get_password() {
        Ok(k) => (k, false),
        Err(keyring::Error::NoEntry) => {
            let k = B64.encode(Aes256Gcm::generate_key(OsRng));
            entry.set_password(&k).map_err(|e| e.to_string())?;
            (k, true)
        }
        Err(e) => return Err(e.to_string()),
    };
    let bytes = B64.decode(key_b64).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("Invalid sessions key in keychain".into());
    }
    Ok((Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&bytes)), created))
}

fn encrypt_with(cipher: &Aes256Gcm, plain: &str) -> Result<String, String> {
    let nonce = Aes256Gcm::generate_nonce(OsRng);
    let data = cipher.encrypt(&nonce, plain.as_bytes()).map_err(|e| e.to_string())?;
    serde_json::to_string(&Envelope { v: 1, nonce: B64.encode(nonce), data: B64.encode(data) })
        .map_err(|e| e.to_string())
}

fn decrypt_with(cipher: &Aes256Gcm, envelope: &str) -> Result<String, String> {
    let env: Envelope = serde_json::from_str(envelope).map_err(|e| e.to_string())?;
    let nonce = B64.decode(env.nonce).map_err(|e| e.to_string())?;
    let data = B64.decode(env.data).map_err(|e| e.to_string())?;
    if nonce.len() != 12 {
        return Err("Corrupted sessions file (bad nonce)".into());
    }
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce), data.as_ref())
        .map_err(|_| "Could not decrypt sessions file (key mismatch or corrupted file)".to_string())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}

fn is_plain_json(raw: &str) -> bool {
    raw.trim_start().starts_with('[')
}

fn has_control_chars(s: &str) -> bool {
    s.chars().any(char::is_control)
}

// YAML double-quoted scalar.
#[cfg(any(target_os = "macos", test))]
fn yaml_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

// Atomic write, owner-only on unix.
fn write_private(path: &Path, contents: &str) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp).map_err(|e| e.to_string())?;
        f.write_all(contents.as_bytes()).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(unix)]
fn restrict(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn restrict(_path: &Path, _mode: u32) {}

fn sessions_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    restrict(&dir, 0o700);
    Ok(dir.join("sessions.json"))
}

#[tauri::command]
fn load_sessions(app: tauri::AppHandle) -> Result<String, String> {
    let path = sessions_path(&app)?;
    if !path.exists() {
        return Ok("[]".into());
    }
    restrict(&path, 0o600);
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (cipher, key_created) = cipher()?;
    if is_plain_json(&raw) {
        // Pre-encryption file: accepted only on the first run of an encrypting build.
        if !key_created {
            return Err("sessions.json is not encrypted but an encryption key already exists; refusing to load it. Restore a backup or delete the file.".into());
        }
        write_private(&path, &encrypt_with(&cipher, &raw)?)?;
        return Ok(raw);
    }
    decrypt_with(&cipher, &raw)
}

#[tauri::command]
fn save_sessions(app: tauri::AppHandle, data: String) -> Result<(), String> {
    let path = sessions_path(&app)?;
    let (cipher, _) = cipher()?;
    write_private(&path, &encrypt_with(&cipher, &data)?)
}

// ponytail: plain replace; enough for `-i ~/.ssh/key`
fn expand_home(command: &str, home: &str) -> String {
    command.replace("~/", &format!("{}/", home.trim_end_matches(['/', '\\'])))
}

fn safe_title(title: &str) -> String {
    let t: String = title
        .chars()
        .filter(|c| c.is_alphanumeric() || " -_.".contains(*c))
        .collect();
    let t = t.trim().to_string();
    if t.is_empty() { "ssh".into() } else { t }
}

#[tauri::command]
fn open_terminal(app: tauri::AppHandle, terminal: String, title: String, command: String) -> Result<(), String> {
    if has_control_chars(&command) || has_control_chars(&title) {
        return Err("Session fields must not contain control characters".into());
    }
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let command = expand_home(&command, &home.to_string_lossy());
    let title = safe_title(&title);
    let status = spawn(&home, &terminal, &title, &command)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{terminal} launcher exited with {status}"))
    }
}

#[cfg(target_os = "macos")]
fn spawn(home: &Path, terminal: &str, title: &str, command: &str) -> Result<ExitStatus, String> {
    let osa = |lines: &[String]| {
        let mut c = Command::new("osascript");
        for l in lines {
            c.arg("-e").arg(l);
        }
        c.status()
    };
    // AppleScript string literal
    let esc = command.replace('\\', "\\\\").replace('"', "\\\"");
    let result = match terminal {
        "terminal" => osa(&[
            format!("tell application \"Terminal\" to do script \"{esc}\""),
            "tell application \"Terminal\" to activate".into(),
        ]),
        "iterm2" => osa(&[
            "tell application \"iTerm\" to activate".into(),
            "tell application \"iTerm\" to create window with default profile".into(),
            format!("tell application \"iTerm\" to tell current session of current window to write text \"{esc}\""),
        ]),
        "warp" => {
            let dir = home.join(".warp/launch_configurations");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let yaml = format!(
                "---\nname: ssh-manager\nwindows:\n  - tabs:\n      - title: {ttl}\n        layout:\n          cwd: {cwd}\n          commands:\n            - exec: {exec}\n",
                ttl = yaml_str(title),
                cwd = yaml_str(&home.to_string_lossy()),
                exec = yaml_str(command)
            );
            let file = dir.join("ssh-manager.yaml");
            write_private(&file, &yaml)?;
            let status = Command::new("open").arg("warp://launch/ssh-manager").status();
            // The config holds the ssh command: give Warp time to read it, then drop it.
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(10));
                let _ = fs::remove_file(file);
            });
            status
        }
        "alacritty" => Command::new("open")
            .args(["-na", "Alacritty", "--args", "-e", "/bin/zsh", "-lc", command])
            .status(),
        "ghostty" => Command::new("open")
            .args(["-na", "Ghostty", "--args", "-e", "/bin/zsh", "-lc", command])
            .status(),
        _ => return Err(format!("Unknown terminal: {terminal}")),
    };
    result.map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn spawn(_home: &Path, terminal: &str, title: &str, command: &str) -> Result<ExitStatus, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let line = match terminal {
        "wt" => format!("wt.exe new-tab --title \"{title}\" cmd /K {command}"),
        "powershell" => format!("powershell.exe -NoExit -Command \"{}\"", command.replace('"', "'")),
        "cmd" => format!("cmd.exe /K {command}"),
        _ => return Err(format!("Unknown terminal: {terminal}")),
    };
    Command::new("cmd.exe")
        .raw_arg(format!("/C start \"\" {line}"))
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| e.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn spawn(_home: &Path, _terminal: &str, _title: &str, _command: &str) -> Result<ExitStatus, String> {
    Err("Linux is not supported yet".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![load_sessions, save_sessions, open_terminal])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_home_and_sanitizes_title() {
        assert_eq!(expand_home("ssh -i ~/.ssh/k u@h", "/Users/me"), "ssh -i /Users/me/.ssh/k u@h");
        assert_eq!(expand_home("ssh -i ~/.ssh/k u@h", "C:\\Users\\me\\"), "ssh -i C:\\Users\\me/.ssh/k u@h");
        assert_eq!(safe_title("aws: prod \"gw\""), "aws prod gw");
        assert_eq!(safe_title("\"\""), "ssh");
        assert!(has_control_chars("ssh u@h\n"));
        assert!(!has_control_chars("ssh -i \"/My Keys/k\" u@h"));
        assert_eq!(yaml_str("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(yaml_str("- x"), "\"- x\"");
    }

    #[test]
    fn encrypts_and_decrypts_sessions() {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&[7u8; 32]));
        let plain = r#"[{"id":"1","name":"prod","host":"10.0.0.1"}]"#;
        let envelope = encrypt_with(&cipher, plain).unwrap();
        assert!(!is_plain_json(&envelope));
        assert!(!envelope.contains("10.0.0.1"));
        assert_eq!(decrypt_with(&cipher, &envelope).unwrap(), plain);
        assert!(is_plain_json(plain));

        let short_nonce = envelope.replacen("\"nonce\":\"", "\"nonce\":\"AAAA", 1);
        assert!(decrypt_with(&cipher, &short_nonce).is_err());

        let other = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&[8u8; 32]));
        assert!(decrypt_with(&other, &envelope).is_err());

        let tampered = envelope.replacen("\"data\":\"", "\"data\":\"AAAA", 1);
        assert!(decrypt_with(&cipher, &tampered).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn private_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("ssh-manager-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sessions.json");
        write_private(&file, "x").unwrap();
        assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::read_to_string(&file).unwrap(), "x");
        let _ = fs::remove_dir_all(dir);
    }
}
