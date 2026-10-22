use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::install;
use crate::model::{Channel, Platform};
use crate::settings::{Player, Settings};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerError {
    pub name: String,
    pub platform: Platform,
    pub message: String,
}

pub fn launch(
    app: AppHandle,
    settings: &Settings,
    streamlink_dir: &Path,
    channel: &Channel,
) -> Result<(), String> {
    let streamlink =
        find_streamlink(&settings.streamlink_path, streamlink_dir).ok_or_else(|| {
            not_found(
                "Streamlink",
                &settings.streamlink_path,
                "Use the Install Streamlink button at the top of the window.",
            )
        })?;
    let player = find_player(settings.player, &settings.player_path).ok_or_else(|| {
        not_found(
            settings.player.label(),
            &settings.player_path,
            "Install it, or choose another player in Settings.",
        )
    })?;
    let url = channel.platform.channel_url(&channel.login);

    let mut command = Command::new(&streamlink);
    command
        .args(build_args(settings, &player, &url))
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

fn not_found(tool: &str, configured: &str, hint: &str) -> String {
    if configured.is_empty() {
        format!("{tool} is not installed. {hint}")
    } else {
        format!("{tool} was not found at {configured}. Check its location in Settings.")
    }
}

pub fn build_args(settings: &Settings, player: &Path, url: &str) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "--player".into(),
        player.into(),
        "--title".into(),
        "{author} - {title}".into(),
    ];
    args.extend(settings.extra_args.split_whitespace().map(OsString::from));
    args.push(url.into());
    args.push(settings.quality.clone().into());
    args
}

pub fn find_streamlink(configured: &str, managed_dir: &Path) -> Option<PathBuf> {
    configured_or(configured, || detect_streamlink(managed_dir))
}

pub fn find_player(player: Player, configured: &str) -> Option<PathBuf> {
    configured_or(configured, || detect_player(player))
}

pub fn detect_streamlink(managed_dir: &Path) -> Option<PathBuf> {
    find_on_path("streamlink").or_else(|| {
        first_file(&[
            env_join("ProgramFiles", r"Streamlink\bin\streamlink.exe"),
            env_join("LOCALAPPDATA", r"Programs\Streamlink\bin\streamlink.exe"),
            Some(install::exe_path(managed_dir)),
        ])
    })
}

pub fn detect_player(player: Player) -> Option<PathBuf> {
    match player {
        Player::Vlc => find_on_path("vlc")
            .or_else(|| {
                first_file(&[
                    env_join("ProgramFiles", r"VideoLAN\VLC\vlc.exe"),
                    env_join("ProgramFiles(x86)", r"VideoLAN\VLC\vlc.exe"),
                ])
            })
            .or_else(|| find_in_install_dirs(&install_roots(), "vlc", "vlc.exe")),
        Player::Mpv => find_on_path("mpv")
            .or_else(|| {
                first_file(&[
                    env_join("USERPROFILE", r"scoop\shims\mpv.exe"),
                    env_join("ProgramData", r"chocolatey\bin\mpv.exe"),
                    env_join("LOCALAPPDATA", r"Microsoft\WinGet\Links\mpv.exe"),
                ])
            })
            .or_else(|| find_in_install_dirs(&install_roots(), "mpv", "mpv.exe")),
    }
}

fn install_roots() -> Vec<PathBuf> {
    [
        env_join("ProgramFiles", ""),
        env_join("ProgramFiles(x86)", ""),
        env_join("LOCALAPPDATA", "Programs"),
        env_join("LOCALAPPDATA", r"Microsoft\WinGet\Packages"),
        env_join("USERPROFILE", r"scoop\apps"),
        env_join("ProgramData", r"chocolatey\lib"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Builds name their folders freely (`MPV Player`, `mpv-x86_64-v3-2026`, `mpv\current`), so any folder naming the player counts.
fn find_in_install_dirs(roots: &[PathBuf], name: &str, exe: &str) -> Option<PathBuf> {
    roots
        .iter()
        .flat_map(|root| subdirs(root))
        .filter(|dir| {
            dir.file_name()
                .is_some_and(|n| n.to_string_lossy().to_lowercase().contains(name))
        })
        .find_map(|dir| {
            let direct = dir.join(exe);
            if direct.is_file() {
                return Some(direct);
            }
            subdirs(&dir)
                .into_iter()
                .map(|sub| sub.join(exe))
                .find(|p| p.is_file())
        })
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn configured_or(configured: &str, detect: impl FnOnce() -> Option<PathBuf>) -> Option<PathBuf> {
    if configured.is_empty() {
        detect()
    } else {
        Some(PathBuf::from(configured)).filter(|p| p.is_file())
    }
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
            Path::new("C:/mpv/mpv.exe"),
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
        assert_eq!(
            find_streamlink("Z:/nope/streamlink.exe", Path::new("Z:/nope")),
            None
        );
        assert_eq!(find_player(Player::Vlc, "Z:/nope/vlc.exe"), None);
    }

    fn install_tree(name: &str, files: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "nopixel-live-players-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        root
    }

    #[test]
    fn finds_players_in_any_named_install_folder() {
        let root = install_tree(
            "named",
            &[
                r"MPV Player\mpv.exe",
                r"mpv.net\mpvnet.exe",
                r"VideoLAN\VLC\vlc.exe",
            ],
        );
        let roots = [root.clone()];
        assert_eq!(
            find_in_install_dirs(&roots, "mpv", "mpv.exe"),
            Some(root.join(r"MPV Player\mpv.exe"))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn finds_players_one_folder_down() {
        let root = install_tree("nested", &[r"mpv\current\mpv.exe"]);
        assert_eq!(
            find_in_install_dirs(std::slice::from_ref(&root), "mpv", "mpv.exe"),
            Some(root.join(r"mpv\current\mpv.exe"))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ignores_other_players() {
        let root = install_tree("other", &[r"mpv.net\mpvnet.exe", r"Tools\mpv.exe"]);
        assert_eq!(
            find_in_install_dirs(std::slice::from_ref(&root), "mpv", "mpv.exe"),
            None
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn not_found_points_to_the_fix() {
        assert_eq!(
            not_found("VLC", "", "Install it."),
            "VLC is not installed. Install it."
        );
        assert_eq!(
            not_found("VLC", "D:/vlc.exe", "Install it."),
            "VLC was not found at D:/vlc.exe. Check its location in Settings."
        );
    }
}
