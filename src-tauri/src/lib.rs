use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use tauri::Manager;

fn sessions_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("sessions.json"))
}

#[tauri::command]
fn load_sessions(app: tauri::AppHandle) -> Result<String, String> {
    let path = sessions_path(&app)?;
    if !path.exists() {
        return Ok("[]".into());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_sessions(app: tauri::AppHandle, data: String) -> Result<(), String> {
    let path = sessions_path(&app)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
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
                "---\nname: ssh-manager\nwindows:\n  - tabs:\n      - title: {title}\n        layout:\n          cwd: {home}\n          commands:\n            - exec: {command}\n",
                home = home.display()
            );
            fs::write(dir.join("ssh-manager.yaml"), yaml).map_err(|e| e.to_string())?;
            Command::new("open").arg("warp://launch/ssh-manager").status()
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
    }
}
