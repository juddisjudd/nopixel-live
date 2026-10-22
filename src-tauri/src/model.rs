use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Twitch,
    Kick,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Platform::Twitch => "Twitch",
            Platform::Kick => "Kick",
        }
    }

    pub fn channel_url(self, login: &str) -> String {
        match self {
            Platform::Twitch => format!("https://www.twitch.tv/{login}"),
            Platform::Kick => format!("https://kick.com/{login}"),
        }
    }

    pub fn chat_url(self, login: &str) -> String {
        match self {
            Platform::Twitch => format!("https://www.twitch.tv/popout/{login}/chat?popout="),
            Platform::Kick => format!("https://kick.com/popout/{login}/chat"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub platform: Platform,
    pub login: String,
    pub display_name: String,
    pub title: String,
    pub viewers: u64,
    pub category: Option<String>,
    pub thumbnail: Option<String>,
    pub avatar: Option<String>,
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Streamer {
    pub key: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub viewers: u64,
    pub channels: Vec<Channel>,
}

impl Streamer {
    pub fn channel(&self, platform: Platform) -> Option<&Channel> {
        self.channels.iter().find(|c| c.platform == platform)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub streamers: Vec<Streamer>,
    pub updated_at: Option<u64>,
    pub errors: Vec<String>,
}

impl Snapshot {
    pub fn streamer(&self, key: &str) -> Option<&Streamer> {
        self.streamers.iter().find(|s| s.key == key)
    }
}

/// Lowercase alphanumerics only, so `ms_star`, `ms-star` and `MsStar` share one key.
pub fn name_key(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Kick names categories like `Grand Theft Auto V (GTA)`, so the parenthetical is ignored.
pub fn same_category(a: &str, b: &str) -> bool {
    fn base(name: &str) -> String {
        let name = match name.find(" (") {
            Some(i) if name.trim_end().ends_with(')') => &name[..i],
            _ => name,
        };
        name.trim().to_lowercase()
    }
    base(a) == base(b)
}

/// Channels without a category come from Kick's event list, which only holds that event's category.
pub fn in_category(channel: &Channel, category: &str) -> bool {
    category.trim().is_empty()
        || channel
            .category
            .as_deref()
            .is_none_or(|c| same_category(c, category))
}

pub fn is_nopixel_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    if lower.contains("nopixel") || lower.contains("no pixel") {
        return true;
    }
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| matches!(word, "np" | "npv" | "np5"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_key_ignores_case_and_separators() {
        assert_eq!(name_key("Ms_Star"), "msstar");
        assert_eq!(name_key("ms-star"), "msstar");
        assert_eq!(name_key("xQc"), "xqc");
    }

    #[test]
    fn nopixel_titles_match() {
        assert!(is_nopixel_title(
            "Luciano DiCenzo | NoPixel V | Old Country"
        ));
        assert!(is_nopixel_title("NOPIXEL V| ERNESTO"));
        assert!(is_nopixel_title("chatting then NP later"));
        assert!(is_nopixel_title("[NPV] heist day"));
        assert!(is_nopixel_title("No Pixel grind"));
    }

    #[test]
    fn categories_match_across_sites() {
        assert!(same_category(
            "Grand Theft Auto V (GTA)",
            "Grand Theft Auto V"
        ));
        assert!(same_category("grand theft auto v", "Grand Theft Auto V"));
        assert!(!same_category("Grand Theft Auto VI", "Grand Theft Auto V"));
        assert!(!same_category("Just Chatting", "Grand Theft Auto V"));
    }

    #[test]
    fn category_filter() {
        let mut channel = Channel {
            platform: Platform::Twitch,
            login: "burn".into(),
            display_name: "Burn".into(),
            title: "NoPixel later".into(),
            viewers: 1,
            category: Some("Just Chatting".into()),
            thumbnail: None,
            avatar: None,
            started_at: None,
        };
        assert!(!in_category(&channel, "Grand Theft Auto V"));
        assert!(in_category(&channel, " "));
        channel.category = None;
        assert!(in_category(&channel, "Grand Theft Auto V"));
        channel.category = Some("Grand Theft Auto V (GTA)".into());
        assert!(in_category(&channel, "Grand Theft Auto V"));
    }

    #[test]
    fn other_titles_do_not_match() {
        assert!(!is_nopixel_title("Ranked grind, road to Radiant"));
        assert!(!is_nopixel_title("snPping"));
        assert!(!is_nopixel_title("Prodigy RP | cop shift"));
    }
}
