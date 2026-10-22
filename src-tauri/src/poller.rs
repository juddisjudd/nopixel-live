use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{PoisonError, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::future::join_all;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

use crate::model::{
    Channel, Platform, Snapshot, Streamer, in_category, is_nopixel_title, name_key,
};
use crate::settings::Settings;
use crate::twitch::TwitchResult;
use crate::{kick, merge, tray, twitch};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
const MAX_KICK_LOOKUPS: usize = 50;

pub struct AppState {
    pub settings_path: PathBuf,
    pub client: reqwest::Client,
    pub refresh: Notify,
    settings: RwLock<Settings>,
    snapshot: RwLock<Snapshot>,
}

impl AppState {
    pub fn new(settings_path: PathBuf) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .build()?;
        Ok(Self {
            settings: RwLock::new(Settings::load(&settings_path)),
            settings_path,
            client,
            refresh: Notify::new(),
            snapshot: RwLock::new(Snapshot::default()),
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set_settings(&self, settings: Settings) {
        *self
            .settings
            .write()
            .unwrap_or_else(PoisonError::into_inner) = settings;
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set_snapshot(&self, snapshot: Snapshot) {
        *self
            .snapshot
            .write()
            .unwrap_or_else(PoisonError::into_inner) = snapshot;
    }

    pub fn drop_hidden(&self, settings: &Settings) -> Snapshot {
        let mut snapshot = self
            .snapshot
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        snapshot.streamers.retain(|s| !settings.is_hidden(s));
        snapshot.clone()
    }
}

pub async fn run(app: AppHandle) {
    let mut tracker = FavoriteTracker::default();
    loop {
        let state = app.state::<AppState>();
        let settings = state.settings();
        let snapshot = collect(&state.client, &settings).await;

        let newly_live = tracker.update(&snapshot, &settings);
        if settings.notify_favorites {
            for streamer in &newly_live {
                notify_live(&app, streamer);
            }
        }
        tray::set_tooltip(&app, &tooltip(&snapshot, &settings));
        state.set_snapshot(snapshot.clone());
        if let Err(e) = app.emit("snapshot", &snapshot) {
            log::warn!("cannot send snapshot to the window: {e}");
        }

        let wait = Duration::from_secs(settings.poll_seconds);
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = state.refresh.notified() => {}
        }
    }
}

pub async fn collect(client: &reqwest::Client, settings: &Settings) -> Snapshot {
    let mut errors = Vec::new();

    let kick_live = kick::fetch_event(client, &settings.kick_event)
        .await
        .unwrap_or_else(|e| {
            errors.push(format!("Kick: {e}"));
            Vec::new()
        });

    let mut lookups: Vec<String> = kick_live.iter().map(|c| c.login.clone()).collect();
    lookups.extend(
        settings
            .favorites
            .iter()
            .filter_map(|f| f.twitch.clone().or_else(|| f.kick.clone())),
    );
    lookups.extend(added_logins(settings, Platform::Twitch).map(String::from));
    let mut seen = HashSet::new();
    lookups.retain(|login| seen.insert(name_key(login)));

    let twitch = twitch::fetch(client, &settings.twitch_tags, &lookups)
        .await
        .unwrap_or_else(|e| {
            errors.push(format!("Twitch: {e}"));
            TwitchResult::default()
        });

    let mut seen: HashSet<String> = kick_live.iter().map(|c| name_key(&c.login)).collect();
    let kick_lookups: Vec<&str> = added_logins(settings, Platform::Kick)
        .chain(settings.favorites.iter().filter_map(|f| f.kick.as_deref()))
        .filter(|slug| seen.insert(name_key(slug)))
        .take(MAX_KICK_LOOKUPS)
        .collect();
    let results = join_all(
        kick_lookups
            .iter()
            .map(|slug| kick::fetch_channel(client, slug)),
    )
    .await;
    let mut kick_extra = Vec::new();
    for (slug, result) in kick_lookups.iter().zip(results) {
        match result {
            Ok(Some(channel)) => kick_extra.push(channel),
            Ok(None) => {}
            Err(e) => log::warn!("Kick lookup for {slug} failed: {e}"),
        }
    }

    Snapshot {
        streamers: assemble(settings, twitch, kick_live, kick_extra),
        updated_at: Some(now_millis()),
        errors,
    }
}

/// Looked-up channels need NoPixel in the title unless the user added them by hand.
pub fn assemble(
    settings: &Settings,
    twitch: TwitchResult,
    kick_live: Vec<Channel>,
    kick_extra: Vec<Channel>,
) -> Vec<Streamer> {
    let wanted =
        |c: &Channel| is_nopixel_title(&c.title) || settings.is_added(c.platform, &c.login);
    let mut channels = twitch.tagged;
    channels.extend(twitch.looked_up.into_iter().filter(|c| wanted(c)));
    channels.extend(kick_live);
    channels.extend(kick_extra.into_iter().filter(|c| wanted(c)));
    channels.retain(|c| in_category(c, &settings.category));

    let mut streamers = merge::merge(channels, &settings.links);
    streamers.retain(|s| !settings.is_hidden(s));
    streamers
}

fn added_logins(settings: &Settings, platform: Platform) -> impl Iterator<Item = &str> {
    settings
        .added
        .iter()
        .filter(move |a| a.platform == platform)
        .map(|a| a.login.as_str())
}

/// Remembers which favorites were live so a failed poll does not cause repeat notifications.
#[derive(Default)]
pub struct FavoriteTracker {
    known: HashSet<String>,
    primed: bool,
}

impl FavoriteTracker {
    pub fn update(&mut self, snapshot: &Snapshot, settings: &Settings) -> Vec<Streamer> {
        let live: HashSet<String> = snapshot
            .streamers
            .iter()
            .filter(|s| settings.is_favorite(&s.key))
            .map(|s| s.key.clone())
            .collect();
        let newly_live = if self.primed {
            snapshot
                .streamers
                .iter()
                .filter(|s| live.contains(&s.key) && !self.known.contains(&s.key))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        if snapshot.errors.is_empty() {
            self.known = live;
        } else {
            self.known.extend(live);
        }
        self.primed = true;
        newly_live
    }
}

fn notify_live(app: &AppHandle, streamer: &Streamer) {
    let sites = streamer
        .channels
        .iter()
        .map(|c| c.platform.label())
        .collect::<Vec<_>>()
        .join(" + ");
    let title = format!("{} is live on {sites}", streamer.display_name);
    let body = streamer
        .channels
        .first()
        .map(|c| c.title.clone())
        .unwrap_or_default();
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("cannot show notification: {e}");
    }
}

fn tooltip(snapshot: &Snapshot, settings: &Settings) -> String {
    let live = snapshot.streamers.len();
    let favorites = snapshot
        .streamers
        .iter()
        .filter(|s| settings.is_favorite(&s.key))
        .count();
    match favorites {
        0 => format!("NoPixel Live: {live} live"),
        n => format!(
            "NoPixel Live: {live} live, {n} favorite{}",
            if n == 1 { "" } else { "s" }
        ),
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{AddedChannel, Favorite, Hidden};

    fn streamer(key: &str) -> Streamer {
        Streamer {
            key: key.into(),
            display_name: key.into(),
            avatar: None,
            viewers: 1,
            channels: vec![Channel {
                platform: Platform::Twitch,
                login: key.into(),
                display_name: key.into(),
                title: "NoPixel V".into(),
                viewers: 1,
                category: None,
                thumbnail: None,
                avatar: None,
                started_at: None,
            }],
        }
    }

    fn snapshot(keys: &[&str], failed: bool) -> Snapshot {
        Snapshot {
            streamers: keys.iter().map(|k| streamer(k)).collect(),
            updated_at: Some(1),
            errors: if failed {
                vec!["Twitch: HTTP 500".into()]
            } else {
                Vec::new()
            },
        }
    }

    fn settings_with(favorites: &[&str]) -> Settings {
        Settings {
            favorites: favorites
                .iter()
                .map(|k| Favorite {
                    key: (*k).into(),
                    name: (*k).into(),
                    twitch: None,
                    kick: None,
                })
                .collect(),
            ..Settings::default()
        }
    }

    fn keys(streamers: Vec<Streamer>) -> Vec<String> {
        streamers.into_iter().map(|s| s.key).collect()
    }

    #[test]
    fn first_poll_does_not_notify() {
        let mut tracker = FavoriteTracker::default();
        let settings = settings_with(&["buddha"]);
        assert!(
            tracker
                .update(&snapshot(&["buddha"], false), &settings)
                .is_empty()
        );
    }

    #[test]
    fn notifies_when_a_favorite_goes_live() {
        let mut tracker = FavoriteTracker::default();
        let settings = settings_with(&["buddha"]);
        tracker.update(&snapshot(&["uhn"], false), &settings);
        assert_eq!(
            keys(tracker.update(&snapshot(&["uhn", "buddha"], false), &settings)),
            ["buddha"]
        );
        assert!(
            tracker
                .update(&snapshot(&["uhn", "buddha"], false), &settings)
                .is_empty()
        );
    }

    #[test]
    fn failed_poll_does_not_repeat_notification() {
        let mut tracker = FavoriteTracker::default();
        let settings = settings_with(&["buddha"]);
        tracker.update(&snapshot(&["buddha"], false), &settings);
        tracker.update(&snapshot(&[], true), &settings);
        assert!(
            tracker
                .update(&snapshot(&["buddha"], false), &settings)
                .is_empty()
        );
    }

    #[test]
    fn notifies_again_after_going_offline() {
        let mut tracker = FavoriteTracker::default();
        let settings = settings_with(&["buddha"]);
        tracker.update(&snapshot(&["buddha"], false), &settings);
        tracker.update(&snapshot(&[], false), &settings);
        assert_eq!(
            keys(tracker.update(&snapshot(&["buddha"], false), &settings)),
            ["buddha"]
        );
    }

    fn live_channel(
        platform: Platform,
        login: &str,
        title: &str,
        category: Option<&str>,
    ) -> Channel {
        Channel {
            platform,
            login: login.into(),
            display_name: login.into(),
            title: title.into(),
            viewers: 10,
            category: category.map(String::from),
            thumbnail: None,
            avatar: None,
            started_at: None,
        }
    }

    fn assembled_keys(
        settings: &Settings,
        twitch: TwitchResult,
        kick_extra: Vec<Channel>,
    ) -> Vec<String> {
        let mut keys = keys(assemble(settings, twitch, Vec::new(), kick_extra));
        keys.sort();
        keys
    }

    #[test]
    fn added_channels_skip_the_title_check() {
        let settings = Settings {
            added: vec![
                AddedChannel {
                    platform: Platform::Twitch,
                    login: "koil".into(),
                },
                AddedChannel {
                    platform: Platform::Kick,
                    login: "ramee".into(),
                },
            ],
            ..Settings::default()
        };
        let gta = Some("Grand Theft Auto V");
        let twitch = TwitchResult {
            tagged: Vec::new(),
            looked_up: vec![
                live_channel(Platform::Twitch, "koil", "dev stream", gta),
                live_channel(Platform::Twitch, "random", "dev stream", gta),
            ],
        };
        let kick_extra = vec![live_channel(
            Platform::Kick,
            "ramee",
            "chill day",
            Some("Grand Theft Auto V (GTA)"),
        )];
        assert_eq!(
            assembled_keys(&settings, twitch, kick_extra),
            ["koil", "ramee"]
        );
    }

    #[test]
    fn added_channels_still_need_the_category() {
        let settings = Settings {
            added: vec![AddedChannel {
                platform: Platform::Twitch,
                login: "koil".into(),
            }],
            ..Settings::default()
        };
        let twitch = TwitchResult {
            tagged: Vec::new(),
            looked_up: vec![live_channel(
                Platform::Twitch,
                "koil",
                "dev stream",
                Some("Just Chatting"),
            )],
        };
        assert!(assembled_keys(&settings, twitch, Vec::new()).is_empty());
    }

    #[test]
    fn hidden_streamers_are_removed() {
        let settings = Settings {
            hidden: vec![Hidden {
                key: "spamguy".into(),
                name: "SpamGuy".into(),
            }],
            ..Settings::default()
        };
        let gta = Some("Grand Theft Auto V");
        let twitch = TwitchResult {
            tagged: vec![
                live_channel(Platform::Twitch, "SpamGuy", "NOPIXEL NOPIXEL NOPIXEL", gta),
                live_channel(Platform::Twitch, "uhn", "NoPixel V", gta),
            ],
            looked_up: Vec::new(),
        };
        assert_eq!(assembled_keys(&settings, twitch, Vec::new()), ["uhn"]);
    }

    #[tokio::test]
    #[ignore = "calls the live Twitch and Kick endpoints"]
    async fn live_endpoints_return_streamers() {
        let state = AppState::new(std::env::temp_dir().join("nopixel-live-none.json")).unwrap();
        let snapshot = collect(&state.client, &Settings::default()).await;
        let count = |p: Platform| {
            snapshot
                .streamers
                .iter()
                .filter(|s| s.channel(p).is_some())
                .count()
        };
        println!(
            "{} streamers, {} on Twitch, {} on Kick, {} on both, errors: {:?}",
            snapshot.streamers.len(),
            count(Platform::Twitch),
            count(Platform::Kick),
            snapshot
                .streamers
                .iter()
                .filter(|s| s.channels.len() > 1)
                .count(),
            snapshot.errors
        );
        for s in snapshot.streamers.iter().take(8) {
            let sites: Vec<_> = s
                .channels
                .iter()
                .map(|c| format!("{}:{}", c.platform.label(), c.viewers))
                .collect();
            println!("  {} {:?}", s.display_name, sites);
        }
        assert!(snapshot.errors.is_empty());
        assert!(!snapshot.streamers.is_empty());
        let channels = || snapshot.streamers.iter().flat_map(|s| &s.channels);
        assert!(channels().all(|c| in_category(c, "Grand Theft Auto V")));
    }

    #[test]
    fn tooltip_counts_favorites() {
        let settings = settings_with(&["buddha"]);
        assert_eq!(
            tooltip(&snapshot(&["buddha", "uhn"], false), &settings),
            "NoPixel Live: 2 live, 1 favorite"
        );
        assert_eq!(
            tooltip(&snapshot(&["uhn"], false), &settings),
            "NoPixel Live: 1 live"
        );
    }
}
