<script lang="ts">
  import { onMount } from 'svelte'

  let flashing = $state(false)
  let flashKey = $state(0)

  onMount(async () => {
    try {
      const { listen } = await import('@tauri-apps/api/event')
      await listen<void>('flash', () => {
        flashing = true
        flashKey++
      })
    } catch {}
  })
</script>

{#if flashing}
  {#key flashKey}
    <div class="overlay" onanimationend={() => { flashing = false }}></div>
  {/key}
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    pointer-events: none;
    animation: flash-pulse 900ms cubic-bezier(0.16, 0.84, 0.32, 1) forwards;
  }

  @keyframes flash-pulse {
    0%   { background-color: rgba(220, 40, 40, 0); }
    24%  { background-color: rgba(220, 40, 40, 0.35); }
    58%  { background-color: rgba(220, 40, 40, 0.15); }
    100% { background-color: rgba(220, 40, 40, 0); }
  }
</style>
