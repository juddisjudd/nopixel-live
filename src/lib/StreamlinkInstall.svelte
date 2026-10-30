<script lang="ts" module>
  import type { InstallProgress } from './types'

  export interface InstallState {
    running: boolean
    progress: InstallProgress | null
    error: string | null
  }
</script>

<script lang="ts">
  import { megabytes } from './format'

  let { install, oninstall }: { install: InstallState; oninstall: () => void } = $props()

  let fraction = $derived.by(() => {
    const p = install.progress
    if (!p) return 0
    return p.stage === 'unpack' || p.total === 0 ? 1 : p.received / p.total
  })
  let status = $derived.by(() => {
    const p = install.progress
    if (!p) return 'Starting the download…'
    if (p.stage === 'unpack') return 'Unpacking. This takes a few seconds…'
    return `Downloading ${megabytes(p.received)} of ${megabytes(p.total)} MB…`
  })
</script>

{#if install.running}
  <div class="running" role="status">
    <progress max="1" value={fraction} aria-label="Streamlink install progress"></progress>
    <span>{status}</span>
  </div>
{:else}
  <div class="start">
    <button type="button" onclick={oninstall}>{install.error ? 'Try again' : 'Install Streamlink'}</button>
    <span>About 85 MB. No admin rights needed.</span>
  </div>
  {#if install.error}
    <p class="problem" role="alert">{install.error}</p>
  {/if}
{/if}

<style>
  .start {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    font-size: 14px;
    color: var(--muted);
  }

  button {
    height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius);
    background: var(--text);
    color: var(--bg);
    font-weight: 600;
    white-space: nowrap;
  }

  .running {
    display: grid;
    gap: 6px;
    min-width: 220px;
    font-size: 14px;
    color: var(--muted);
  }

  progress {
    appearance: none;
    width: 100%;
    height: 4px;
    border: 0;
    border-radius: 2px;
    overflow: hidden;
    background: var(--line);
  }

  progress::-webkit-progress-bar {
    background: var(--line);
  }

  progress::-webkit-progress-value {
    background: var(--text);
    transition: inline-size 200ms ease-out;
  }

  .problem {
    margin-top: 6px;
    font-size: 14px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
</style>
