use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const LATEST_RELEASE: &str =
    "https://api.github.com/repos/streamlink/windows-builds/releases/latest";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Download,
    Unpack,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Progress {
    pub stage: Stage,
    pub received: u64,
    pub total: u64,
}

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

pub fn exe_path(dir: &Path) -> PathBuf {
    dir.join("bin").join("streamlink.exe")
}

/// Installs the portable Streamlink build into `dir`, which needs no admin rights.
pub async fn install(
    client: &reqwest::Client,
    dir: &Path,
    on_progress: impl Fn(Progress),
) -> Result<PathBuf, String> {
    let asset = latest_portable(client).await?;
    let expected = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or("The Streamlink download has no checksum, so it cannot be checked.")?
        .to_ascii_lowercase();
    let (bytes, actual) = download(client, &asset, &on_progress).await?;
    if actual != expected {
        return Err("The Streamlink download was damaged. Try again.".into());
    }
    on_progress(Progress {
        stage: Stage::Unpack,
        received: 0,
        total: 0,
    });
    let dir = dir.to_path_buf();
    tauri::async_runtime::spawn_blocking(move || unpack(bytes, &dir))
        .await
        .map_err(|e| format!("Unpacking Streamlink stopped: {e}"))?
}

async fn latest_portable(client: &reqwest::Client) -> Result<Asset, String> {
    let response = client
        .get(LATEST_RELEASE)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Cannot reach GitHub: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub answered HTTP {}.", response.status()));
    }
    let release: Release = response
        .json()
        .await
        .map_err(|e| format!("Unexpected answer from GitHub: {e}"))?;
    release
        .assets
        .into_iter()
        .find(|a| is_portable_zip(&a.name))
        .ok_or_else(|| "The latest Streamlink release has no Windows download.".into())
}

fn is_portable_zip(name: &str) -> bool {
    name.starts_with("streamlink-") && name.ends_with("-x86_64.zip")
}

async fn download(
    client: &reqwest::Client,
    asset: &Asset,
    on_progress: &impl Fn(Progress),
) -> Result<(Vec<u8>, String), String> {
    let mut response = client
        .get(&asset.browser_download_url)
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}.", response.status()));
    }
    let total = asset.size;
    let step = (total / 100).max(1);
    let mut bytes = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    let mut hasher = Sha256::new();
    let mut reported = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Download failed: {e}"))?
    {
        hasher.update(&chunk);
        bytes.extend_from_slice(&chunk);
        let received = bytes.len() as u64;
        if received > total {
            return Err("The Streamlink download is larger than expected.".into());
        }
        if received - reported >= step || received == total {
            reported = received;
            on_progress(Progress {
                stage: Stage::Download,
                received,
                total,
            });
        }
    }
    Ok((bytes, hex(&hasher.finalize())))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unpack(bytes: Vec<u8>, dir: &Path) -> Result<PathBuf, String> {
    let staging = dir.with_extension("new");
    let result = unpack_into(bytes, &staging).and_then(|()| {
        remove_dir(dir)?;
        fs::rename(&staging, dir).map_err(|e| format!("Cannot move Streamlink into place: {e}"))?;
        Ok(exe_path(dir))
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn unpack_into(bytes: Vec<u8>, staging: &Path) -> Result<(), String> {
    remove_dir(staging)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("The Streamlink download is not a valid zip file: {e}"))?;
    archive
        .extract_unwrapped_root_dir(staging, zip::read::root_dir_common_filter)
        .map_err(|e| format!("Cannot unpack Streamlink: {e}"))?;
    if exe_path(staging).is_file() {
        Ok(())
    } else {
        Err("The Streamlink download did not contain streamlink.exe.".into())
    }
}

fn remove_dir(dir: &Path) -> Result<(), String> {
    match fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Cannot remove {}: {e}", dir.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;

    use super::*;

    fn zip_with(files: &[&str]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for name in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nopixel-live-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn unpacks_the_root_folder_into_place() {
        let dir = temp_dir("unpack");
        let bytes = zip_with(&[
            "streamlink-8.0.0-1-py314-x86_64/LICENSE.txt",
            "streamlink-8.0.0-1-py314-x86_64/bin/streamlink.exe",
        ]);
        assert_eq!(unpack(bytes, &dir).unwrap(), exe_path(&dir));
        assert!(dir.join("LICENSE.txt").is_file());
        assert!(!dir.with_extension("new").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replaces_an_old_copy() {
        let dir = temp_dir("replace");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("old.txt"), "old").unwrap();
        unpack(zip_with(&["root/bin/streamlink.exe"]), &dir).unwrap();
        assert!(!dir.join("old.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_a_zip_without_streamlink() {
        let dir = temp_dir("reject");
        assert!(unpack(zip_with(&["root/readme.txt"]), &dir).is_err());
        assert!(!dir.exists());
        assert!(!dir.with_extension("new").exists());
    }

    #[test]
    fn picks_the_portable_windows_zip() {
        assert!(is_portable_zip("streamlink-8.6.2-2-py314-x86_64.zip"));
        assert!(!is_portable_zip("streamlink-8.6.2-2-py314-x86_64.exe"));
        assert!(!is_portable_zip("streamlink-8.6.2-2-py314-aarch64.zip"));
    }

    #[test]
    fn hex_is_lowercase() {
        assert_eq!(hex(&[0x0a, 0xff]), "0aff");
    }

    #[tokio::test]
    #[ignore = "downloads Streamlink from GitHub"]
    async fn installs_the_latest_streamlink() {
        let dir = temp_dir("live-install");
        let client = reqwest::Client::builder()
            .user_agent("nopixel-live-test")
            .build()
            .unwrap();
        let exe = install(&client, &dir, |p| println!("{p:?}")).await.unwrap();
        assert!(exe.is_file());
        let version = std::process::Command::new(&exe)
            .arg("--version")
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&version.stdout));
        assert!(version.status.success());
        fs::remove_dir_all(&dir).unwrap();
    }
}
