<script lang="ts">
  import { CAMERA_FPS_OPTIONS } from './config'

  let {
    selected,
    onSelect,
  }: {
    selected: number
    onSelect: (value: number) => void
  } = $props()
</script>

<div class="row">
  {#each CAMERA_FPS_OPTIONS as value}
    <button class:selected={selected === value} type="button" onclick={() => onSelect(value)}>
      {value}
    </button>
  {/each}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }

  button {
    min-height: 36px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.88);
    border: 1px solid var(--fg-line);
    color: var(--fg-text);
    font-size: 0.82rem;
    font-weight: 700;
    transition:
      transform 120ms ease,
      border-color 120ms ease,
      background 120ms ease,
      color 120ms ease;
  }

  button.selected {
    background: var(--fg-accent);
    border-color: transparent;
    color: white;
  }
</style>
