<script lang="ts">
  import Icon from './Icon.svelte'
  import { PLATFORM_LABEL } from './format'
  import type { AddedChannel, Favorite, Hidden, Link, Platform, Settings, Tools } from './types'

  interface Props {
    settings: Settings
    tools: Tools | null
    onsave: (settings: Settings) => Promise<void>
    onclose: () => void
  }

  let { settings, tools, onsave, onclose }: Props = $props()

  interface Form {
    streamlinkPath: string
    playerPath: string
    quality: string
    extraArgs: string
    preferredPlatform: Platform
    pollSeconds: number
    tagsText: string
    kickEvent: string
    category: string
    notifyFavorites: boolean
    closeToTray: boolean
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
      playerPath: s.playerPath,
      quality: s.quality,
      extraArgs: s.extraArgs,
      preferredPlatform: s.preferredPlatform,
      pollSeconds: s.pollSeconds,
      tagsText: s.twitchTags.join(', '),
      kickEvent: s.kickEvent,
      category: s.category,
      notifyFavorites: s.notifyFavorites,
      closeToTray: s.closeToTray,
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
      playerPath: f.playerPath,
      quality: f.quality,
      extraArgs: f.extraArgs,
      preferredPlatform: f.preferredPlatform,
      pollSeconds: f.pollSeconds,
      twitchTags: f.tagsText.split(/[\s,]+/).filter(Boolean),
      kickEvent: f.kickEvent,
      category: f.category,
      notifyFavorites: f.notifyFavorites,
      closeToTray: f.closeToTray,
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
  let saving = $state(false)
  let error = $state<string | null>(null)
  let dialog = $state<HTMLDialogElement>()

  let streamlinkMissing = $derived(!tools?.streamlink && !form.streamlinkPath.trim())
  let addedGroups = $derived(
    [...new Set(form.added.map((a) => a.login))].map((login) => ({
      login,
      platforms: form.added.filter((a) => a.login === login).map((a) => a.platform),
    })),
  )

  const nameKey = (name: string) => name.toLowerCase().replace(/[^a-z0-9]/g, '')

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
  <form onsubmit={save}>
    <header>
      <h2 id="settings-title">Settings</h2>
      <button type="button" class="icon" onclick={() => dialog?.close()} aria-label="Close settings">
        <Icon name="close" />
      </button>
    </header>

    <div class="body">
      <fieldset>
        <legend>Playback</legend>
        <label>
          Streamlink
          <input
            bind:value={form.streamlinkPath}
            placeholder={tools?.streamlink ? `Auto: ${tools.streamlink}` : 'Path to streamlink.exe'}
            spellcheck="false"
          />
        </label>
        {#if streamlinkMissing}
          <p class="problem">Streamlink is not installed in a usual place. Enter the path to streamlink.exe.</p>
        {/if}
        <label>
          Player
          <input
            bind:value={form.playerPath}
            placeholder={tools?.player ? `Auto: ${tools.player}` : "Auto: Streamlink's default"}
            spellcheck="false"
          />
        </label>
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
            <div class="segmented">
              {#each ['twitch', 'kick'] as const as platform (platform)}
                <label class={['segment', form.preferredPlatform === platform && 'on']}>
                  <input type="radio" name="preferred" value={platform} bind:group={form.preferredPlatform} />
                  <span class={[form.preferredPlatform !== platform && platform]}>{PLATFORM_LABEL[platform]}</span>
                </label>
              {/each}
            </div>
          </div>
        </div>
        <label>
          Extra Streamlink options
          <input bind:value={form.extraArgs} placeholder="--twitch-low-latency" spellcheck="false" />
        </label>
      </fieldset>

      <fieldset>
        <legend>Finding streams</legend>
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

      <fieldset>
        <legend>App</legend>
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
    </div>

    <footer>
      {#if error}
        <p class="problem" role="alert">{error}</p>
      {/if}
      <button type="submit" class="primary" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    margin: 0 0 0 auto;
    width: min(440px, 100vw);
    height: 100vh;
    max-height: 100vh;
    padding: 0;
    border: 0;
    border-left: 1px solid var(--line);
    background: var(--raised);
    color: var(--text);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.7);
  }

  form {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header,
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
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

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 4px 20px 20px;
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

  .segmented {
    display: flex;
    height: 36px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .segment {
    display: flex;
    align-items: center;
    padding: 0 14px;
    color: var(--muted);
    cursor: pointer;
  }

  .segment input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .segment:has(input:focus-visible) {
    outline: 2px solid var(--muted);
    outline-offset: -4px;
  }

  .segment.on {
    color: var(--bg);
    background: var(--text);
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

  .note {
    font-size: 14px;
    color: var(--muted);
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
</style>
