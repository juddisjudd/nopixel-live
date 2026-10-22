use serde::Deserialize;
use serde_json::{Map, Value};

use crate::model::{Channel, Platform};

const GQL_URL: &str = "https://gql.twitch.tv/gql";
// Twitch's public web client ID; Streamlink uses the same one.
const WEB_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
// Paging past the first page needs a browser integrity token, so coverage comes from several sort orders instead.
const SORTS: [&str; 3] = ["VIEWER_COUNT", "VIEWER_COUNT_ASC", "RECENT"];
const PAGE_SIZE: usize = 30;
const LOGINS_PER_LOOKUP: usize = 100;
const STREAM_FIELDS: &str = "title viewersCount createdAt previewImageURL(width: 440, height: 248) \
    game { displayName } broadcaster { login displayName profileImageURL(width: 70) }";

#[derive(Debug, Default, PartialEq)]
pub struct TwitchResult {
    pub tagged: Vec<Channel>,
    pub looked_up: Vec<Channel>,
}

pub async fn fetch(
    client: &reqwest::Client,
    tags: &[String],
    logins: &[String],
) -> Result<TwitchResult, String> {
    let Some(query) = build_query(tags, logins) else {
        return Ok(TwitchResult::default());
    };
    let response = client
        .post(GQL_URL)
        .header("Client-Id", WEB_CLIENT_ID)
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("unreadable response: {e}"))?;
    parse_response(&body)
}

pub fn build_query(tags: &[String], logins: &[String]) -> Option<String> {
    let mut parts = Vec::new();
    let tags = tags.iter().filter(|t| is_valid_tag(t));
    for (ti, tag) in tags.enumerate() {
        for (si, sort) in SORTS.iter().enumerate() {
            parts.push(format!(
                "t{ti}_{si}: streams(first: {PAGE_SIZE}, options: {{sort: {sort}, freeformTags: [\"{tag}\"]}}) \
                 {{ edges {{ node {{ {STREAM_FIELDS} }} }} }}"
            ));
        }
    }
    let logins: Vec<String> = logins.iter().filter_map(|l| to_twitch_login(l)).collect();
    for (ui, chunk) in logins.chunks(LOGINS_PER_LOOKUP).enumerate() {
        let list = chunk
            .iter()
            .map(|l| format!("\"{l}\""))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!(
            "u{ui}: users(logins: [{list}]) {{ stream {{ {STREAM_FIELDS} }} }}"
        ));
    }
    (!parts.is_empty()).then(|| format!("query {{ {} }}", parts.join(" ")))
}

pub fn parse_response(body: &Value) -> Result<TwitchResult, String> {
    let errors = body
        .get("errors")
        .and_then(Value::as_array)
        .map(|errors| {
            errors
                .iter()
                .filter_map(|e| e.get("message").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|joined| !joined.is_empty());
    let Some(data) = body.get("data").and_then(Value::as_object) else {
        return Err(errors.unwrap_or_else(|| "response had no data".into()));
    };
    if let Some(errors) = errors {
        log::warn!("Twitch returned partial data: {errors}");
    }

    let mut result = TwitchResult::default();
    for (alias, value) in sorted_entries(data) {
        if alias.starts_with('t') {
            let list: StreamList = serde_json::from_value(value.clone()).unwrap_or_default();
            result
                .tagged
                .extend(list.edges.into_iter().filter_map(|e| e.node.into_channel()));
        } else if alias.starts_with('u') {
            let users: Vec<Option<User>> =
                serde_json::from_value(value.clone()).unwrap_or_default();
            result.looked_up.extend(
                users
                    .into_iter()
                    .flatten()
                    .filter_map(|u| u.stream.and_then(Stream::into_channel)),
            );
        }
    }
    Ok(result)
}

fn sorted_entries(data: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut entries: Vec<_> = data.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

fn is_valid_tag(tag: &str) -> bool {
    !tag.is_empty() && tag.len() <= 25 && tag.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Twitch logins are 1 to 25 of `a-z`, `0-9` and `_`; Kick slugs use `-` where Twitch uses `_`.
fn to_twitch_login(name: &str) -> Option<String> {
    let login: String = name.trim().to_ascii_lowercase().replace('-', "_");
    let valid = !login.is_empty()
        && login.len() <= 25
        && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then_some(login)
}

#[derive(Deserialize, Default)]
struct StreamList {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Stream,
}

#[derive(Deserialize)]
struct User {
    stream: Option<Stream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stream {
    title: Option<String>,
    viewers_count: Option<u64>,
    created_at: Option<String>,
    #[serde(rename = "previewImageURL")]
    preview_image_url: Option<String>,
    game: Option<Game>,
    broadcaster: Option<Broadcaster>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Broadcaster {
    login: String,
    display_name: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
}

impl Stream {
    fn into_channel(self) -> Option<Channel> {
        let broadcaster = self.broadcaster?;
        Some(Channel {
            platform: Platform::Twitch,
            display_name: broadcaster
                .display_name
                .unwrap_or_else(|| broadcaster.login.clone()),
            login: broadcaster.login,
            title: self.title.unwrap_or_default(),
            viewers: self.viewers_count.unwrap_or(0),
            category: self.game.and_then(|g| g.display_name),
            thumbnail: self.preview_image_url,
            avatar: broadcaster.profile_image_url,
            started_at: self.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/twitch_gql.json")).unwrap()
    }

    #[test]
    fn query_has_one_alias_per_tag_and_sort() {
        let query = build_query(&["NoPixelV".into(), "NoPixel".into()], &[]).unwrap();
        assert_eq!(query.matches(": streams(").count(), 6);
        assert!(query.contains(
            "t1_2: streams(first: 30, options: {sort: RECENT, freeformTags: [\"NoPixel\"]})"
        ));
        assert!(!query.contains("users("));
    }

    #[test]
    fn query_drops_unsafe_tags_and_logins() {
        let query = build_query(
            &["NoPixel\"}) { x".into()],
            &["ms-star".into(), "bad\"login".into(), "Buddha".into()],
        )
        .unwrap();
        assert!(!query.contains("streams("));
        assert!(query.contains("users(logins: [\"ms_star\", \"buddha\"])"));
    }

    #[test]
    fn query_is_none_without_tags_or_logins() {
        assert_eq!(build_query(&[], &[]), None);
    }

    #[test]
    fn query_splits_large_lookups() {
        let logins: Vec<String> = (0..150).map(|i| format!("user{i}")).collect();
        let query = build_query(&[], &logins).unwrap();
        assert!(query.contains("u0: users("));
        assert!(query.contains("u1: users("));
    }

    #[test]
    fn parses_tagged_streams_and_lookups() {
        let result = parse_response(&fixture()).unwrap();
        assert_eq!(result.tagged.len(), 4);
        let uhn = &result.tagged[0];
        assert_eq!(uhn.platform, Platform::Twitch);
        assert_eq!(uhn.login, "uhn");
        assert_eq!(uhn.display_name, "UHN");
        assert_eq!(uhn.category.as_deref(), Some("Grand Theft Auto V"));
        assert!(uhn.viewers > 0);
        assert!(uhn.thumbnail.as_deref().unwrap().contains("live_user_uhn"));

        assert_eq!(result.looked_up.len(), 1);
        assert_eq!(result.looked_up[0].login, "buddha");
    }

    #[test]
    fn errors_without_data_fail() {
        let body = serde_json::json!({ "errors": [{ "message": "failed integrity check" }] });
        assert_eq!(parse_response(&body).unwrap_err(), "failed integrity check");
    }

    #[test]
    fn partial_errors_keep_data() {
        let mut body = fixture();
        body["errors"] = serde_json::json!([{ "message": "service error" }]);
        assert_eq!(parse_response(&body).unwrap().tagged.len(), 4);
    }
}
