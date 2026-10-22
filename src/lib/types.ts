export type Platform = 'twitch' | 'kick'

export interface Channel {
  platform: Platform
  login: string
  displayName: string
  title: string
  viewers: number
  category: string | null
  thumbnail: string | null
  avatar: string | null
  startedAt: string | null
}

export interface Streamer {
  key: string
  displayName: string
  avatar: string | null
  viewers: number
  channels: Channel[]
}

export interface Snapshot {
  streamers: Streamer[]
  updatedAt: number | null
  errors: string[]
}

export interface Favorite {
  key: string
  name: string
  twitch: string | null
  kick: string | null
}

export interface AddedChannel {
  platform: Platform
  login: string
}

export interface Hidden {
  key: string
  name: string
}

export interface Link {
  twitch: string
  kick: string
}

export interface Settings {
  streamlinkPath: string
  playerPath: string
  quality: string
  extraArgs: string
  preferredPlatform: Platform
  pollSeconds: number
  twitchTags: string[]
  kickEvent: string
  category: string
  notifyFavorites: boolean
  closeToTray: boolean
  favorites: Favorite[]
  links: Link[]
  added: AddedChannel[]
  hidden: Hidden[]
}

export interface Tools {
  streamlink: string | null
  player: string | null
}

export interface PlayerError {
  name: string
  platform: Platform
  message: string
}
