import type { Channel, ChatMode, Platform, Player, Streamer } from './types'

export const PLATFORM_LABEL: Record<Platform, string> = { twitch: 'Twitch', kick: 'Kick' }

export const PLAYER_LABEL: Record<Player, string> = { vlc: 'VLC', mpv: 'mpv' }

export const PLATFORM_OPTIONS: { value: Platform; label: string; tone: string }[] = [
  { value: 'twitch', label: PLATFORM_LABEL.twitch, tone: 'twitch' },
  { value: 'kick', label: PLATFORM_LABEL.kick, tone: 'kick' },
]

export const PLAYER_OPTIONS: { value: Player; label: string }[] = [
  { value: 'vlc', label: PLAYER_LABEL.vlc },
  { value: 'mpv', label: PLAYER_LABEL.mpv },
]

export const CHAT_MODES: { value: ChatMode; label: string }[] = [
  { value: 'window', label: 'In a window' },
  { value: 'browser', label: 'In my browser' },
  { value: 'off', label: 'Off' },
]

export type SiteFilter = 'all' | 'twitch' | 'kick' | 'both'

const compactFormat = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 })

export const megabytes = (bytes: number) => Math.round(bytes / 1_000_000)

export function compact(n: number): string {
  return compactFormat.format(n)
}

export function uptime(startedAt: string | null, now: number): string | null {
  if (!startedAt) return null
  const started = Date.parse(startedAt)
  if (Number.isNaN(started)) return null
  const minutes = Math.max(0, Math.floor((now - started) / 60_000))
  const hours = Math.floor(minutes / 60)
  return hours > 0 ? `${hours}h ${minutes % 60}m` : `${minutes}m`
}

export function ago(time: number | null, now: number): string {
  if (time === null) return 'never'
  const seconds = Math.max(0, Math.round((now - time) / 1000))
  if (seconds < 10) return 'just now'
  if (seconds < 60) return `${seconds}s ago`
  return `${Math.floor(seconds / 60)}m ago`
}

export function orderedChannels(streamer: Streamer, preferred: Platform): Channel[] {
  return [...streamer.channels].sort(
    (a, b) => Number(b.platform === preferred) - Number(a.platform === preferred),
  )
}

// Twitch reuses one preview URL per channel, so a coarse cache key keeps previews fresh without refetching every poll.
export function thumbnailUrl(channel: Channel, updatedAt: number | null): string | null {
  if (!channel.thumbnail) return null
  if (channel.platform !== 'twitch' || updatedAt === null) return channel.thumbnail
  return `${channel.thumbnail}?t=${Math.floor(updatedAt / 300_000)}`
}

export const DEFAULT_CATEGORY = 'Grand Theft Auto V'

// Mirrors same_category in model.rs: Kick appends a parenthetical such as "(GTA)".
export function sameCategory(a: string, b: string): boolean {
  const base = (name: string) => name.replace(/\s\([^)]*\)\s*$/, '').trim().toLowerCase()
  return base(a) === base(b)
}

export function matchesSite(streamer: Streamer, filter: SiteFilter): boolean {
  switch (filter) {
    case 'all':
      return true
    case 'both':
      return streamer.channels.length > 1
    default:
      return streamer.channels.some((c) => c.platform === filter)
  }
}

export function matchesQuery(streamer: Streamer, query: string): boolean {
  const needle = query.trim().toLowerCase()
  if (!needle) return true
  return streamer.channels.some((c) =>
    [c.displayName, c.login, c.title, c.category ?? ''].some((text) => text.toLowerCase().includes(needle)),
  )
}
