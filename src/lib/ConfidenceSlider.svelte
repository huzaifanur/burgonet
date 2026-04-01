<script lang="ts">
  let {
    value,
    onValueChange,
  }: {
    value: number
    onValueChange: (value: number) => void
  } = $props()
</script>

<label class="field">
  <div class="copy">
    <span>Precision</span>
    <strong>{value}%</strong>
  </div>
  <div class="input-row">
    <input
      type="number"
      min="0"
      max="100"
      step="1"
      value={value}
      oninput={(event) => {
        const nextValue = (event.currentTarget as HTMLInputElement).valueAsNumber
        if (!Number.isNaN(nextValue)) {
          onValueChange(nextValue)
        }
      }}
    />
    <span class="suffix">%</span>
  </div>
  <p>Lower confidence levels trigger alerts sooner.</p>
</label>

<style>
  .field {
    display: grid;
    gap: 10px;
    padding: 14px;
    border-radius: 14px;
    background: var(--fg-panel);
    border: 1px solid var(--fg-line);
  }

  .copy {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--fg-muted);
    font-size: 0.82rem;
  }

  .copy strong {
    color: var(--fg-text);
  }

  .input-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
  }

  input {
    min-height: 44px;
    border-radius: 12px;
    border: 1px solid var(--fg-line);
    background: var(--fg-surface-strong);
    color: var(--fg-text);
    padding: 0 12px;
    font: inherit;
  }

  .suffix {
    color: var(--fg-muted);
    font-size: 0.82rem;
    font-weight: 700;
  }

  p {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.8rem;
  }
</style>
