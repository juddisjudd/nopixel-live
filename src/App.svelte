<script lang="ts">
  import { onMount } from 'svelte'
  import Icon from './lib/Icon.svelte'
  import Logo from './lib/Logo.svelte'
  import SettingsPanel from './lib/SettingsPanel.svelte'
  import SetupWizard from './lib/SetupWizard.svelte'
  import StreamerTile from './lib/StreamerTile.svelte'
  import StreamlinkInstall, { type InstallState } from './lib/StreamlinkInstall.svelte'
  import { api, onInstallProgress, onPlayerError, onSnapshot } from './lib/api'
  import {
    DEFAULT_CATEGORY,
    PLATFORM_LABEL,
    PLAYER_LABEL,
    ago,
    matchesQuery,
    matchesSite,
    megabytes,
    type SiteFilter,
  } from './lib/format'
  import type { Platform, Player, Settings, Snapshot, Streamer, Tools } from './lib/types'
  import { Updater } from './lib/updater.svelte'

  interface Toast {
    id: number
    text: string
    tone: 'info' | 'error'
    action?: { label: string; run: () => void }
  }

  const SITE_FILTERS: { value: SiteFilter; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'twitch', label: 'Twitch' },
    { value: 'kick', label: 'Kick' },
    { value: 'both', label: 'Both' },
  ]

  let snapshot = $state.raw<Snapshot>({ streamers: [], updatedAt: null, errors: [] })
  let settings = $state.raw<Settings | null>(null)
  let tools = $state.raw<Tools | null>(null)
  let query = $state('')
  let site = $state<SiteFilter>('all')
  let settingsOpen = $state(false)
  let setupOpen = $state(false)
  let refreshing = $state(false)
  let now = $state(Date.now())
  let toasts = $state<Toast[]>([])
  let search = $state<HTMLInputElement>()
  let install = $state<InstallState>({ running: false, progress: null, error: null })
  const updater = new Updater()
  let nextToastId = 0

  let loading = $derived(snapshot.updatedAt === null)
  let preferred = $derived<Platform>(settings?.preferredPlatform ?? 'twitch')
  let player = $derived<Player>(settings?.player ?? 'vlc')
  let needsStreamlink = $derived(!!tools && !!settings && !settings.streamlinkPath && !tools.streamlink)
  let needsPlayer = $derived(!!tools && !!settings && !settings.playerPath && !tools[player])
  let update = $derived.by(() => {
    const state = updater.state
    if (state.kind === 'available') return { version: state.update.version, progress: null, error: null }
    if (state.kind === 'downloading') {
      const { received, total } = state
      const progress = total ? `Downloading ${megabytes(received)} of ${megabytes(total)} MB…` : 'Downloading…'
      return { version: state.update.version, progress, error: null }
    }
    if (state.kind === 'failed' && state.update) return { version: state.update.version, progress: null, error: state.message }
    return null
  })
  let favoriteKeys = $derived(new Set(settings?.favorites.map((f) => f.key) ?? []))
  let searched = $derived(snapshot.streamers.filter((s) => matchesQuery(s, query)))
  let counts = $derived(
    Object.fromEntries(SITE_FILTERS.map((f) => [f.value, searched.filter((s) => matchesSite(s, f.value)).length])),
  )
  let visible = $derived(searched.filter((s) => matchesSite(s, site)))
  let favorites = $derived(visible.filter((s) => favoriteKeys.has(s.key)))
  let others = $derived(visible.filter((s) => !favoriteKeys.has(s.key)))

  function applySnapshot(next: Snapshot) {
    if ((next.updatedAt ?? 0) >= (snapshot.updatedAt ?? 0)) snapshot = next
    refreshing = false
  }

  onMount(() => {
    const unlisten = [
      onSnapshot(applySnapshot),
      onPlayerError((e) => toast(`${e.name} on ${PLATFORM_LABEL[e.platform]} did not play. ${e.message}`, 'error')),
    ]
    api.snapshot().then(applySnapshot)
    api.settings().then((s) => {
      settings = s
      setupOpen = !s.setupDone
    })
    detectTools()
    const clock = setInterval(() => (now = Date.now()), 15_000)
    const stopUpdates = updater.start()
    return () => {
      clearInterval(clock)
      stopUpdates()
      for (const pending of unlisten) pending.then((stop) => stop())
    }
  })

  function toast(text: string, tone: Toast['tone'] = 'info', action?: Toast['action']) {
    const id = ++nextToastId
    toasts.push({ id, text, tone, action })
    setTimeout(() => dismiss(id), tone === 'error' || action ? 10_000 : 4_000)
  }

  function dismiss(id: number) {
    toasts = toasts.filter((t) => t.id !== id)
  }

  async function play(streamer: Streamer, platform: Platform) {
    try {
      await api.play(streamer.key, platform)
      toast(`Opening ${streamer.displayName} on ${PLATFORM_LABEL[platform]}…`)
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function openPage(streamer: Streamer, platform: Platform) {
    try {
      await api.openPage(streamer.key, platform)
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function toggleFavorite(streamer: Streamer) {
    try {
      settings = await api.toggleFavorite(streamer.key)
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function hide(streamer: Streamer) {
    try {
      settings = await api.hide(streamer.key)
      toast(`Hid ${streamer.displayName}.`, 'info', { label: 'Undo', run: () => unhide(streamer.key) })
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function unhide(key: string) {
    try {
      settings = await api.unhide(key)
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function refresh() {
    refreshing = true
    await api.refresh()
  }

  async function saveSettings(next: Settings) {
    settings = await api.saveSettings(next)
    await detectTools()
  }

  async function detectTools() {
    tools = await api.detectTools()
  }

  async function installStreamlink(): Promise<string | null> {
    install = { running: true, progress: null, error: null }
    const stop = await onInstallProgress((progress) => (install.progress = progress))
    try {
      const path = await api.installStreamlink()
      await detectTools()
      toast('Streamlink is installed. Click a stream to watch it.')
      return path
    } catch (e) {
      install.error = String(e)
      return null
    } finally {
      install.running = false
      stop()
    }
  }

  async function finishSetup(next: Settings) {
    settings = await api.saveSettings(next)
    await detectTools()
  }

  async function closeSetup() {
    setupOpen = false
    if (!settings || settings.setupDone) return
    try {
      settings = await api.saveSettings({ ...settings, setupDone: true })
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  async function openPlayerDownload(choice: Player) {
    try {
      await api.openPlayerDownload(choice)
    } catch (e) {
      toast(String(e), 'error')
    }
  }

  function onWindowKey(event: KeyboardEvent) {
    const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement
    if (event.key === '/' && !typing && !settingsOpen && !setupOpen) {
      event.preventDefault()
      search?.focus()
    }
  }
</script>

<svelte:window onkeydown={onWindowKey} onfocus={detectTools} />

{#snippet tile(streamer: Streamer)}
  <StreamerTile
    {streamer}
    {preferred}
    {now}
    homeCategory={settings?.category || DEFAULT_CATEGORY}
    updatedAt={snapshot.updatedAt}
    favorite={favoriteKeys.has(streamer.key)}
    onplay={(platform) => play(streamer, platform)}
    onopen={(platform) => openPage(streamer, platform)}
    ontogglefavorite={() => toggleFavorite(streamer)}
    onhide={() => hide(streamer)}
  />
{/snippet}

<header class="strip">
  <h1 class="brand"><Logo size={28} /><span class="visually-hidden">NoPixel Live</span></h1>

  <p class="count" aria-live="polite">
    {#if loading}
      Checking Twitch and Kick…
    {:else}
      <strong>{snapshot.streamers.length}</strong> live
    {/if}
  </p>

  <label class="search">
    <Icon name="search" size={16} />
    <span class="visually-hidden">Search streamers or characters</span>
    <input
      bind:this={search}
      bind:value={query}
      type="search"
      placeholder="Search streamers or characters"
      spellcheck="false"
      onkeydown={(event) => event.key === 'Escape' && (query = '')}
    />
  </label>

  <div class="filters" role="group" aria-label="Show streams from">
    {#each SITE_FILTERS as option (option.value)}
      <button class={[site === option.value && 'on']} aria-pressed={site === option.value} onclick={() => (site = option.value)}>
        <span class={[site !== option.value && option.value]}>{option.label}</span>
        <span class="n">{counts[option.value]}</span>
      </button>
    {/each}
  </div>

  <div class="actions">
    <span class="updated">{refreshing ? 'Refreshing…' : `Updated ${ago(snapshot.updatedAt, now)}`}</span>
    <button class="icon" onclick={refresh} disabled={refreshing} aria-label="Refresh now" title="Refresh now">
      <Icon name="refresh" />
    </button>
    <button class="icon" onclick={() => (settingsOpen = true)} disabled={!settings} aria-label="Settings" title="Settings">
      <Icon name="sliders" />
    </button>
  </div>
</header>

<div class="scroll">
  {#if needsStreamlink || needsPlayer || update}
    <div class="setup">
      {#if update}
        <div class="setup-row">
          <p>
            <strong>NoPixel Live {update.version} is ready.</strong> The app closes and opens again while it updates.
            {#if update.error}<span class="problem">{update.error}</span>{/if}
          </p>
          {#if update.progress}
            <span class="progress" role="status">{update.progress}</span>
          {:else}
            <button class="secondary" onclick={() => updater.install()}>
              {update.error ? 'Try again' : 'Update and restart'}
            </button>
          {/if}
        </div>
      {/if}
      {#if needsStreamlink}
        <div class="setup-row">
          <p><strong>Streamlink is needed to play streams.</strong> NoPixel Live can install it for you.</p>
          <StreamlinkInstall {install} oninstall={installStreamlink} />
        </div>
      {/if}
      {#if needsPlayer}
        <div class="setup-row">
          <p>
            <strong>{PLAYER_LABEL[player]} is not installed.</strong> Install it, or choose another player in Settings.
          </p>
          <button class="secondary" onclick={() => openPlayerDownload(player)}>
            Get {PLAYER_LABEL[player]} <Icon name="external" size={15} />
          </button>
        </div>
      {/if}
    </div>
  {/if}

  {#if snapshot.errors.length}
    <div class="errors" role="alert">
      {#each snapshot.errors as error (error)}
        <p><Icon name="alert" size={16} /> {error}</p>
      {/each}
    </div>
  {/if}

  <main>
    {#if loading}
      <div class="grid" aria-hidden="true">
        {#each { length: 8 }, i (i)}
          <div class="placeholder"><span></span><span></span><span></span></div>
        {/each}
      </div>
    {:else if visible.length === 0}
      <div class="empty">
        {#if query.trim()}
          <p>No one live matches “{query.trim()}”.</p>
          <button class="secondary" onclick={() => (query = '')}>Clear search</button>
        {:else if site !== 'all'}
          <p>No one is live on {site === 'both' ? 'both sites' : PLATFORM_LABEL[site]} right now.</p>
          <button class="secondary" onclick={() => (site = 'all')}>Show all sites</button>
        {:else}
          <p>No NoPixel streams are live right now.</p>
        {/if}
      </div>
    {:else}
      {#if favorites.length}
        <section aria-labelledby="favorites-heading">
          <h2 id="favorites-heading">Favorites</h2>
          <div class="grid">
            {#each favorites as streamer (streamer.key)}
              {@render tile(streamer)}
            {/each}
          </div>
        </section>
      {/if}
      {#if others.length}
        <section aria-labelledby="live-heading">
          <h2 id="live-heading" class={[!favorites.length && 'visually-hidden']}>Everyone else</h2>
          <div class="grid">
            {#each others as streamer (streamer.key)}
              {@render tile(streamer)}
            {/each}
          </div>
        </section>
      {/if}
    {/if}
  </main>
</div>

<div class="toasts">
  {#each toasts as item (item.id)}
    <div class={['toast', item.tone]} role={item.tone === 'error' ? 'alert' : 'status'}>
      {#if item.tone === 'error'}
        <Icon name="alert" size={16} />
      {/if}
      <p>{item.text}</p>
      {#if item.action}
        <button
          class="action"
          onclick={() => {
            item.action?.run()
            dismiss(item.id)
          }}>{item.action.label}</button
        >
      {/if}
      <button onclick={() => dismiss(item.id)} aria-label="Dismiss"><Icon name="close" size={14} /></button>
    </div>
  {/each}
</div>

{#if settingsOpen && settings}
  <SettingsPanel
    {settings}
    {tools}
    {install}
    {updater}
    oninstall={installStreamlink}
    onsave={saveSettings}
    onsetup={() => {
      settingsOpen = false
      setupOpen = true
    }}
    onclose={() => (settingsOpen = false)}
  />
{/if}

{#if setupOpen && settings && tools}
  <SetupWizard
    {settings}
    {tools}
    {install}
    oninstall={installStreamlink}
    ondetect={detectTools}
    onfinish={finishSetup}
    onclose={closeSetup}
  />
{/if}

<style>
  .strip {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px 16px;
    padding: 14px 24px;
    background: var(--bg);
    border-bottom: 1px solid var(--line);
  }

  .brand {
    display: flex;
    color: var(--text);
  }

  .count {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 16px;
    color: var(--muted);
    white-space: nowrap;
  }

  .count strong {
    font-size: 22px;
    font-weight: 700;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .search {
    flex: 1 1 220px;
    max-width: 420px;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    color: var(--muted);
    background: var(--raised);
  }

  .search:focus-within {
    border-color: var(--muted);
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: 0;
    background: none;
  }

  .search input::placeholder {
    color: var(--muted);
  }

  .filters {
    display: flex;
    height: 38px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .filters button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    border: 0;
    background: none;
    color: var(--muted);
    white-space: nowrap;
  }

  .filters button + button {
    border-left: 1px solid var(--line);
  }

  .filters button:hover {
    color: var(--text);
  }

  .filters button.on {
    color: var(--bg);
    background: var(--text);
  }

  .filters .n {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }

  .updated {
    margin-right: 6px;
    font-size: 13px;
    color: var(--muted);
    white-space: nowrap;
  }

  button.icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
  }

  button.icon:hover:not(:disabled) {
    color: var(--text);
    background: var(--raised);
  }

  button.icon:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .errors {
    padding: 10px 24px;
    background: var(--raised);
    border-bottom: 1px solid var(--line);
    font-size: 14px;
  }

  .errors p {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .setup {
    display: grid;
    gap: 14px;
    padding: 16px 24px;
    border-bottom: 1px solid var(--line);
    font-size: 15px;
  }

  .setup-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 20px;
  }

  .setup-row p {
    flex: 1 1 320px;
  }

  .setup-row .problem {
    display: block;
    margin-top: 4px;
    font-size: 14px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .progress {
    font-size: 14px;
    color: var(--muted);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  main {
    padding: 20px 24px 48px;
  }

  section + section {
    margin-top: 36px;
  }

  h2 {
    margin-bottom: 14px;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: 30px 20px;
  }

  .placeholder {
    display: grid;
    gap: 10px;
  }

  .placeholder span {
    display: block;
    height: 14px;
    border-radius: var(--radius);
    background: var(--raised);
  }

  .placeholder span:first-child {
    height: auto;
    aspect-ratio: 16 / 9;
  }

  .placeholder span:last-child {
    width: 60%;
  }

  .empty {
    display: grid;
    justify-items: start;
    gap: 14px;
    padding: 40px 0;
    font-size: 17px;
  }

  .secondary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: none;
    font-weight: 600;
  }

  .secondary:hover {
    background: var(--raised);
  }

  .toasts {
    position: fixed;
    left: 24px;
    bottom: 24px;
    z-index: 3;
    display: grid;
    gap: 8px;
    max-width: min(460px, calc(100vw - 48px));
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 8px 12px 14px;
    border-radius: var(--radius);
    background: var(--raised);
    border: 1px solid var(--line);
    font-size: 14px;
  }

  .toast.error {
    border-color: var(--muted);
  }

  .toast p {
    flex: 1;
    overflow-wrap: anywhere;
  }

  .toast > :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .toast .action {
    width: auto;
    padding: 0 8px;
    color: var(--text);
    font-weight: 700;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .toast button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
  }

  @media (max-width: 640px) {
    .strip,
    .setup,
    main {
      padding-left: 16px;
      padding-right: 16px;
    }

    .search {
      order: 3;
      flex-basis: 100%;
      max-width: none;
    }
  }
</style>
