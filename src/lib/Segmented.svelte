<script lang="ts" generics="T extends string">
  interface Option {
    value: T
    label: string
    tone?: string
  }

  interface Props {
    name: string
    options: readonly Option[]
    value: T
    onchange?: (value: T) => void
  }

  let { name, options, value = $bindable(), onchange }: Props = $props()
</script>

<div class="segmented">
  {#each options as option (option.value)}
    <label class={['segment', value === option.value && 'on']}>
      <input type="radio" {name} value={option.value} bind:group={value} onchange={() => onchange?.(option.value)} />
      <span class={[value !== option.value && option.tone]}>{option.label}</span>
    </label>
  {/each}
</div>

<style>
  .segmented {
    justify-self: start;
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
    white-space: nowrap;
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
</style>
