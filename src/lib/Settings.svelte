<script lang="ts">
  import { onMount } from 'svelte'
  import {
    ZONE_OFFSET_LIMITS,
    DEFAULT_CONFIG,
    confidencePresetForValue,
    SOUND_OPTIONS,
  } from './config'
  import {
    attachRuntimeListeners,
    loadConfigFromBackend,
    pickAudioFileInBackend,
    saveConfigToBackend,
    testSoundInBackend,
    type PreviewPointPayload,
    type PreviewZonePayload,
    type RuntimeStatus,
  } from './backend'
  import ConfidenceSlider from './ConfidenceSlider.svelte'
  import DelaySelector from './DelaySelector.svelte'
  import FpsSelector from './FpsSelector.svelte'
  import SoundSelector from './SoundSelector.svelte'
  import { commitDraft, draftConfig, isDirty, loadConfig, saveState, savedConfig } from './stores'

  let runtimeStatus = $state<RuntimeStatus>('paused')
  let sessionAlerts = $state(0)
  let previewSrc = $state<string | null>(null)
  let previewWidth = $state(640)
  let previewHeight = $state(480)
  let previewFps = $state(0)
  let hasFace = $state(false)
  let hasHands = $state(false)
  let handInZone = $state(false)
  let previewZone = $state<PreviewZonePayload | null>(null)
  let fingertips = $state<PreviewPointPayload[]>([])
  let statusDetail = $state('Camera feed will appear here when tracking starts.')

  onMount(() => {
    let cleanup = () => {}

    void (async () => {
      const loaded = await loadConfigFromBackend()
      loadConfig(loaded)

      cleanup = await attachRuntimeListeners({
        onAppState: (payload) => {
          runtimeStatus = payload.status
          sessionAlerts = payload.session_alerts
        },
        onSidecarEvent: (payload) => {
          if (payload.state) {
            runtimeStatus = payload.state
          }

          if (typeof payload.fps === 'number' && payload.event !== 'preview') {
            previewFps = payload.fps
          }

          if (payload.event === 'preview' && payload.jpeg) {
            previewSrc = payload.jpeg
            previewWidth = payload.width ?? previewWidth
            previewHeight = payload.height ?? previewHeight
            previewFps = payload.fps ?? previewFps
            hasFace = payload.has_face ?? false
            hasHands = payload.has_hands ?? false
            handInZone = payload.hand_in_zone ?? false
            previewZone = payload.zone ?? null
            fingertips = payload.fingertips ?? []
            statusDetail = handInZone
              ? 'Hand contact detected inside the face zone.'
              : hasFace
                ? 'Face zone locked. Monitoring fingertip proximity.'
                : 'Searching for a face to establish the active zone.'
          }

          if (payload.event === 'camera_lost') {
            statusDetail =
              payload.reason === 'conflict'
                ? `Camera is busy in ${payload.process ?? 'another app'}.`
                : 'Camera feed is unavailable. Waiting for recovery.'
          }

          if (payload.event === 'camera_recovered') {
            statusDetail = 'Camera recovered. Monitoring resumed.'
          }

          if (payload.event === 'error' && payload.message) {
            statusDetail = payload.message
          }
        },
      })
    })()

    return () => cleanup()
  })

  let statusTone = $derived(handInZone ? 'warning' : runtimeStatus)
  let statusHeadline = $derived(
    runtimeStatus === 'active'
      ? handInZone
        ? 'Touch detected'
        : 'Watching live'
      : runtimeStatus === 'paused'
        ? 'Monitoring paused'
        : 'Needs attention',
  )
  let displayZone = $derived(adjustPreviewZone(previewZone, $savedConfig.zone, $draftConfig.zone))

  function updateSound(sound: string): void {
    draftConfig.update((value) => ({
      ...value,
      alert: {
        ...value.alert,
        sound,
      },
    }))
  }

  function updateDelay(delaySec: number): void {
    const normalized = Math.round(Math.max(0, Math.min(10, delaySec)) * 10) / 10
    draftConfig.update((value) => ({
      ...value,
      alert: {
        ...value.alert,
        delaySec: normalized,
      },
    }))
  }

  function updateConfidenceValue(confidenceValue: number): void {
    const normalized = Math.round(Math.max(0, Math.min(100, confidenceValue)))

    draftConfig.update((value) => ({
      ...value,
      alert: {
        ...value.alert,
        confidenceValue: normalized,
        confidence: confidencePresetForValue(normalized),
      },
    }))
  }

  function updateCameraFps(fps: number): void {
    draftConfig.update((value) => ({
      ...value,
      camera: {
        ...value.camera,
        fps,
      },
    }))
  }

  function updateZoneOffset(
    key: keyof typeof DEFAULT_CONFIG.zone,
    rawValue: number,
  ): void {
    const clamped = Math.round(Math.max(ZONE_OFFSET_LIMITS.min, Math.min(ZONE_OFFSET_LIMITS.max, rawValue)))
    draftConfig.update((value) => ({
      ...value,
      zone: {
        ...value.zone,
        [key]: clamped,
      },
    }))
  }

  function updateAppFlag<K extends keyof typeof DEFAULT_CONFIG.app>(key: K, value: boolean): void {
    draftConfig.update((current) => ({
      ...current,
      app: {
        ...current.app,
        [key]: value,
      },
    }))
  }

  async function testSound(): Promise<void> {
    saveState.set('idle')
    await testSoundInBackend($draftConfig.alert.sound)
  }

  async function addCustomSound(): Promise<void> {
    const path = await pickAudioFileInBackend()
    if (!path) {
      return
    }

    draftConfig.update((value) => ({
      ...value,
      alert: {
        ...value.alert,
        sound: path,
        customSoundPaths: Array.from(new Set([...value.alert.customSoundPaths, path])),
      },
    }))
  }

  async function save(): Promise<void> {
    saveState.set('saving')
    const saved = await saveConfigToBackend($draftConfig)
    loadConfig(saved)
    commitDraft()
    saveState.set('saved')
    setTimeout(() => {
      saveState.set('idle')
    }, 1600)
  }

  function pct(value: number): string {
    return `${Math.max(0, Math.min(100, value * 100))}%`
  }

  function clampNormalized(value: number): number {
    return Math.max(0, Math.min(1, value))
  }

  function adjustPreviewZone(
    zone: PreviewZonePayload | null,
    savedZone: typeof DEFAULT_CONFIG.zone,
    draftZone: typeof DEFAULT_CONFIG.zone,
  ): PreviewZonePayload | null {
    if (!zone) {
      return null
    }

    const horizontalScale = Math.max(
      0.1,
      1.1 + savedZone.leftOffsetPct / 100 + savedZone.rightOffsetPct / 100,
    )
    const verticalScale = Math.max(
      0.1,
      1.1 + savedZone.topOffsetPct / 100 + savedZone.bottomOffsetPct / 100,
    )
    const faceWidth = (zone.x_max - zone.x_min) / horizontalScale
    const faceHeight = (zone.y_max - zone.y_min) / verticalScale

    return {
      x_min: clampNormalized(zone.x_min - faceWidth * (draftZone.leftOffsetPct - savedZone.leftOffsetPct) / 100),
      x_max: clampNormalized(zone.x_max + faceWidth * (draftZone.rightOffsetPct - savedZone.rightOffsetPct) / 100),
      y_min: clampNormalized(zone.y_min - faceHeight * (draftZone.topOffsetPct - savedZone.topOffsetPct) / 100),
      y_max: clampNormalized(zone.y_max + faceHeight * (draftZone.bottomOffsetPct - savedZone.bottomOffsetPct) / 100),
    }
  }

  function zoneStyle(zone: PreviewZonePayload | null): string {
    if (!zone) {
      return ''
    }

    return [
      `left:${pct(zone.x_min)}`,
      `top:${pct(zone.y_min)}`,
      `width:${pct(zone.x_max - zone.x_min)}`,
      `height:${pct(zone.y_max - zone.y_min)}`,
    ].join(';')
  }

  function pointStyle(point: PreviewPointPayload): string {
    return `left:${pct(point.x)};top:${pct(point.y)};`
  }
</script>

<section class="workspace">
  <div class="bg-orb bg-orb-a"></div>
  <div class="bg-orb bg-orb-b"></div>

  <div class="frame">
    <header class="topbar">
      <div>
        <p class="eyebrow">Burgonet</p>
        <h1>Live monitoring and detection settings</h1>
      </div>

      <div class="topbar-pills">
        <div class="metric-pill">
          <span>Status</span>
          <strong>{runtimeStatus}</strong>
        </div>
        <div class="metric-pill">
          <span>Preview FPS</span>
          <strong>{previewFps}</strong>
        </div>
        <div class="metric-pill alert-pill">
          <span>Session alerts</span>
          <strong>{sessionAlerts}</strong>
        </div>
      </div>
    </header>

    <div class="layout">
      <section class="stage-card">
        <div class="stage-copy">
          <div>
            <p class="section-label">Camera preview</p>
            <h2>{statusHeadline}</h2>
          </div>
          <div class="runtime-pill" data-state={statusTone}>{runtimeStatus}</div>
        </div>

        <div class="stage-shell">
          <div class="stage">
            {#if previewSrc}
              <img
                class="preview-image"
                src={`data:image/jpeg;base64,${previewSrc}`}
                alt="Live camera preview"
              />
            {:else}
              <div class="preview-placeholder">
                <strong>Waiting for camera frames</strong>
                <span>Keep Burgonet active and the preview will appear here.</span>
              </div>
            {/if}

            <div class="stage-head">
              <div class="stage-meta">
                <span class:online={hasFace}>Face {hasFace ? 'locked' : 'searching'}</span>
                <span class:online={hasHands}>Hands {hasHands ? 'tracked' : 'idle'}</span>
                <span class:warning={handInZone}>Zone {handInZone ? 'breached' : 'clear'}</span>
              </div>
            </div>

            {#if displayZone}
              <div class="zone-box" style={zoneStyle(displayZone)}></div>
            {/if}

            {#each fingertips as point, index (index)}
              <span class:active={point.active} class="finger-dot" style={pointStyle(point)}></span>
            {/each}

            <div class="stage-overlay">
              <div class="status-banner" data-state={statusTone}>
                <strong>{statusHeadline}</strong>
                <span>{statusDetail}</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <article class="control-card">
        <div class="card-heading">
          <div>
            <p class="section-label">Control panel</p>
            <h2>Tune the alert behavior</h2>
          </div>
          <div class="sync-pill" class:dirty={$isDirty}>
            {#if $saveState === 'saved'}
              Saved
            {:else if $isDirty}
              Unsaved
            {:else}
              Synced
            {/if}
          </div>
        </div>

        <div class="sections">
          <section class="control-section">
            <p class="section-label">Alert sounds</p>
            <SoundSelector
              options={SOUND_OPTIONS}
              customPaths={$draftConfig.alert.customSoundPaths}
              selected={$draftConfig.alert.sound}
              onSelect={updateSound}
              onAddCustom={addCustomSound}
            />
          </section>

          <section class="control-section">
            <p class="section-label">Alert delay</p>
            <DelaySelector value={$draftConfig.alert.delaySec} onValueChange={updateDelay} />
          </section>

          <section class="control-section">
            <p class="section-label">Alert confidence</p>
            <ConfidenceSlider
              value={$draftConfig.alert.confidenceValue}
              onValueChange={updateConfidenceValue}
            />
          </section>

          <section class="control-section">
            <p class="section-label">Camera target FPS</p>
            <FpsSelector selected={$draftConfig.camera.fps} onSelect={updateCameraFps} />
          </section>

          <section class="control-section">
            <div class="section-header">
              <div>
                <p class="section-label">Zone tuning</p>
                <h3>Adjust each anchored edge</h3>
              </div>
            </div>

            <div class="zone-tuning">
              <label class="zone-input-row">
                <span>Left edge</span>
                <div class="zone-input-wrap">
                  <input
                    type="number"
                    min={ZONE_OFFSET_LIMITS.min}
                    max={ZONE_OFFSET_LIMITS.max}
                    step="1"
                    value={$draftConfig.zone.leftOffsetPct}
                    oninput={(event) => {
                      const nextValue = (event.currentTarget as HTMLInputElement).valueAsNumber
                      if (!Number.isNaN(nextValue)) {
                        updateZoneOffset('leftOffsetPct', nextValue)
                      }
                    }}
                  />
                  <span>%</span>
                </div>
              </label>

              <label class="zone-input-row">
                <span>Right edge</span>
                <div class="zone-input-wrap">
                  <input
                    type="number"
                    min={ZONE_OFFSET_LIMITS.min}
                    max={ZONE_OFFSET_LIMITS.max}
                    step="1"
                    value={$draftConfig.zone.rightOffsetPct}
                    oninput={(event) => {
                      const nextValue = (event.currentTarget as HTMLInputElement).valueAsNumber
                      if (!Number.isNaN(nextValue)) {
                        updateZoneOffset('rightOffsetPct', nextValue)
                      }
                    }}
                  />
                  <span>%</span>
                </div>
              </label>

              <label class="zone-input-row">
                <span>Top edge</span>
                <div class="zone-input-wrap">
                  <input
                    type="number"
                    min={ZONE_OFFSET_LIMITS.min}
                    max={ZONE_OFFSET_LIMITS.max}
                    step="1"
                    value={$draftConfig.zone.topOffsetPct}
                    oninput={(event) => {
                      const nextValue = (event.currentTarget as HTMLInputElement).valueAsNumber
                      if (!Number.isNaN(nextValue)) {
                        updateZoneOffset('topOffsetPct', nextValue)
                      }
                    }}
                  />
                  <span>%</span>
                </div>
              </label>

              <label class="zone-input-row">
                <span>Bottom edge</span>
                <div class="zone-input-wrap">
                  <input
                    type="number"
                    min={ZONE_OFFSET_LIMITS.min}
                    max={ZONE_OFFSET_LIMITS.max}
                    step="1"
                    value={$draftConfig.zone.bottomOffsetPct}
                    oninput={(event) => {
                      const nextValue = (event.currentTarget as HTMLInputElement).valueAsNumber
                      if (!Number.isNaN(nextValue)) {
                        updateZoneOffset('bottomOffsetPct', nextValue)
                      }
                    }}
                  />
                  <span>%</span>
                </div>
              </label>
            </div>
          </section>

          <section class="control-section">
            <div class="section-header">
              <div>
                <p class="section-label">App behavior</p>
                <h3>Desktop shell preferences</h3>
              </div>
            </div>

            <div class="toggle-list">
              <label class="toggle-row">
                <div>
                  <strong>Start on login</strong>
                  <span>Launch Burgonet automatically after sign-in and restore tray monitoring.</span>
                </div>
                <input
                  type="checkbox"
                  checked={$draftConfig.app.autostart}
                  onchange={(event) =>
                    updateAppFlag('autostart', (event.currentTarget as HTMLInputElement).checked)}
                />
              </label>

              <label class="toggle-row">
                <div>
                  <strong>Start minimized</strong>
                  <span>Keep the window hidden at launch and let the tray own the workflow.</span>
                </div>
                <input
                  type="checkbox"
                  checked={$draftConfig.app.startMinimized}
                  onchange={(event) =>
                    updateAppFlag(
                      'startMinimized',
                      (event.currentTarget as HTMLInputElement).checked,
                    )}
                />
              </label>

              <label class="toggle-row">
                <div>
                  <strong>Desktop notifications</strong>
                  <span>Show pause, resume, and recovery notices while Burgonet runs in the tray.</span>
                </div>
                <input
                  type="checkbox"
                  checked={$draftConfig.app.notificationsEnabled}
                  onchange={(event) =>
                    updateAppFlag(
                      'notificationsEnabled',
                      (event.currentTarget as HTMLInputElement).checked,
                    )}
                />
              </label>
            </div>
          </section>
        </div>

        <footer class="bottom-bar">
          <button class="secondary" type="button" onclick={testSound}>
            Test Sound
          </button>
          <button class="primary" type="button" disabled={!$isDirty} onclick={save}>
            {$saveState === 'saving' ? 'Saving…' : $saveState === 'saved' ? 'Saved!' : 'Save changes'}
          </button>
        </footer>
      </article>
    </div>
  </div>
</section>

<style>
  .workspace {
    position: relative;
    height: 100%;
    overflow: hidden;
    padding: clamp(10px, 1.3vw, 16px);
  }

  .bg-orb {
    position: absolute;
    border-radius: 999px;
    filter: blur(10px);
    pointer-events: none;
  }

  .bg-orb-a {
    width: 360px;
    height: 360px;
    top: -120px;
    right: -80px;
    background: rgba(159, 130, 211, 0.22);
  }

  .bg-orb-b {
    width: 280px;
    height: 280px;
    left: -70px;
    bottom: -90px;
    background: rgba(105, 177, 255, 0.18);
  }

  .frame {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 10px;
    height: 100%;
    min-height: 0;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: end;
    padding: 2px 2px 0;
  }

  .eyebrow,
  .section-label {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  h1,
  h2,
  h3,
  p {
    margin: 0;
  }

  h1 {
    margin-top: 2px;
    font-size: clamp(1.16rem, 1.5vw, 1.56rem);
    line-height: 1.1;
  }

  h2 {
    margin-top: 4px;
    font-size: 1.16rem;
    line-height: 1.1;
  }

  h3 {
    margin-top: 2px;
    font-size: 0.92rem;
  }

  .topbar-pills {
    display: grid;
    grid-template-columns: repeat(3, minmax(88px, 1fr));
    gap: 6px;
    min-width: min(100%, 320px);
  }

  .metric-pill {
    padding: 8px 11px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.76);
    border: 1px solid var(--fg-line);
    box-shadow: 0 6px 16px rgba(55, 29, 92, 0.08);
  }

  .metric-pill span {
    display: block;
    color: var(--fg-muted);
    font-size: 0.66rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .metric-pill strong {
    display: block;
    margin-top: 3px;
    font-size: 0.96rem;
  }

  .alert-pill strong {
    color: #c15158;
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.55fr) minmax(340px, 0.9fr);
    gap: 10px;
    min-height: 0;
    overflow: hidden;
  }

  .stage-card,
  .control-card {
    min-height: 0;
    border-radius: 20px;
    border: 1px solid rgba(133, 104, 181, 0.14);
    background: rgba(255, 255, 255, 0.72);
    box-shadow: var(--fg-shadow);
    backdrop-filter: blur(18px);
  }

  .stage-card {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 10px;
    padding: 14px;
    overflow: hidden;
  }

  .stage-copy,
  .card-heading,
  .section-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
  }

  .runtime-pill,
  .sync-pill {
    flex-shrink: 0;
    padding: 7px 10px;
    border-radius: 10px;
    font-size: 0.74rem;
    font-weight: 700;
    text-transform: capitalize;
    border: 1px solid transparent;
  }

  .runtime-pill[data-state='active'] {
    color: #0f8c60;
    background: rgba(15, 140, 96, 0.12);
  }

  .runtime-pill[data-state='paused'] {
    color: #9b6f0f;
    background: rgba(225, 182, 70, 0.18);
  }

  .runtime-pill[data-state='error'],
  .runtime-pill[data-state='warning'] {
    color: #b74e54;
    background: rgba(215, 95, 104, 0.14);
  }

  .sync-pill {
    color: var(--fg-muted);
    background: rgba(143, 112, 200, 0.1);
  }

  .sync-pill.dirty {
    color: #a05d00;
    background: rgba(237, 196, 85, 0.24);
  }

  .stage-shell {
    min-height: 0;
  }

  .stage-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .stage-meta span {
    padding: 6px 9px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid var(--fg-line);
    color: var(--fg-muted);
    font-size: 0.74rem;
    font-weight: 700;
  }

  .stage-meta span.online {
    color: #0f8c60;
    border-color: rgba(15, 140, 96, 0.2);
  }

  .stage-meta span.warning {
    color: #c25d18;
    border-color: rgba(194, 93, 24, 0.24);
  }

  .stage {
    position: relative;
    display: block;
    overflow: hidden;
    border-radius: 18px;
    border: 1px solid rgba(255, 255, 255, 0.65);
    background:
      radial-gradient(circle at top right, rgba(255, 255, 255, 0.24), transparent 28%),
      linear-gradient(180deg, rgba(20, 15, 30, 0.96), rgba(37, 28, 48, 0.9));
    height: 100%;
    min-height: clamp(300px, 48vh, 520px);
  }

  .preview-image,
  .preview-placeholder {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .preview-image {
    display: block;
    object-fit: cover;
    object-position: center;
    background: #e7e1ef;
  }

  .preview-placeholder {
    display: grid;
    place-content: center;
    gap: 6px;
    text-align: center;
    color: rgba(255, 255, 255, 0.8);
    padding: 24px;
  }

  .stage-head {
    position: absolute;
    inset: 12px 12px auto 12px;
    z-index: 2;
    display: flex;
    justify-content: flex-start;
    pointer-events: none;
  }

  .zone-box {
    position: absolute;
    z-index: 1;
    border: 2px solid rgba(255, 116, 64, 0.94);
    border-radius: 18px;
    box-shadow:
      0 0 0 2px rgba(255, 184, 136, 0.12),
      0 0 40px rgba(255, 119, 49, 0.18);
  }

  .finger-dot {
    position: absolute;
    z-index: 1;
    width: 12px;
    height: 12px;
    margin-left: -6px;
    margin-top: -6px;
    border-radius: 999px;
    border: 2px solid rgba(255, 255, 255, 0.9);
    background: rgba(255, 163, 119, 0.8);
    box-shadow: 0 0 16px rgba(255, 137, 73, 0.24);
  }

  .finger-dot.active {
    width: 16px;
    height: 16px;
    margin-left: -8px;
    margin-top: -8px;
    background: rgba(255, 100, 70, 0.96);
    box-shadow: 0 0 0 4px rgba(255, 100, 70, 0.18);
  }

  .stage-overlay {
    position: absolute;
    inset: auto 12px 12px 12px;
    display: flex;
    justify-content: flex-start;
    pointer-events: none;
  }

  .status-banner {
    max-width: min(92%, 520px);
    display: grid;
    gap: 3px;
    padding: 10px 12px;
    border-radius: 12px;
    color: white;
    background: rgba(20, 15, 30, 0.68);
    backdrop-filter: blur(14px);
    border: 1px solid rgba(255, 255, 255, 0.12);
  }

  .status-banner[data-state='active'] {
    background: rgba(14, 97, 70, 0.78);
  }

  .status-banner[data-state='paused'] {
    background: rgba(111, 81, 16, 0.82);
  }

  .status-banner[data-state='error'],
  .status-banner[data-state='warning'] {
    background: rgba(123, 48, 42, 0.84);
  }

  .status-banner strong {
    font-size: 0.88rem;
  }

  .status-banner span {
    color: rgba(255, 255, 255, 0.82);
    font-size: 0.8rem;
  }

  .control-card {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 10px;
    padding: 14px;
  }

  .sections {
    display: grid;
    gap: 8px;
    align-content: start;
    min-height: 0;
    overflow: auto;
    padding-right: 4px;
  }

  .control-section {
    display: grid;
    gap: 8px;
    padding: 11px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.72);
    border: 1px solid var(--fg-line);
  }

  .zone-tuning {
    display: grid;
    gap: 8px;
  }

  .zone-input-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
    padding: 10px 11px;
    border-radius: 10px;
    background: rgba(248, 244, 255, 0.9);
    border: 1px solid rgba(143, 112, 200, 0.12);
  }

  .zone-input-row span {
    font-size: 0.82rem;
  }

  .zone-input-wrap {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .zone-input-wrap span {
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }

  .zone-input-wrap input {
    width: 84px;
    min-height: 38px;
    border-radius: 9px;
    border: 1px solid var(--fg-line);
    background: rgba(255, 255, 255, 0.92);
    color: var(--fg-text);
    padding: 0 10px;
    font: inherit;
    text-align: right;
  }

  .toggle-list {
    display: grid;
    gap: 8px;
  }

  .toggle-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
    padding: 10px 11px;
    border-radius: 10px;
    background: rgba(248, 244, 255, 0.9);
    border: 1px solid rgba(143, 112, 200, 0.12);
  }

  .toggle-row span {
    display: block;
    margin-top: 4px;
    color: var(--fg-muted);
    font-size: 0.82rem;
  }

  input[type='checkbox'] {
    width: 20px;
    height: 20px;
    margin-top: 2px;
    accent-color: var(--fg-accent);
  }

  .bottom-bar {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .primary,
  .secondary {
    min-height: 40px;
    border-radius: 10px;
    font-weight: 800;
    transition:
      transform 140ms ease,
      opacity 140ms ease,
      box-shadow 140ms ease;
  }

  .primary {
    color: white;
    background: linear-gradient(135deg, #8f70c8 0%, #7661c8 100%);
    box-shadow: 0 14px 30px rgba(110, 87, 178, 0.24);
  }

  .primary:disabled {
    opacity: 0.56;
    cursor: default;
    box-shadow: none;
  }

  .secondary {
    color: var(--fg-text);
    background: rgba(255, 255, 255, 0.88);
    border: 1px solid var(--fg-line);
  }

  .primary:not(:disabled):hover,
  .secondary:hover {
    transform: translateY(-1px);
  }

  @media (max-width: 1120px) {
    .layout {
      grid-template-columns: 1fr;
      overflow: auto;
    }

    .frame {
      grid-template-rows: auto auto;
      height: auto;
      min-height: 100%;
    }
  }

  @media (max-width: 760px) {
    .workspace {
      padding: 10px;
      height: auto;
      min-height: 100%;
      overflow: auto;
    }

    .topbar,
    .stage-copy,
    .card-heading {
      grid-template-columns: 1fr;
      display: grid;
      align-items: start;
    }

    .topbar-pills {
      grid-template-columns: 1fr;
      min-width: 0;
    }

    .stage-card,
    .control-card {
      padding: 12px;
      border-radius: 16px;
    }

    .bottom-bar {
      grid-template-columns: 1fr;
    }
  }
</style>
