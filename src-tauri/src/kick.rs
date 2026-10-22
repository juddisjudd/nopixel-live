use serde::Deserialize;

use crate::model::{Channel, Platform};

pub async fn fetch_event(client: &reqwest::Client, event: &str) -> Result<Vec<Channel>, String> {
    if !is_valid_slug(event) {
        return Err(format!("\"{event}\" is not a valid event name"));
    }
    let url = format!("https://web.kick.com/api/v1/events/{event}/streams");
    let response = get(client, &url).await?;
    parse_event(&read_body(response).await?)
}

pub async fn fetch_channel(
    client: &reqwest::Client,
    slug: &str,
) -> Result<Option<Channel>, String> {
    if !is_valid_slug(slug) {
        return Ok(None);
    }
    let url = format!("https://kick.com/api/v2/channels/{slug}");
    let response = get(client, &url).await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    parse_channel(&read_body(response).await?)
}

async fn get(client: &reqwest::Client, url: &str) -> Result<reqwest::Response, String> {
    client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))
}

async fn read_body(response: reqwest::Response) -> Result<String, String> {
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("HTTP 403, probably a Cloudflare bot check".into());
    }
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    response
        .text()
        .await
        .map_err(|e| format!("unreadable response: {e}"))
}

pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn parse_event(body: &str) -> Result<Vec<Channel>, String> {
    let parsed: EventResponse =
        serde_json::from_str(body).map_err(|e| format!("unexpected event response: {e}"))?;
    Ok(parsed
        .data
        .streams
        .into_iter()
        .map(EventStream::into_channel)
        .collect())
}

pub fn parse_channel(body: &str) -> Result<Option<Channel>, String> {
    let parsed: ChannelResponse =
        serde_json::from_str(body).map_err(|e| format!("unexpected channel response: {e}"))?;
    Ok(parsed.into_channel())
}

#[derive(Deserialize)]
struct EventResponse {
    data: EventData,
}

#[derive(Deserialize)]
struct EventData {
    #[serde(default)]
    streams: Vec<EventStream>,
}

#[derive(Deserialize)]
struct EventStream {
    channel: EventChannel,
    livestream: EventLivestream,
    user: Option<EventUser>,
}

#[derive(Deserialize)]
struct EventChannel {
    slug: String,
}

#[derive(Deserialize)]
struct EventLivestream {
    title: Option<String>,
    view_count: Option<u64>,
    thumbnail: Option<EventThumbnail>,
}

#[derive(Deserialize)]
struct EventThumbnail {
    src: Option<String>,
}

#[derive(Deserialize)]
struct EventUser {
    username: Option<String>,
    profile_picture_url: Option<String>,
}

impl EventStream {
    fn into_channel(self) -> Channel {
        let (display_name, avatar) = match self.user {
            Some(user) => (user.username, user.profile_picture_url),
            None => (None, None),
        };
        Channel {
            platform: Platform::Kick,
            display_name: display_name.unwrap_or_else(|| self.channel.slug.clone()),
            login: self.channel.slug,
            title: self.livestream.title.unwrap_or_default(),
            viewers: self.livestream.view_count.unwrap_or(0),
            category: None,
            thumbnail: self.livestream.thumbnail.and_then(|t| t.src),
            avatar,
            started_at: None,
        }
    }
}

#[derive(Deserialize)]
struct ChannelResponse {
    slug: String,
    user: Option<ChannelUser>,
    livestream: Option<ChannelLivestream>,
}

#[derive(Deserialize)]
struct ChannelUser {
    username: Option<String>,
    profile_pic: Option<String>,
}

#[derive(Deserialize)]
struct ChannelLivestream {
    session_title: Option<String>,
    is_live: Option<bool>,
    viewer_count: Option<u64>,
    start_time: Option<String>,
    thumbnail: Option<ChannelThumbnail>,
    #[serde(default)]
    categories: Vec<ChannelCategory>,
}

#[derive(Deserialize)]
struct ChannelThumbnail {
    url: Option<String>,
}

#[derive(Deserialize)]
struct ChannelCategory {
    name: String,
}

impl ChannelResponse {
    fn into_channel(self) -> Option<Channel> {
        let live = self.livestream.filter(|l| l.is_live != Some(false))?;
        let (display_name, avatar) = match self.user {
            Some(user) => (user.username, user.profile_pic),
            None => (None, None),
        };
        Some(Channel {
            platform: Platform::Kick,
            display_name: display_name.unwrap_or_else(|| self.slug.clone()),
            login: self.slug,
            title: live.session_title.unwrap_or_default(),
            viewers: live.viewer_count.unwrap_or(0),
            category: live.categories.into_iter().next().map(|c| c.name),
            thumbnail: live.thumbnail.and_then(|t| t.url),
            avatar,
            started_at: live.start_time.map(|t| kick_time_to_rfc3339(&t)),
        })
    }
}

/// Kick sends UTC as `2026-10-16 19:43:02`; the UI parses RFC 3339.
fn kick_time_to_rfc3339(time: &str) -> String {
    if time.len() == 19 && time.as_bytes()[10] == b' ' {
        format!("{}T{}Z", &time[..10], &time[11..])
    } else {
        time.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_event_streams() {
        let channels = parse_event(include_str!("../fixtures/kick_event_streams.json")).unwrap();
        assert_eq!(channels.len(), 3);
        let first = &channels[0];
        assert_eq!(first.platform, Platform::Kick);
        assert_eq!(first.login, "xqc");
        assert_eq!(first.display_name, "xQc");
        assert!(first.viewers > 0);
        assert!(
            first
                .thumbnail
                .as_deref()
                .unwrap()
                .starts_with("https://images.kick.com/")
        );
        assert!(first.avatar.is_some());
    }

    #[test]
    fn parses_live_channel() {
        let channel = parse_channel(include_str!("../fixtures/kick_channel_live.json"))
            .unwrap()
            .unwrap();
        assert_eq!(channel.login, "zetark");
        assert_eq!(channel.display_name, "Zetark");
        assert_eq!(
            channel.category.as_deref(),
            Some("Grand Theft Auto V (GTA)")
        );
        assert_eq!(channel.started_at.as_deref(), Some("2026-10-16T19:43:02Z"));
    }

    #[test]
    fn offline_channel_is_none() {
        assert_eq!(
            parse_channel(include_str!("../fixtures/kick_channel_offline.json")).unwrap(),
            None
        );
    }

    #[test]
    fn rejects_unsafe_slugs() {
        assert!(is_valid_slug("nopixel-v"));
        assert!(is_valid_slug("ms_star"));
        assert!(!is_valid_slug("../api"));
        assert!(!is_valid_slug(""));
    }

    #[test]
    fn invalid_body_is_an_error() {
        assert!(parse_event("<html>Just a moment...</html>").is_err());
    }
}
