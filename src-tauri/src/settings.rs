use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::kick::is_valid_slug;
use crate::model::{Platform, Streamer, name_key};

const MIN_POLL_SECONDS: u64 = 30;
const MAX_POLL_SECONDS: u64 = 600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub streamlink_path: String,
    pub player: Player,
    pub player_path: String,
    pub chat: Chat,
    pub quality: String,
    pub extra_args: String,
    pub preferred_platform: Platform,
    pub poll_seconds: u64,
    pub twitch_tags: Vec<String>,
    pub kick_event: String,
    pub category: String,
    pub notify_favorites: bool,
    pub close_to_tray: bool,
    pub favorites: Vec<Favorite>,
    pub links: Vec<Link>,
    pub added: Vec<AddedChannel>,
    pub hidden: Vec<Hidden>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            streamlink_path: String::new(),
            player: Player::Vlc,
            player_path: String::new(),
            chat: Chat::Window,
            quality: "best".into(),
            extra_args: String::new(),
            preferred_platform: Platform::Twitch,
            poll_seconds: 60,
            twitch_tags: vec!["NoPixelV".into(), "NoPixel".into()],
            kick_event: "nopixel-v".into(),
            category: "Grand Theft Auto V".into(),
            notify_favorites: true,
            close_to_tray: true,
            favorites: Vec::new(),
            links: Vec::new(),
            added: Vec::new(),
            hidden: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Player {
    Vlc,
    Mpv,
}

impl Player {
    pub fn label(self) -> &'static str {
        match self {
            Player::Vlc => "VLC",
            Player::Mpv => "mpv",
        }
    }

    pub fn download_url(self) -> &'static str {
        match self {
            Player::Vlc => "https://www.videolan.org/vlc/",
            Player::Mpv => "https://mpv.io/installation/",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chat {
    Window,
    Browser,
    Off,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub twitch: Option<String>,
    #[serde(default)]
    pub kick: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub twitch: String,
    pub kick: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddedChannel {
    pub platform: Platform,
    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hidden {
    pub key: String,
    pub name: String,
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(settings) => settings.sanitized(),
            Err(e) => {
                log::warn!("ignoring unreadable settings at {}: {e}", path.display());
                Self::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    pub fn sanitized(mut self) -> Self {
        self.streamlink_path = self.streamlink_path.trim().to_string();
        self.player_path = self.player_path.trim().to_string();
        self.extra_args = self.extra_args.trim().to_string();
        self.quality = match self.quality.trim() {
            "" => "best".into(),
            q => q.to_string(),
        };
        self.kick_event = self.kick_event.trim().to_ascii_lowercase();
        self.category = self.category.trim().to_string();
        self.poll_seconds = self.poll_seconds.clamp(MIN_POLL_SECONDS, MAX_POLL_SECONDS);

        let mut tags: Vec<String> = Vec::new();
        for tag in self
            .twitch_tags
            .iter()
            .map(|t| t.trim().trim_start_matches('#'))
        {
            if !tag.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                tags.push(tag.to_string());
            }
        }
        self.twitch_tags = tags;

        let mut favorites: Vec<Favorite> = Vec::new();
        for mut fav in self.favorites {
            fav.key = name_key(&fav.key);
            fav.twitch = fav
                .twitch
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            fav.kick = fav
                .kick
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            if !fav.key.is_empty() && !favorites.iter().any(|f| f.key == fav.key) {
                favorites.push(fav);
            }
        }
        self.favorites = favorites;

        self.links
            .retain(|l| !name_key(&l.twitch).is_empty() && !name_key(&l.kick).is_empty());

        let mut added: Vec<AddedChannel> = Vec::new();
        for mut channel in self.added {
            channel.login = channel
                .login
                .trim()
                .trim_start_matches('@')
                .to_ascii_lowercase();
            if is_valid_slug(&channel.login) && !added.contains(&channel) {
                added.push(channel);
            }
        }
        self.added = added;

        let mut hidden: Vec<Hidden> = Vec::new();
        for mut entry in self.hidden {
            entry.key = name_key(&entry.key);
            entry.name = entry.name.trim().to_string();
            if !entry.key.is_empty() && !hidden.iter().any(|h| h.key == entry.key) {
                hidden.push(entry);
            }
        }
        self.hidden = hidden;
        self
    }

    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorites.iter().any(|f| f.key == key)
    }

    pub fn is_added(&self, platform: Platform, login: &str) -> bool {
        let key = name_key(login);
        self.added
            .iter()
            .any(|a| a.platform == platform && name_key(&a.login) == key)
    }

    pub fn is_hidden(&self, streamer: &Streamer) -> bool {
        self.hidden.iter().any(|h| {
            h.key == streamer.key
                || streamer
                    .channels
                    .iter()
                    .any(|c| name_key(&c.login) == h.key)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "quality": "720p60" }"#).unwrap();
        assert_eq!(settings.quality, "720p60");
        assert_eq!(settings.player, Player::Vlc);
        assert_eq!(settings.chat, Chat::Window);
        assert_eq!(settings.kick_event, "nopixel-v");
        assert_eq!(settings.category, "Grand Theft Auto V");
        assert_eq!(settings.twitch_tags, ["NoPixelV", "NoPixel"]);
    }

    #[test]
    fn sanitizing_cleans_input() {
        let settings = Settings {
            quality: "  ".into(),
            poll_seconds: 5,
            twitch_tags: vec![
                "#NoPixelV".into(),
                "nopixelv".into(),
                " ".into(),
                "NP".into(),
            ],
            favorites: vec![
                Favorite {
                    key: "Ms_Star".into(),
                    name: "Ms Star".into(),
                    twitch: Some(" ".into()),
                    kick: None,
                },
                Favorite {
                    key: "msstar".into(),
                    name: "dupe".into(),
                    twitch: None,
                    kick: None,
                },
            ],
            links: vec![Link {
                twitch: "".into(),
                kick: "x".into(),
            }],
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(settings.quality, "best");
        assert_eq!(settings.poll_seconds, MIN_POLL_SECONDS);
        assert_eq!(settings.twitch_tags, ["NoPixelV", "NP"]);
        assert_eq!(settings.favorites.len(), 1);
        assert_eq!(settings.favorites[0].key, "msstar");
        assert_eq!(settings.favorites[0].twitch, None);
        assert!(settings.links.is_empty());
    }

    #[test]
    fn sanitizing_cleans_added_and_hidden() {
        let settings = Settings {
            added: vec![
                AddedChannel {
                    platform: Platform::Twitch,
                    login: " @Koil ".into(),
                },
                AddedChannel {
                    platform: Platform::Twitch,
                    login: "koil".into(),
                },
                AddedChannel {
                    platform: Platform::Kick,
                    login: "koil".into(),
                },
                AddedChannel {
                    platform: Platform::Kick,
                    login: "../api".into(),
                },
            ],
            hidden: vec![
                Hidden {
                    key: "Spam_Guy".into(),
                    name: " Spam_Guy ".into(),
                },
                Hidden {
                    key: "spamguy".into(),
                    name: "dupe".into(),
                },
                Hidden {
                    key: "__".into(),
                    name: "empty".into(),
                },
            ],
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(
            settings.added,
            [
                AddedChannel {
                    platform: Platform::Twitch,
                    login: "koil".into()
                },
                AddedChannel {
                    platform: Platform::Kick,
                    login: "koil".into()
                },
            ]
        );
        assert_eq!(
            settings.hidden,
            [Hidden {
                key: "spamguy".into(),
                name: "Spam_Guy".into()
            }]
        );
        assert!(settings.is_added(Platform::Kick, "Koil"));
        assert!(!settings.is_added(Platform::Twitch, "buddha"));
    }

    #[test]
    fn hidden_matches_streamer_key_or_any_login() {
        let settings = Settings {
            hidden: vec![Hidden {
                key: "spamguy".into(),
                name: "Spam Guy".into(),
            }],
            ..Settings::default()
        };
        let streamer = |key: &str, login: &str| Streamer {
            key: key.into(),
            display_name: key.into(),
            avatar: None,
            viewers: 1,
            channels: vec![crate::model::Channel {
                platform: Platform::Kick,
                login: login.into(),
                display_name: login.into(),
                title: "NoPixel".into(),
                viewers: 1,
                category: None,
                thumbnail: None,
                avatar: None,
                started_at: None,
            }],
        };
        assert!(settings.is_hidden(&streamer("spamguy", "spamguy")));
        assert!(settings.is_hidden(&streamer("linkedname", "spam_guy")));
        assert!(!settings.is_hidden(&streamer("buddha", "buddha")));
    }

    #[test]
    fn save_and_load_round_trip() {
        let path =
            std::env::temp_dir().join(format!("nopixel-live-test-{}.json", std::process::id()));
        let settings = Settings {
            quality: "720p60".into(),
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let path =
            std::env::temp_dir().join(format!("nopixel-live-bad-{}.json", std::process::id()));
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        fs::remove_file(&path).unwrap();
    }
}
