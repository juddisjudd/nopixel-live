use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::model::{Channel, Platform};
use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerError {
    pub name: String,
    pub platform: Platform,
    pub message: String,
}

pub fn launch(app: AppHandle, settings: &Settings, channel: &Channel) -> Result<(), String> {
    let streamlink = find_streamlink(&settings.streamlink_path)
        .ok_or("Streamlink was not found. Install it, or set its path in Settings.")?;
    let player = find_player(&settings.player_path);
    let url = channel.platform.channel_url(&channel.login);

    let mut command = Command::new(&streamlink);
    command
        .args(build_args(settings, player.as_deref(), &url))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console_window(&mut command);
    let child = command
        .spawn()
        .map_err(|e| format!("Cannot start Streamlink at {}: {e}", streamlink.display()))?;

    let name = channel.display_name.clone();
    let platform = channel.platform;
    std::thread::spawn(move || {
        let message = match child.wait_with_output() {
            Ok(output) if output.status.success() => return,
            Ok(output) => last_error_line(&output.stdout, &output.stderr),
            Err(e) => e.to_string(),
        };
        let _ = app.emit(
            "player-error",
            PlayerError {
                name,
                platform,
                message,
            },
        );
    });
    Ok(())
}

pub fn build_args(settings: &Settings, player: Option<&Path>, url: &str) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    if let Some(player) = player {
        args.push("--player".into());
        args.push(player.into());
    }
    args.push("--title".into());
    args.push("{author} - {title}".into());
    args.extend(settings.extra_args.split_whitespace().map(OsString::from));
    args.push(url.into());
    args.push(settings.quality.clone().into());
    args
}

pub fn find_streamlink(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() {
        return Some(PathBuf::from(configured)).filter(|p| p.is_file());
    }
    find_on_path("streamlink").or_else(|| {
        first_file(&[
            env_join("ProgramFiles", r"Streamlink\bin\streamlink.exe"),
            env_join("LOCALAPPDATA", r"Programs\Streamlink\bin\streamlink.exe"),
        ])
    })
}

/// `None` lets Streamlink pick its own default player.
pub fn find_player(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() {
        return Some(PathBuf::from(configured)).filter(|p| p.is_file());
    }
    find_on_path("mpv")
        .or_else(|| find_on_path("vlc"))
        .or_else(|| {
            first_file(&[
                env_join("ProgramFiles", r"VideoLAN\VLC\vlc.exe"),
                env_join("ProgramFiles(x86)", r"VideoLAN\VLC\vlc.exe"),
            ])
        })
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(&exe))
        .find(|p| p.is_file())
}

fn env_join(var: &str, rest: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(|base| PathBuf::from(base).join(rest))
}

fn first_file(candidates: &[Option<PathBuf>]) -> Option<PathBuf> {
    candidates.iter().flatten().find(|p| p.is_file()).cloned()
}

fn last_error_line(stdout: &[u8], stderr: &[u8]) -> String {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let line = lines
        .iter()
        .rev()
        .find(|l| l.starts_with("error:"))
        .or(lines.last())
        .copied()
        .unwrap_or("Streamlink exited with an error.");
    line.chars().take(300).collect()
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: Vec<OsString>) -> Vec<String> {
        args.into_iter().map(|a| a.into_string().unwrap()).collect()
    }

    #[test]
    fn args_with_player_and_extras() {
        let settings = Settings {
            quality: "720p60,best".into(),
            extra_args: "--twitch-low-latency  --hls-live-edge 2".into(),
            ..Settings::default()
        };
        let args = build_args(
            &settings,
            Some(Path::new("C:/mpv/mpv.exe")),
            "https://www.twitch.tv/uhn",
        );
        assert_eq!(
            strings(args),
            [
                "--player",
                "C:/mpv/mpv.exe",
                "--title",
                "{author} - {title}",
                "--twitch-low-latency",
                "--hls-live-edge",
                "2",
                "https://www.twitch.tv/uhn",
                "720p60,best",
            ]
        );
    }

    #[test]
    fn args_without_player_let_streamlink_choose() {
        let args = strings(build_args(
            &Settings::default(),
            None,
            "https://kick.com/buddha",
        ));
        assert!(!args.contains(&"--player".to_string()));
        assert_eq!(args[args.len() - 2..], ["https://kick.com/buddha", "best"]);
    }

    #[test]
    fn error_line_prefers_streamlink_errors() {
        let out = b"[cli][info] Found matching plugin kick\nerror: No playable streams found on this URL\n";
        assert_eq!(
            last_error_line(out, b""),
            "error: No playable streams found on this URL"
        );
        assert_eq!(
            last_error_line(b"", b"something broke\n"),
            "something broke"
        );
        assert_eq!(
            last_error_line(b"", b""),
            "Streamlink exited with an error."
        );
    }

    #[test]
    fn missing_configured_paths_are_none() {
        assert_eq!(find_streamlink("Z:/nope/streamlink.exe"), None);
        assert_eq!(find_player("Z:/nope/vlc.exe"), None);
    }
}
