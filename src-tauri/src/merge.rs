use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use crate::model::{Channel, Platform, Streamer, name_key};
use crate::settings::Link;

pub fn merge(channels: Vec<Channel>, links: &[Link]) -> Vec<Streamer> {
    let kick_to_twitch: HashMap<String, String> = links
        .iter()
        .map(|l| (name_key(&l.kick), name_key(&l.twitch)))
        .collect();

    let mut seen = HashSet::new();
    let mut groups: Vec<(String, Vec<Channel>)> = Vec::new();
    for channel in channels {
        let login_key = name_key(&channel.login);
        if !seen.insert((channel.platform, login_key.clone())) {
            continue;
        }
        let key = match channel.platform {
            Platform::Kick => kick_to_twitch.get(&login_key).cloned().unwrap_or(login_key),
            Platform::Twitch => login_key,
        };
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, group)) => group.push(channel),
            None => groups.push((key, vec![channel])),
        }
    }

    let mut streamers: Vec<Streamer> = groups
        .into_iter()
        .map(|(key, mut channels)| {
            channels.sort_by_key(|c| Reverse(c.viewers));
            let lead = &channels[0];
            Streamer {
                display_name: lead.display_name.clone(),
                avatar: channels.iter().find_map(|c| c.avatar.clone()),
                viewers: channels.iter().map(|c| c.viewers).sum(),
                key,
                channels,
            }
        })
        .collect();
    streamers.sort_by(|a, b| b.viewers.cmp(&a.viewers).then_with(|| a.key.cmp(&b.key)));
    streamers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(platform: Platform, login: &str, viewers: u64) -> Channel {
        Channel {
            platform,
            login: login.into(),
            display_name: login.to_uppercase(),
            title: "NoPixel V".into(),
            viewers,
            category: None,
            thumbnail: None,
            avatar: None,
            started_at: None,
        }
    }

    #[test]
    fn same_name_on_both_sites_becomes_one_streamer() {
        let streamers = merge(
            vec![
                channel(Platform::Twitch, "buddha", 900),
                channel(Platform::Kick, "buddha", 8000),
                channel(Platform::Twitch, "uhn", 1700),
            ],
            &[],
        );
        assert_eq!(streamers.len(), 2);
        let buddha = &streamers[0];
        assert_eq!(buddha.key, "buddha");
        assert_eq!(buddha.viewers, 8900);
        assert_eq!(buddha.channels[0].platform, Platform::Kick);
        assert!(buddha.channel(Platform::Twitch).is_some());
    }

    #[test]
    fn separators_do_not_split_a_streamer() {
        let streamers = merge(
            vec![
                channel(Platform::Twitch, "ms_star", 10),
                channel(Platform::Kick, "ms-star", 20),
            ],
            &[],
        );
        assert_eq!(streamers.len(), 1);
    }

    #[test]
    fn links_join_different_names() {
        let links = [Link {
            twitch: "koil".into(),
            kick: "koilkick".into(),
        }];
        let streamers = merge(
            vec![
                channel(Platform::Twitch, "koil", 10),
                channel(Platform::Kick, "KoilKick", 20),
            ],
            &links,
        );
        assert_eq!(streamers.len(), 1);
        assert_eq!(streamers[0].key, "koil");
    }

    #[test]
    fn duplicate_channels_are_dropped() {
        let streamers = merge(
            vec![
                channel(Platform::Twitch, "uhn", 10),
                channel(Platform::Twitch, "UHN", 10),
            ],
            &[],
        );
        assert_eq!(streamers[0].channels.len(), 1);
        assert_eq!(streamers[0].viewers, 10);
    }

    #[test]
    fn sorted_by_total_viewers() {
        let streamers = merge(
            vec![
                channel(Platform::Twitch, "small", 5),
                channel(Platform::Kick, "big", 500),
            ],
            &[],
        );
        assert_eq!(streamers[0].key, "big");
    }
}
