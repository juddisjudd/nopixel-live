<script lang="ts">
  import { tick } from 'svelte'
  import Icon from './Icon.svelte'
  import Segmented from './Segmented.svelte'
  import StreamlinkInstall, { type InstallState } from './StreamlinkInstall.svelte'
  import { api } from './api'
  import { CHAT_MODES, PLATFORM_LABEL, PLATFORM_OPTIONS, PLAYER_LABEL, PLAYER_OPTIONS } from './format'
  import type { AddedChannel, ChatMode, Favorite, Hidden, Link, Platform, Player, Settings, Tools } from './types'
  import type { Updater } from './updater.svelte'

  interface Props {
    settings: Settings
    tools: Tools | null
    install: InstallState
    updater: Updater
    oninstall: () => void
    onsave: (settings: Settings) => Promise<void>
    onsetup: () => void
    onclose: () => void
  }

  let { settings, tools, install, updater, oninstall, onsave, onsetup, onclose }: Props = $props()

  type Tab = 'playback' | 'sources' | 'streamers' | 'app'
  const TABS: { id: Tab; label: string }[] = [
    { id: 'playback', label: 'Playback' },
    { id: 'sources', label: 'Finding streams' },
    { id: 'streamers', label: 'Streamers' },
    { id: 'app', label: 'App' },
  ]

  interface Form {
    streamlinkPath: string
    player: Player
    playerPath: string
    chat: ChatMode
    quality: string
    extraArgs: string
    preferredPlatform: Platform
    pollSeconds: number
    tagsText: string
    kickEvent: string
    category: string
    notifyFavorites: boolean
    closeToTray: boolean
    setupDone: boolean
    favorites: Favorite[]
    links: Link[]
    added: AddedChannel[]
    hidden: Hidden[]
    newFavorite: string
    newChannels: string
    newSites: Platform | 'both'
  }

  function toForm(s: Settings): Form {
    return {
      streamlinkPath: s.streamlinkPath,
      player: s.player,
      playerPath: s.playerPath,
      chat: s.chat,
      quality: s.quality,
      extraArgs: s.extraArgs,
      preferredPlatform: s.preferredPlatform,
      pollSeconds: s.pollSeconds,
      tagsText: s.twitchTags.join(', '),
      kickEvent: s.kickEvent,
      category: s.category,
      notifyFavorites: s.notifyFavorites,
      closeToTray: s.closeToTray,
      setupDone: s.setupDone,
      favorites: s.favorites.map((f) => ({ ...f })),
      links: s.links.map((l) => ({ ...l })),
      added: s.added.map((a) => ({ ...a })),
      hidden: s.hidden.map((h) => ({ ...h })),
      newFavorite: '',
      newChannels: '',
      newSites: 'both',
    }
  }

  function fromForm(f: Form): Settings {
    return {
      streamlinkPath: f.streamlinkPath,
      player: f.player,
      playerPath: f.playerPath,
      chat: f.chat,
      quality: f.quality,
      extraArgs: f.extraArgs,
      preferredPlatform: f.preferredPlatform,
      pollSeconds: f.pollSeconds,
      twitchTags: f.tagsText.split(/[\s,]+/).filter(Boolean),
      kickEvent: f.kickEvent,
      category: f.category,
      notifyFavorites: f.notifyFavorites,
      closeToTray: f.closeToTray,
      setupDone: f.setupDone,
      favorites: f.favorites,
      links: f.links.filter((l) => l.twitch.trim() && l.kick.trim()),
      added: f.added,
      hidden: f.hidden,
    }
  }

  let form = $derived.by(() => {
    const draft = $state(toForm(settings))
    return draft
  })
  let tab = $state<Tab>('playback')
  let saving = $state(false)
  let error = $state<string | null>(null)
  let dialog = $state<HTMLDialogElement>()
  let formElement = $state<HTMLFormElement>()

  let playerFound = $derived(tools?.[form.player] ?? null)
  let readyUpdate = $derived(
    updater.state.kind === 'available' || updater.state.kind === 'failed' ? updater.state.update : null,
  )
  let updateStatus = $derived.by(() => {
    const state = updater.state
    switch (state.kind) {
      case 'checking':
        return 'Checking…'
      case 'current':
        return 'You have the newest version.'
      case 'available':
        return `Version ${state.update.version} is ready.`
      case 'downloading':
        return 'Downloading the update…'
      case 'failed':
        return `${state.update ? 'The update did not install.' : 'Could not check for updates.'} ${state.message}`
      default:
        return ''
    }
  })
  let addedGroups = $derived(
    [...new Set(form.added.map((a) => a.login))].map((login) => ({
      login,
      platforms: form.added.filter((a) => a.login === login).map((a) => a.platform),
    })),
  )

  const nameKey = (name: string) => name.toLowerCase().replace(/[^a-z0-9]/g, '')
  const tabId = (id: Tab) => `settings-tab-${id}`
  const panelId = (id: Tab) => `settings-panel-${id}`

  function onTabKey(event: KeyboardEvent) {
    const current = TABS.findIndex((t) => t.id === tab)
    const steps: Record<string, number> = {
      ArrowDown: current + 1,
      ArrowRight: current + 1,
      ArrowUp: current - 1,
      ArrowLeft: current - 1,
      Home: 0,
      End: TABS.length - 1,
    }
    const next = steps[event.key]
    if (next === undefined) return
    event.preventDefault()
    tab = TABS[(next + TABS.length) % TABS.length].id
    document.getElementById(tabId(tab))?.focus()
  }

  function addChannels() {
    for (const token of form.newChannels.split(/[\s,]+/).filter(Boolean)) {
      const link = token.match(/(twitch\.tv|kick\.com)\/([\w-]+)/i)
      const login = (link ? link[2] : token.replace(/^@/, '')).toLowerCase()
      if (!/^[a-z0-9_-]{1,64}$/.test(login)) continue
      const platforms: Platform[] = link
        ? [link[1].toLowerCase() === 'twitch.tv' ? 'twitch' : 'kick']
        : form.newSites === 'both'
          ? ['twitch', 'kick']
          : [form.newSites]
      for (const platform of platforms) {
        if (!form.added.some((a) => a.platform === platform && a.login === login)) {
          form.added.push({ platform, login })
        }
      }
      form.hidden = form.hidden.filter((h) => h.key !== nameKey(login))
    }
    form.newChannels = ''
  }

  function addFavorite() {
    const name = form.newFavorite.trim()
    if (!name) return
    const key = nameKey(name)
    if (key && !form.favorites.some((f) => f.key === key)) {
      form.favorites.push({ key, name, twitch: name, kick: name })
    }
    form.newFavorite = ''
  }

  async function save(event: SubmitEvent) {
    event.preventDefault()
    // Fields on hidden tabs cannot show the browser's own validation message, so open their tab first.
    const invalid = formElement?.querySelector<HTMLInputElement>('input:invalid')
    if (invalid) {
      tab = TABS.find((t) => invalid.closest(`#${panelId(t.id)}`))?.id ?? tab
      await tick()
      invalid.reportValidity()
      return
    }
    saving = true
    error = null
    try {
      await onsave(fromForm(form))
      dialog?.close()
    } catch (e) {
      error = String(e)
    } finally {
      saving = false
    }
  }
</script>

<dialog
  bind:this={dialog}
  {@attach (node: HTMLDialogElement) => node.showModal()}
  onclose={onclose}
  onclick={(event) => event.target === dialog && dialog?.close()}
  aria-labelledby="settings-title"
>
  <form bind:this={formElement} onsubmit={save} novalidate>
    <header>
      <h2 id="settings-title">Settings</h2>
      <button type="button" class="icon" onclick={() => dialog?.close()} aria-label="Close settings">
        <Icon name="close" />
      </button>
    </header>

    <div class="layout">
      <div class="tabs" role="tablist" aria-label="Settings sections" aria-orientation="vertical">
        {#each TABS as item (item.id)}
          <button
            type="button"
            role="tab"
            id={tabId(item.id)}
            aria-selected={tab === item.id}
            aria-controls={panelId(item.id)}
            tabindex={tab === item.id ? 0 : -1}
            onclick={() => (tab = item.id)}
            onkeydown={onTabKey}
          >
            {item.label}
          </button>
        {/each}
      </div>

      <div class="panel" role="tabpanel" id={panelId('playback')} aria-labelledby={tabId('playback')} hidden={tab !== 'playback'}>
        <fieldset>
          <legend>Apps</legend>
          <div class="field" role="radiogroup" aria-labelledby="player-label">
            <span id="player-label">Player</span>
            <Segmented name="player" options={PLAYER_OPTIONS} bind:value={form.player} onchange={() => (form.playerPath = '')} />
            {#if form.playerPath.trim()}
              <p class="note">Uses the location under Custom locations.</p>
            {:else if playerFound}
              <p class="note path">Found at {playerFound}</p>
            {:else}
              <p class="problem">
                {PLAYER_LABEL[form.player]} is not installed.
                <button type="button" class="text-button" onclick={() => api.openPlayerDownload(form.player)}>
                  Get {PLAYER_LABEL[form.player]}
                </button>
              </p>
            {/if}
          </div>
          <div class="field">
            <span>Streamlink</span>
            {#if form.streamlinkPath.trim()}
              <p class="note">Uses the location under Custom locations.</p>
            {:else if tools?.streamlink}
              <p class="note path">Found at {tools.streamlink}</p>
            {:else}
              <StreamlinkInstall {install} {oninstall} />
            {/if}
          </div>
          <details open={!!(settings.streamlinkPath || settings.playerPath)}>
            <summary>Custom locations</summary>
            <label>
              Streamlink location
              <input
                bind:value={form.streamlinkPath}
                placeholder={tools?.streamlink ? `Auto: ${tools.streamlink}` : 'Path to streamlink.exe'}
                spellcheck="false"
              />
            </label>
            <label>
              {PLAYER_LABEL[form.player]} location
              <input
                bind:value={form.playerPath}
                placeholder={playerFound ? `Auto: ${playerFound}` : `Path to ${form.player}.exe`}
                spellcheck="false"
              />
            </label>
          </details>
        </fieldset>

        <fieldset>
          <legend>Watching</legend>
          <div class="field" role="radiogroup" aria-labelledby="chat-label">
            <span id="chat-label">Open chat when a stream plays</span>
            <Segmented name="chat" options={CHAT_MODES} bind:value={form.chat} />
          </div>
          <div class="pair">
            <label>
              Quality
              <input bind:value={form.quality} list="qualities" spellcheck="false" />
              <datalist id="qualities">
                <option value="best"></option>
                <option value="1080p60,best"></option>
                <option value="720p60,best"></option>
                <option value="480p,best"></option>
                <option value="worst"></option>
                <option value="audio_only"></option>
              </datalist>
            </label>
            <div class="field" role="radiogroup" aria-labelledby="preferred-label">
              <span id="preferred-label">Play first on</span>
              <Segmented name="preferred" options={PLATFORM_OPTIONS} bind:value={form.preferredPlatform} />
            </div>
          </div>
          <label>
            Extra Streamlink options
            <input bind:value={form.extraArgs} placeholder="--twitch-low-latency" spellcheck="false" />
          </label>
        </fieldset>
      </div>

      <div class="panel" role="tabpanel" id={panelId('sources')} aria-labelledby={tabId('sources')} hidden={tab !== 'sources'}>
        <fieldset>
          <legend>Where to look</legend>
          <label>
            Twitch tags
            <input bind:value={form.tagsText} placeholder="NoPixelV, NoPixel" spellcheck="false" />
          </label>
          <label>
            Kick event
            <span class="prefixed">
              <span aria-hidden="true">kick.com/events/</span>
              <input bind:value={form.kickEvent} pattern="[A-Za-z0-9_\-]+" required spellcheck="false" />
            </span>
          </label>
        </fieldset>

        <fieldset>
          <legend>What to show</legend>
          <label>
            Only this category
            <input bind:value={form.category} placeholder="Any category" spellcheck="false" />
          </label>
          <label>
            Check every
            <span class="suffixed">
              <input type="number" bind:value={form.pollSeconds} min="30" max="600" step="10" required />
              <span>seconds</span>
            </span>
          </label>
        </fieldset>
      </div>

      <div class="panel" role="tabpanel" id={panelId('streamers')} aria-labelledby={tabId('streamers')} hidden={tab !== 'streamers'}>
        <fieldset>
          <legend>Favorites</legend>
          {#if form.favorites.length}
            <ul class="chips">
              {#each form.favorites as favorite, i (favorite.key)}
                <li>
                  {favorite.name}
                  <button type="button" onclick={() => form.favorites.splice(i, 1)} aria-label="Remove {favorite.name}">
                    <Icon name="close" size={14} />
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <div class="add">
            <input
              bind:value={form.newFavorite}
              placeholder="Channel name"
              aria-label="Add a favorite by channel name"
              spellcheck="false"
              onkeydown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                  addFavorite()
                }
              }}
            />
            <button type="button" class="secondary" onclick={addFavorite}><Icon name="plus" size={16} /> Add</button>
          </div>
        </fieldset>

        <fieldset>
          <legend>Added channels</legend>
          <p class="note">Shown when live in the category, even if the title does not mention NoPixel.</p>
          {#if addedGroups.length}
            <ul class="chips">
              {#each addedGroups as group (group.login)}
                <li>
                  {group.login}
                  {#each group.platforms as platform (platform)}
                    <span class={['site', platform]}>{PLATFORM_LABEL[platform]}</span>
                  {/each}
                  <button
                    type="button"
                    onclick={() => (form.added = form.added.filter((a) => a.login !== group.login))}
                    aria-label="Remove {group.login}"
                  >
                    <Icon name="close" size={14} />
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <div class="add">
            <input
              bind:value={form.newChannels}
              placeholder="Names or links"
              aria-label="Channels to add, separated by commas"
              spellcheck="false"
              onkeydown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                  addChannels()
                }
              }}
            />
            <select bind:value={form.newSites} aria-label="Sites to check">
              <option value="both">Both sites</option>
              <option value="twitch">Twitch</option>
              <option value="kick">Kick</option>
            </select>
            <button type="button" class="secondary" onclick={addChannels}><Icon name="plus" size={16} /> Add</button>
          </div>
        </fieldset>

        <fieldset>
          <legend>Linked names</legend>
          <p class="note">Join one streamer who uses a different name on each site.</p>
          {#each form.links as link, i (i)}
            <div class="link">
              <input bind:value={link.twitch} placeholder="Twitch name" aria-label="Twitch name" spellcheck="false" />
              <input bind:value={link.kick} placeholder="Kick name" aria-label="Kick name" spellcheck="false" />
              <button type="button" class="icon" onclick={() => form.links.splice(i, 1)} aria-label="Remove link">
                <Icon name="close" size={16} />
              </button>
            </div>
          {/each}
          <button type="button" class="secondary" onclick={() => form.links.push({ twitch: '', kick: '' })}>
            <Icon name="plus" size={16} /> Link names
          </button>
        </fieldset>

        <fieldset>
          <legend>Hidden</legend>
          {#if form.hidden.length}
            <ul class="chips">
              {#each form.hidden as entry (entry.key)}
                <li>
                  {entry.name}
                  <button
                    type="button"
                    onclick={() => (form.hidden = form.hidden.filter((h) => h.key !== entry.key))}
                    aria-label="Unhide {entry.name}"
                  >
                    <Icon name="close" size={14} />
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="note">Hide a streamer with the eye button on their tile.</p>
          {/if}
        </fieldset>
      </div>

      <div class="panel" role="tabpanel" id={panelId('app')} aria-labelledby={tabId('app')} hidden={tab !== 'app'}>
        <fieldset>
          <legend>Window and alerts</legend>
          <label class="check">
            <input type="checkbox" bind:checked={form.notifyFavorites} />
            Notify me when a favorite goes live
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={form.closeToTray} />
            Keep running in the tray when the window closes
          </label>
        </fieldset>

        <fieldset>
          <legend>Updates</legend>
          <p class="note">You have version {updater.version}. New versions are checked every 6 hours.</p>
          <div class="row">
            {#if readyUpdate}
              <button type="button" class="primary" onclick={() => updater.install()}>
                Update to {readyUpdate.version} and restart
              </button>
            {:else}
              <button
                type="button"
                class="secondary"
                onclick={() => updater.check()}
                disabled={updater.state.kind === 'checking' || updater.state.kind === 'downloading'}
              >
                Check for updates
              </button>
            {/if}
            <p class="note" role="status">{updateStatus}</p>
          </div>
        </fieldset>

        <fieldset>
          <legend>Setup</legend>
          <p class="note">Goes through Streamlink, your player, and chat again. Save first if you changed anything here.</p>
          <button type="button" class="secondary" onclick={onsetup}>Run setup again</button>
        </fieldset>

        <fieldset>
          <legend>Support</legend>
          <p class="note">NoPixel Live is free. If you enjoy it, you can support it on Ko-fi.</p>
          <button type="button" class="secondary" onclick={() => api.openSupportPage()}>
            Support on Ko-fi <Icon name="external" size={15} />
          </button>
        </fieldset>
      </div>
    </div>

    <footer>
      {#if error}
        <p class="problem" role="alert">{error}</p>
      {/if}
      <button type="button" class="secondary" onclick={() => dialog?.close()}>Cancel</button>
      <button type="submit" class="primary" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 32px));
    height: min(680px, calc(100vh - 32px));
    max-width: none;
    max-height: none;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--raised);
    color: var(--text);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.7);
  }

  form {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    height: 100%;
  }

  header,
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 20px;
  }

  header {
    border-bottom: 1px solid var(--line);
  }

  footer {
    border-top: 1px solid var(--line);
    justify-content: flex-end;
  }

  footer .problem {
    margin-right: auto;
  }

  h2 {
    font-size: 20px;
    font-weight: 700;
    margin-right: auto;
  }

  .layout {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr);
    min-height: 0;
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px;
    border-right: 1px solid var(--line);
  }

  [role='tab'] {
    flex: none;
    height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
  }

  [role='tab']:hover {
    color: var(--text);
  }

  [role='tab'][aria-selected='true'] {
    color: var(--text);
    background: var(--bg);
  }

  .panel {
    min-height: 0;
    overflow-y: auto;
    padding: 4px 24px 24px;
    overscroll-behavior: contain;
  }

  fieldset {
    border: 0;
    padding: 0;
    margin: 20px 0 0;
    display: grid;
    gap: 12px;
  }

  legend {
    font-weight: 700;
    font-size: 16px;
    padding: 0;
    margin-bottom: 12px;
  }

  label,
  .field {
    display: grid;
    gap: 5px;
    font-size: 14px;
    color: var(--muted);
  }

  select,
  input:not([type='checkbox'], [type='radio']) {
    width: 100%;
    min-width: 0;
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--text);
  }

  input::placeholder {
    color: var(--muted);
    opacity: 0.6;
    font-style: italic;
  }

  input:invalid {
    border-color: var(--text);
    border-style: dashed;
  }

  .pair {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 12px;
  }

  .prefixed,
  .suffixed {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }

  .prefixed input {
    flex: 1;
  }

  .suffixed input {
    width: 96px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text);
    cursor: pointer;
  }

  .check input {
    width: 18px;
    height: 18px;
    accent-color: var(--text);
  }

  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chips .site {
    margin-left: 4px;
    font-size: 13px;
    font-weight: 600;
  }

  .add select {
    width: auto;
    flex: none;
  }

  .chips li {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-left: 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    font-size: 14px;
  }

  .chips button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    background: none;
    color: var(--muted);
  }

  .chips button:hover {
    color: var(--text);
  }

  .add,
  .link {
    display: flex;
    gap: 8px;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
  }

  .row .note {
    flex: 1 1 200px;
    overflow-wrap: anywhere;
  }

  .secondary:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .note {
    font-size: 14px;
    color: var(--muted);
  }

  .path {
    overflow-wrap: anywhere;
  }

  .text-button {
    padding: 0;
    border: 0;
    background: none;
    font-weight: 700;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  details label {
    margin-top: 12px;
  }

  summary {
    font-size: 14px;
    color: var(--muted);
    cursor: pointer;
  }

  .problem {
    font-size: 14px;
    font-weight: 600;
  }

  button.icon {
    display: grid;
    place-items: center;
    flex: none;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
  }

  button.icon:hover {
    color: var(--text);
    background: var(--bg);
  }

  .secondary,
  .primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 36px;
    padding: 0 14px;
    border-radius: var(--radius);
    font-weight: 600;
    white-space: nowrap;
  }

  .secondary {
    justify-self: start;
    border: 1px solid var(--line);
    background: none;
  }

  .secondary:hover {
    background: var(--bg);
  }

  .primary {
    border: 0;
    min-width: 96px;
    background: var(--text);
    color: var(--bg);
  }

  .primary:disabled {
    opacity: 0.6;
    cursor: progress;
  }

  @media (max-width: 640px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto minmax(0, 1fr);
    }

    .tabs {
      flex-direction: row;
      overflow-x: auto;
      padding: 8px 12px;
      border-right: 0;
      border-bottom: 1px solid var(--line);
      scrollbar-width: none;
    }

    .panel {
      padding: 4px 16px 20px;
    }
  }
</style>
