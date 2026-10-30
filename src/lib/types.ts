export type Platform = 'twitch' | 'kick'
export type Player = 'vlc' | 'mpv'
export type ChatMode = 'window' | 'browser' | 'off'

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
  player: Player
  playerPath: string
  chat: ChatMode
  quality: string
  extraArgs: string
  preferredPlatform: Platform
  pollSeconds: number
  twitchTags: string[]
  kickEvent: string
  category: string
  notifyFavorites: boolean
  closeToTray: boolean
  setupDone: boolean
  favorites: Favorite[]
  links: Link[]
  added: AddedChannel[]
  hidden: Hidden[]
}

export interface Tools {
  streamlink: string | null
  vlc: string | null
  mpv: string | null
}

export interface InstallProgress {
  stage: 'download' | 'unpack'
  received: number
  total: number
}

export interface PlayerError {
  name: string
  platform: Platform
  message: string
}
