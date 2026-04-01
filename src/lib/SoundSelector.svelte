<script lang="ts">
  import { isBuiltinSound, soundLabel, type BuiltinAlertSound } from './config'

  interface SoundOption {
    value: BuiltinAlertSound
    label: string
  }

  let {
    options,
    customPaths,
    selected,
    onSelect,
    onAddCustom,
  }: {
    options: SoundOption[]
    customPaths: string[]
    selected: string
    onSelect: (value: string) => void
    onAddCustom: () => void | Promise<void>
  } = $props()

  let customOptions = $derived(
    Array.from(
      new Set([
        ...customPaths,
        ...(selected.length > 0 && !isBuiltinSound(selected) ? [selected] : []),
      ]),
    ),
  )
</script>

<div class="stack">
  <label class="field">
    <span>Selected audio</span>
    <select value={selected} onchange={(event) => onSelect((event.currentTarget as HTMLSelectElement).value)}>
      <optgroup label="Built-in sounds">
        {#each options as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </optgroup>

      {#if customOptions.length > 0}
        <optgroup label="Custom audio files">
          {#each customOptions as path}
            <option value={path}>{soundLabel(path)}</option>
          {/each}
        </optgroup>
      {/if}
    </select>
  </label>

  <div class="actions">
    <button type="button" onclick={onAddCustom}>Add audio file</button>
    <p>Supported: .wav, .mp3. Recommended: .wav for the most reliable playback.</p>
  </div>
</div>

<style>
  .stack {
    display: grid;
    gap: 12px;
  }

  .field {
    display: grid;
    gap: 8px;
    color: var(--fg-muted);
    font-size: 0.82rem;
  }

  select {
    min-height: 44px;
    border-radius: 12px;
    border: 1px solid var(--fg-line);
    background: var(--fg-surface-strong);
    color: var(--fg-text);
    padding: 0 14px;
    font: inherit;
  }

  .actions {
    display: grid;
    gap: 8px;
  }

  button {
    min-height: 44px;
    border-radius: 12px;
    border: 1px solid var(--fg-line);
    background: rgba(255, 255, 255, 0.72);
    color: var(--fg-text);
    font: inherit;
    font-weight: 700;
  }

  button:hover {
    transform: translateY(-1px);
    background: var(--fg-surface-strong);
  }

  p {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.8rem;
  }
</style>
