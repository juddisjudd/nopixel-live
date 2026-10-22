use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::model::{Channel, Platform};
use crate::settings::Chat;

const CHAT_HOSTS: [&str; 2] = ["twitch.tv", "kick.com"];

pub fn open(app: &AppHandle, mode: Chat, channel: &Channel) -> Result<(), String> {
    let url = channel.platform.chat_url(&channel.login);
    match mode {
        Chat::Off => Ok(()),
        Chat::Browser => open_in_browser(app, &url),
        Chat::Window => open_window(app, channel, &url),
    }
}

pub fn open_in_browser(app: &AppHandle, url: &str) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("Cannot open the browser: {e}"))
}

fn open_window(app: &AppHandle, channel: &Channel, url: &str) -> Result<(), String> {
    let label = window_label(channel);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let url: Url = url.parse().map_err(|e| format!("Bad chat address: {e}"))?;
    let handle = app.clone();
    WebviewWindowBuilder::new(app, label, WebviewUrl::External(url))
        .title(format!(
            "{} chat on {}",
            channel.display_name,
            channel.platform.label()
        ))
        .inner_size(380.0, 720.0)
        .min_inner_size(300.0, 360.0)
        .on_navigation(move |url| {
            if url.scheme() == "about" || is_chat_host(url) {
                return true;
            }
            if matches!(url.scheme(), "http" | "https") {
                let _ = open_in_browser(&handle, url.as_str());
            }
            false
        })
        .build()
        .map_err(|e| format!("Cannot open the chat window: {e}"))?;
    Ok(())
}

fn window_label(channel: &Channel) -> String {
    let site = match channel.platform {
        Platform::Twitch => "twitch",
        Platform::Kick => "kick",
    };
    format!("chat-{site}-{}", channel.login.to_ascii_lowercase())
}

/// Links in chat leave the window, so it only ever shows Twitch or Kick pages.
fn is_chat_host(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| {
            CHAT_HOSTS
                .iter()
                .any(|d| host == *d || host.strip_suffix(d).is_some_and(|sub| sub.ends_with('.')))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn chat_hosts_include_subdomains_only() {
        assert!(is_chat_host(&url(
            "https://www.twitch.tv/popout/xqc/chat?popout="
        )));
        assert!(is_chat_host(&url("https://id.twitch.tv/oauth2/authorize")));
        assert!(is_chat_host(&url("https://kick.com/popout/xqc/chat")));
        assert!(!is_chat_host(&url("https://eviltwitch.tv/")));
        assert!(!is_chat_host(&url("https://twitch.tv.evil.com/")));
        assert!(!is_chat_host(&url("http://www.twitch.tv/")));
    }

    #[test]
    fn chat_urls_match_the_popout_pages() {
        assert_eq!(
            Platform::Twitch.chat_url("xqc"),
            "https://www.twitch.tv/popout/xqc/chat?popout="
        );
        assert_eq!(
            Platform::Kick.chat_url("xqc"),
            "https://kick.com/popout/xqc/chat"
        );
    }
}
