<script lang="ts">
  import { untrack } from 'svelte'
  import Icon from './Icon.svelte'
  import Logo from './Logo.svelte'
  import Segmented from './Segmented.svelte'
  import StreamlinkInstall, { type InstallState } from './StreamlinkInstall.svelte'
  import { api } from './api'
  import { CHAT_MODES, PLATFORM_OPTIONS, PLAYER_LABEL, PLAYER_OPTIONS } from './format'
  import type { ChatMode, Player, Settings, Tools } from './types'

  interface Props {
    settings: Settings
    tools: Tools | null
    install: InstallState
    oninstall: () => Promise<string | null>
    ondetect: () => Promise<void>
    onfinish: (settings: Settings) => Promise<void>
    onclose: () => void
  }

  let { settings, tools, install, oninstall, ondetect, onfinish, onclose }: Props = $props()

  type Check = { state: 'checking' } | { state: 'missing' } | { state: 'ok'; version: string } | { state: 'failed'; message: string }

  const STEPS = ['Streamlink', 'Player', 'Chat', 'Done']
  const CHAT_SUMMARY: Record<ChatMode, string> = {
    window: 'Chat opens in a window next to the video.',
    browser: 'Chat opens in your web browser.',
    off: 'Chat does not open.',
  }

  const otherPlayer = (player: Player): Player => (player === 'vlc' ? 'mpv' : 'vlc')

  let draft = $derived.by(() => {
    const found = untrack(() => tools)
    const swap = !settings.playerPath && !found?.[settings.player] && !!found?.[otherPlayer(settings.player)]
    const next = $state({
      streamlinkPath: settings.streamlinkPath,
      player: swap ? otherPlayer(settings.player) : settings.player,
      playerPath: settings.playerPath,
      chat: settings.chat,
      preferredPlatform: settings.preferredPlatform,
      notifyFavorites: settings.notifyFavorites,
      closeToTray: settings.closeToTray,
    })
    return next
  })
  let step = $state(0)
  let saving = $state(false)
  let error = $state<string | null>(null)
  let dialog = $state<HTMLDialogElement>()

  let streamlinkTarget = $derived(draft.streamlinkPath.trim() || tools?.streamlink || '')
  let streamlinkResult = $state.raw<{ target: string; check: Check } | null>(null)
  let streamlink = $derived<Check>(
    !streamlinkTarget
      ? { state: 'missing' }
      : streamlinkResult?.target === streamlinkTarget
        ? streamlinkResult.check
        : { state: 'checking' },
  )

  let customPlayerPath = $derived(draft.playerPath.trim())
  let customPlayerResult = $state.raw<{ path: string; found: boolean } | null>(null)
  let playerFound = $derived<boolean | null>(
    customPlayerPath
      ? customPlayerResult?.path === customPlayerPath
        ? customPlayerResult.found
        : null
      : !!tools?.[draft.player],
  )
  let playerLabel = $derived(PLAYER_LABEL[draft.player])
  let ready = $derived(streamlink.state === 'ok' && playerFound === true)
  let last = $derived(step === STEPS.length - 1)

  $effect(() => {
    if (streamlinkTarget) checkStreamlink(streamlinkTarget, draft.streamlinkPath)
  })

  $effect(() => {
    const path = customPlayerPath
    if (!path) return
    api.checkPlayer(draft.player, path).then((found) => (customPlayerResult = { path, found: found !== null }))
  })

  function checkStreamlink(target: string, configured: string) {
    api.checkStreamlink(configured).then(
      (version) =>
        (streamlinkResult = { target, check: version === null ? { state: 'missing' } : { state: 'ok', version } }),
      (e) => (streamlinkResult = { target, check: { state: 'failed', message: String(e) } }),
    )
  }

  function openWithPrimaryFocus(node: HTMLDialogElement) {
    node.showModal()
    node.querySelector<HTMLButtonElement>('footer .primary')?.focus()
  }

  async function installStreamlink() {
    const broken = streamlink.state === 'failed'
    const path = await oninstall()
    if (path && broken) draft.streamlinkPath = path
  }

  async function next() {
    if (!last) {
      step += 1
      return
    }
    saving = true
    error = null
    try {
      await onfinish({ ...settings, ...draft, setupDone: true })
      dialog?.close()
    } catch (e) {
      error = String(e)
    } finally {
      saving = false
    }
  }
</script>

{#snippet status(ok: boolean, text: string)}
  <p class={['status', !ok && 'problem']}>
    <Icon name={ok ? 'check' : 'alert'} size={16} />
    <span>{text}</span>
  </p>
{/snippet}

<dialog bind:this={dialog} {@attach openWithPrimaryFocus} {onclose} aria-labelledby="setup-title">
  <header>
    <Logo size={26} />
    <h2 id="setup-title">Set up NoPixel Live</h2>
    <button type="button" class="text-button" onclick={() => dialog?.close()}>Skip setup</button>
  </header>

  <ol class="steps" aria-label="Setup steps">
    {#each STEPS as name, i (name)}
      <li class={[i === step && 'current', i < step && 'done']} aria-current={i === step ? 'step' : undefined}>
        {#if i < step}<Icon name="check" size={14} />{:else}<span class="n">{i + 1}</span>{/if}
        {name}
      </li>
    {/each}
  </ol>

  <div class="body">
    {#if step === 0}
      <h3>Get Streamlink</h3>
      <p class="lead">
        NoPixel Live plays streams in your own video player. Streamlink is the free tool that gets the video from
        Twitch and Kick.
      </p>
      {#if streamlink.state === 'checking'}
        <p class="status">Checking Streamlink…</p>
      {:else if streamlink.state === 'ok'}
        {@render status(true, streamlink.version ? `Streamlink ${streamlink.version} works.` : 'Streamlink works.')}
        <p class="note path">{streamlinkTarget}</p>
      {:else}
        {#if streamlink.state === 'failed'}
          {@render status(false, `Streamlink is installed but did not start. ${streamlink.message}`)}
        {:else if !install.running}
          {@render status(false, 'Streamlink is not installed yet.')}
        {/if}
        <StreamlinkInstall {install} oninstall={installStreamlink} />
      {/if}
    {:else if step === 1}
      <h3>Choose a video player</h3>
      <p class="lead">The video plays in VLC or mpv. Both are free. Pick the one you have, or the one you want.</p>
      <div class="field" role="radiogroup" aria-label="Player">
        <Segmented
          name="setup-player"
          options={PLAYER_OPTIONS}
          bind:value={draft.player}
          onchange={() => (draft.playerPath = '')}
        />
      </div>
      {#if playerFound === null}
        <p class="status">Checking that location…</p>
      {:else if playerFound}
        {@render status(true, `${playerLabel} is ready.`)}
        <p class="note path">{customPlayerPath || tools?.[draft.player]}</p>
      {:else}
        {@render status(
          false,
          customPlayerPath ? `${playerLabel} is not at that location.` : `${playerLabel} is not installed yet.`,
        )}
        {#if !customPlayerPath}
          <div class="row">
            <button type="button" class="primary" onclick={() => api.openPlayerDownload(draft.player)}>
              Get {playerLabel} <Icon name="external" size={15} />
            </button>
            <button type="button" class="secondary" onclick={ondetect}>Check again</button>
          </div>
          <p class="note">After you install it, come back to this window. NoPixel Live checks again by itself.</p>
        {/if}
      {/if}
      <details open={!!customPlayerPath}>
        <summary>{playerLabel} is in another folder</summary>
        <label>
          {playerLabel} location
          <input bind:value={draft.playerPath} placeholder="Path to {draft.player}.exe" spellcheck="false" />
        </label>
      </details>
    {:else if step === 2}
      <h3>Chat and alerts</h3>
      <div class="field" role="radiogroup" aria-labelledby="setup-chat-label">
        <span id="setup-chat-label">Open chat when a stream plays</span>
        <Segmented name="setup-chat" options={CHAT_MODES} bind:value={draft.chat} />
      </div>
      <div class="field" role="radiogroup" aria-labelledby="setup-preferred-label">
        <span id="setup-preferred-label">Play first on, when a streamer is live on both sites</span>
        <Segmented name="setup-preferred" options={PLATFORM_OPTIONS} bind:value={draft.preferredPlatform} />
      </div>
      <label class="check">
        <input type="checkbox" bind:checked={draft.notifyFavorites} />
        Notify me when a favorite goes live
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={draft.closeToTray} />
        Keep running in the tray when the window closes
      </label>
    {:else}
      <h3>{ready ? "You're ready" : 'Almost ready'}</h3>
      <ul class="summary">
        <li>
          {#if streamlink.state === 'ok'}
            {@render status(true, 'Streamlink works.')}
          {:else if streamlink.state === 'checking'}
            <p class="status">Checking Streamlink…</p>
          {:else}
            {@render status(false, 'Streamlink is not working yet.')}
            <button type="button" class="text-button" onclick={() => (step = 0)}>Fix</button>
          {/if}
        </li>
        <li>
          {#if playerFound}
            {@render status(true, `${playerLabel} is ready.`)}
          {:else}
            {@render status(false, `${playerLabel} is not found yet.`)}
            <button type="button" class="text-button" onclick={() => (step = 1)}>Fix</button>
          {/if}
        </li>
        <li>{@render status(true, CHAT_SUMMARY[draft.chat])}</li>
      </ul>
      <p class="lead">
        {ready ? 'Click any stream to watch it.' : 'Streams cannot play until both are fixed.'}
        You can change all of this later in Settings.
      </p>
    {/if}
  </div>

  <footer>
    {#if error}
      <p class="status problem" role="alert">{error}</p>
    {/if}
    {#if step > 0}
      <button type="button" class="secondary" onclick={() => (step -= 1)}>Back</button>
    {/if}
    <button type="button" class="primary" onclick={next} disabled={saving}>
      {#if !last}Next{:else if saving}Saving…{:else if ready}Start watching{:else}Finish{/if}
    </button>
  </footer>
</dialog>

<style>
  dialog {
    width: min(580px, calc(100vw - 32px));
    height: min(540px, calc(100vh - 32px));
    max-width: none;
    max-height: none;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--raised);
    color: var(--text);
  }

  dialog[open] {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.85);
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
    justify-content: flex-end;
    border-top: 1px solid var(--line);
  }

  footer .problem {
    margin-right: auto;
  }

  h2 {
    margin-right: auto;
    font-size: 20px;
    font-weight: 700;
  }

  h3 {
    font-size: 18px;
    font-weight: 700;
  }

  .steps {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 20px;
    margin: 0;
    padding: 14px 24px 0;
    list-style: none;
    font-size: 14px;
    color: var(--muted);
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .steps .n {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 1px solid var(--line);
    border-radius: 50%;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .steps .current {
    color: var(--text);
    font-weight: 600;
  }

  .steps .current .n {
    border-color: var(--text);
    background: var(--text);
    color: var(--bg);
  }

  .body {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 24px 24px;
    overscroll-behavior: contain;
  }

  .lead {
    color: var(--muted);
  }

  .status {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .status :global(svg) {
    flex: none;
    margin-top: 3px;
  }

  .note {
    font-size: 14px;
    color: var(--muted);
  }

  .path {
    margin-top: -8px;
    overflow-wrap: anywhere;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  label,
  .field {
    display: grid;
    gap: 5px;
    font-size: 14px;
    color: var(--muted);
  }

  details {
    align-self: stretch;
  }

  details label {
    margin-top: 12px;
  }

  summary {
    font-size: 14px;
    color: var(--muted);
    cursor: pointer;
  }

  input:not([type='checkbox']) {
    width: 100%;
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

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 15px;
    color: var(--text);
    cursor: pointer;
  }

  .check input {
    width: 18px;
    height: 18px;
    accent-color: var(--text);
  }

  .summary {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .summary li {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  .text-button {
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: 14px;
    font-weight: 700;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .text-button:hover {
    color: var(--text);
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
    border: 1px solid var(--line);
    background: none;
  }

  .secondary:hover {
    background: var(--bg);
  }

  .primary {
    min-width: 96px;
    border: 0;
    background: var(--text);
    color: var(--bg);
  }

  .primary:disabled {
    opacity: 0.6;
    cursor: progress;
  }
</style>
