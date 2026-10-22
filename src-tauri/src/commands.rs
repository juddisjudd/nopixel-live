use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::model::{Channel, Platform, Snapshot};
use crate::player;
use crate::poller::AppState;
use crate::settings::{Favorite, Hidden, Settings};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    streamlink: Option<String>,
    player: Option<String>,
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snapshot()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings, String> {
    let settings = persist(&state, settings)?;
    state.refresh.notify_one();
    Ok(settings)
}

#[tauri::command]
pub fn toggle_favorite(state: State<'_, AppState>, key: String) -> Result<Settings, String> {
    let mut settings = state.settings();
    if settings.is_favorite(&key) {
        settings.favorites.retain(|f| f.key != key);
    } else {
        let snapshot = state.snapshot();
        let streamer = snapshot
            .streamer(&key)
            .ok_or("That streamer is no longer live.")?;
        settings.favorites.push(Favorite {
            key: streamer.key.clone(),
            name: streamer.display_name.clone(),
            twitch: streamer.channel(Platform::Twitch).map(|c| c.login.clone()),
            kick: streamer.channel(Platform::Kick).map(|c| c.login.clone()),
        });
    }
    persist(&state, settings)
}

#[tauri::command]
pub fn hide_streamer(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
) -> Result<Settings, String> {
    let mut settings = state.settings();
    let name = state
        .snapshot()
        .streamer(&key)
        .map_or_else(|| key.clone(), |s| s.display_name.clone());
    settings.hidden.push(Hidden { key, name });
    let settings = persist(&state, settings)?;
    if let Err(e) = app.emit("snapshot", state.drop_hidden(&settings)) {
        log::warn!("cannot send snapshot to the window: {e}");
    }
    Ok(settings)
}

#[tauri::command]
pub fn unhide_streamer(state: State<'_, AppState>, key: String) -> Result<Settings, String> {
    let mut settings = state.settings();
    settings.hidden.retain(|h| h.key != key);
    let settings = persist(&state, settings)?;
    state.refresh.notify_one();
    Ok(settings)
}

#[tauri::command]
pub fn refresh_now(state: State<'_, AppState>) {
    state.refresh.notify_one();
}

#[tauri::command]
pub fn play(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    platform: Platform,
) -> Result<(), String> {
    let channel = live_channel(&state, &key, platform)?;
    player::launch(app, &state.settings(), &channel)
}

#[tauri::command]
pub fn open_page(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    platform: Platform,
) -> Result<(), String> {
    let channel = live_channel(&state, &key, platform)?;
    app.opener()
        .open_url(platform.channel_url(&channel.login), None::<&str>)
        .map_err(|e| format!("Cannot open the browser: {e}"))
}

#[tauri::command]
pub fn detect_tools(state: State<'_, AppState>) -> Tools {
    let settings = state.settings();
    Tools {
        streamlink: player::find_streamlink(&settings.streamlink_path)
            .map(|p| p.display().to_string()),
        player: player::find_player(&settings.player_path).map(|p| p.display().to_string()),
    }
}

fn persist(state: &AppState, settings: Settings) -> Result<Settings, String> {
    let settings = settings.sanitized();
    settings.save(&state.settings_path)?;
    state.set_settings(settings.clone());
    Ok(settings)
}

fn live_channel(state: &AppState, key: &str, platform: Platform) -> Result<Channel, String> {
    state
        .snapshot()
        .streamer(key)
        .and_then(|s| s.channel(platform))
        .cloned()
        .ok_or_else(|| format!("That stream is no longer live on {}.", platform.label()))
}
