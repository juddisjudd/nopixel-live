import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Platform, PlayerError, Settings, Snapshot, Tools } from './types'

export const api = {
  snapshot: () => invoke<Snapshot>('get_snapshot'),
  settings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  toggleFavorite: (key: string) => invoke<Settings>('toggle_favorite', { key }),
  hide: (key: string) => invoke<Settings>('hide_streamer', { key }),
  unhide: (key: string) => invoke<Settings>('unhide_streamer', { key }),
  refresh: () => invoke<void>('refresh_now'),
  play: (key: string, platform: Platform) => invoke<void>('play', { key, platform }),
  openPage: (key: string, platform: Platform) => invoke<void>('open_page', { key, platform }),
  detectTools: () => invoke<Tools>('detect_tools'),
}

export const onSnapshot = (handler: (snapshot: Snapshot) => void) =>
  listen<Snapshot>('snapshot', (event) => handler(event.payload))

export const onPlayerError = (handler: (error: PlayerError) => void) =>
  listen<PlayerError>('player-error', (event) => handler(event.payload))
