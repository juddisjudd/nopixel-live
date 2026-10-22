<script lang="ts">
  import Icon from './Icon.svelte'
  import { PLATFORM_LABEL, compact, orderedChannels, sameCategory, thumbnailUrl, uptime } from './format'
  import type { Platform, Streamer } from './types'

  interface Props {
    streamer: Streamer
    favorite: boolean
    preferred: Platform
    homeCategory: string
    updatedAt: number | null
    now: number
    onplay: (platform: Platform) => void
    onopen: (platform: Platform) => void
    ontogglefavorite: () => void
    onhide: () => void
  }

  let { streamer, favorite, preferred, homeCategory, updatedAt, now, onplay, onopen, ontogglefavorite, onhide }: Props =
    $props()

  let channels = $derived(orderedChannels(streamer, preferred))
  let primary = $derived(channels[0])
  let thumbnail = $derived(channels.map((c) => thumbnailUrl(c, updatedAt)).find(Boolean) ?? null)
  let live = $derived(uptime(streamer.channels.find((c) => c.startedAt)?.startedAt ?? null, now))
  let category = $derived(
    streamer.channels.map((c) => c.category).find((c) => c && !sameCategory(c, homeCategory)) ?? null,
  )
</script>

<article class="tile" aria-label={streamer.displayName}>
  <button
    class="thumb"
    onclick={() => onplay(primary.platform)}
    aria-label="Play {streamer.displayName} on {PLATFORM_LABEL[primary.platform]}"
  >
    {#if thumbnail}
      <img src={thumbnail} alt="" loading="lazy" decoding="async" />
    {/if}
    <span class="glyph"><Icon name="play" size={28} filled /></span>
  </button>

  <div class="who">
    {#if streamer.avatar}
      <img class="avatar" src={streamer.avatar} alt="" width="26" height="26" loading="lazy" />
    {/if}
    <h3>{streamer.displayName}</h3>
    {#if live}
      <span class="uptime" title="Live for {live}">{live}</span>
    {/if}
    <button
      class="hide"
      onclick={onhide}
      aria-label="Hide {streamer.displayName}"
      title="Hide this streamer"
    >
      <Icon name="hide" size={17} />
    </button>
    <button
      class={['star', favorite && 'on']}
      onclick={ontogglefavorite}
      aria-pressed={favorite}
      aria-label={favorite ? `Remove ${streamer.displayName} from favorites` : `Add ${streamer.displayName} to favorites`}
      title={favorite ? 'Remove from favorites' : 'Add to favorites'}
    >
      <Icon name="star" size={18} filled={favorite} />
    </button>
  </div>

  <p class="title" title={primary.title}>{primary.title}</p>
  {#if category}
    <p class="category">{category}</p>
  {/if}

  <div class="sites">
    {#each channels as channel (channel.platform)}
      <div class="site">
        <button
          class="watch"
          onclick={() => onplay(channel.platform)}
          aria-label="Play on {PLATFORM_LABEL[channel.platform]}, {channel.viewers.toLocaleString()} viewers"
        >
          <span class={channel.platform}>{PLATFORM_LABEL[channel.platform]}</span>
          <span class="viewers">{compact(channel.viewers)}</span>
        </button>
        <button
          class="open"
          onclick={() => onopen(channel.platform)}
          aria-label="Open {streamer.displayName} on {PLATFORM_LABEL[channel.platform]} in your browser"
          title="Open in browser"
        >
          <Icon name="external" size={15} />
        </button>
      </div>
    {/each}
  </div>
</article>

<style>
  .tile {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .thumb {
    position: relative;
    display: block;
    aspect-ratio: 16 / 9;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--raised);
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .glyph {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--text);
    background: rgb(0 0 0 / 0.5);
    opacity: 0;
    transition: opacity 120ms ease-out;
  }

  .thumb:hover .glyph,
  .thumb:focus-visible .glyph {
    opacity: 1;
  }

  .thumb:focus-visible {
    outline-offset: -2px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    min-width: 0;
  }

  .avatar {
    border-radius: 50%;
    flex: none;
  }

  h3 {
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .uptime {
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .hide,
  .star {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--muted);
  }

  .hide {
    margin-left: auto;
    opacity: 0;
  }

  .tile:hover .hide,
  .hide:focus-visible {
    opacity: 1;
  }

  .hide:hover,
  .star:hover,
  .star.on {
    color: var(--text);
  }

  .title {
    margin-top: 4px;
    min-height: 2lh;
    color: var(--muted);
    font-size: 14px;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  .category {
    margin-top: 4px;
    font-size: 13px;
    font-weight: 600;
  }

  .sites {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: auto;
    padding-top: 10px;
  }

  .site {
    display: flex;
    border: 1px solid var(--line);
    border-radius: var(--radius);
  }

  .site button {
    border: 0;
    background: none;
    min-height: 32px;
  }

  .site button:hover {
    background: var(--raised);
  }

  .watch {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    font-weight: 600;
    font-size: 14px;
    border-radius: var(--radius) 0 0 var(--radius);
  }

  .viewers {
    color: var(--muted);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }

  .site .open {
    display: grid;
    place-items: center;
    width: 28px;
    padding: 0;
    color: var(--muted);
    border-left: 1px solid var(--line);
    border-radius: 0 var(--radius) var(--radius) 0;
  }

  .site .open:hover {
    color: var(--text);
  }
</style>
