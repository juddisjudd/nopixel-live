import { getVersion } from '@tauri-apps/api/app'
import { check, type Update } from '@tauri-apps/plugin-updater'

const FIRST_CHECK_MS = 5_000
const CHECK_EVERY_MS = 6 * 60 * 60_000

export type UpdateState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'current' }
  | { kind: 'available'; update: Update }
  | { kind: 'downloading'; update: Update; received: number; total: number }
  | { kind: 'failed'; message: string; update: Update | null }

export class Updater {
  version = $state('')
  state = $state.raw<UpdateState>({ kind: 'idle' })

  start(): () => void {
    getVersion().then((version) => (this.version = version))
    const first = setTimeout(() => this.check(true), FIRST_CHECK_MS)
    const every = setInterval(() => this.check(true), CHECK_EVERY_MS)
    return () => {
      clearTimeout(first)
      clearInterval(every)
    }
  }

  async check(quiet = false) {
    const busy = this.state.kind === 'checking' || this.state.kind === 'downloading'
    if (busy || (quiet && this.state.kind === 'available')) return
    if (!quiet) this.state = { kind: 'checking' }
    try {
      const update = await check()
      this.state = update ? { kind: 'available', update } : { kind: 'current' }
    } catch (e) {
      if (!quiet) this.state = { kind: 'failed', message: String(e), update: null }
    }
  }

  // On Windows the app exits as soon as the installer starts, and the installer opens it again.
  async install() {
    const current = this.state
    const update = current.kind === 'available' || current.kind === 'failed' ? current.update : null
    if (!update) return
    let received = 0
    let total = 0
    this.state = { kind: 'downloading', update, received, total }
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0
        if (event.event === 'Progress') received += event.data.chunkLength
        this.state = { kind: 'downloading', update, received, total }
      })
    } catch (e) {
      this.state = { kind: 'failed', message: String(e), update }
    }
  }
}
