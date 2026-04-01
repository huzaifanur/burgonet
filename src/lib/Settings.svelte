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
    padding: clamp(12px, 1.4vw, 18px);
  }

  .bg-orb {
    position: absolute;
    border-radius: 999px;
    filter: blur(24px);
    pointer-events: none;
  }

  .bg-orb-a {
    width: 420px;
    height: 420px;
    top: -180px;
    right: -120px;
    background: rgba(255, 255, 255, 0.46);
  }

  .bg-orb-b {
    width: 340px;
    height: 340px;
    left: -120px;
    bottom: -120px;
    background: rgba(207, 222, 230, 0.34);
  }

  .frame {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 14px;
    height: 100%;
    min-height: 0;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: end;
    padding: 0 2px;
  }

  .eyebrow,
  .section-label {
    margin: 0;
    color: var(--fg-muted);
    font-size: 0.66rem;
    font-weight: 800;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  h1,
  h2,
  h3,
  p {
    margin: 0;
  }

  h1 {
    margin-top: 4px;
    max-width: 18ch;
    font-size: clamp(1.34rem, 1.8vw, 1.92rem);
    line-height: 1.04;
    letter-spacing: -0.04em;
  }

  h2 {
    margin-top: 4px;
    font-size: 1.22rem;
    line-height: 1.08;
    letter-spacing: -0.03em;
  }

  h3 {
    margin-top: 3px;
    font-size: 0.98rem;
    letter-spacing: -0.02em;
  }

  .topbar-pills {
    display: grid;
    grid-template-columns: repeat(3, minmax(88px, 1fr));
    gap: 8px;
    min-width: min(100%, 356px);
  }

  .metric-pill {
    padding: 10px 12px;
    border-radius: 14px;
    background: rgba(250, 252, 253, 0.86);
    border: 1px solid var(--fg-line);
    box-shadow: 0 8px 18px rgba(28, 42, 54, 0.06);
  }

  .metric-pill span {
    display: block;
    color: var(--fg-muted);
    font-size: 0.64rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }

  .metric-pill strong {
    display: block;
    margin-top: 6px;
    font-size: 1.08rem;
    line-height: 1;
  }

  .alert-pill strong {
    color: var(--fg-danger);
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.7fr) minmax(360px, 0.92fr);
    gap: 14px;
    min-height: 0;
    overflow: hidden;
  }

  .stage-card,
  .control-card {
    min-height: 0;
    border-radius: 22px;
    border: 1px solid rgba(28, 43, 56, 0.08);
    background: rgba(247, 249, 251, 0.78);
    box-shadow: var(--fg-shadow);
    backdrop-filter: blur(14px);
  }

  .stage-card {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 14px;
    padding: 18px;
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
    padding: 8px 12px;
    border-radius: 999px;
    font-size: 0.74rem;
    font-weight: 700;
    text-transform: capitalize;
    border: 1px solid rgba(0, 0, 0, 0.04);
  }

  .runtime-pill[data-state='active'] {
    color: var(--fg-success);
    background: rgba(24, 104, 79, 0.12);
  }

  .runtime-pill[data-state='paused'] {
    color: var(--fg-warning);
    background: rgba(141, 101, 25, 0.12);
  }

  .runtime-pill[data-state='error'],
  .runtime-pill[data-state='warning'] {
    color: var(--fg-danger);
    background: rgba(159, 67, 61, 0.12);
  }

  .sync-pill {
    color: var(--fg-muted);
    background: rgba(221, 227, 232, 0.84);
  }

  .sync-pill.dirty {
    color: var(--fg-warning);
    background: rgba(232, 221, 195, 0.9);
  }

  .stage-shell {
    min-height: 0;
  }

  .stage-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .stage-meta span {
    padding: 7px 10px;
    border-radius: 999px;
    background: rgba(245, 247, 249, 0.9);
    border: 1px solid var(--fg-line);
    color: var(--fg-muted);
    font-size: 0.74rem;
    font-weight: 700;
  }

  .stage-meta span.online {
    color: var(--fg-success);
    border-color: rgba(24, 104, 79, 0.18);
  }

  .stage-meta span.warning {
    color: var(--fg-warning);
    border-color: rgba(141, 101, 25, 0.2);
  }

  .stage {
    position: relative;
    display: block;
    overflow: hidden;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    background:
      radial-gradient(circle at top right, rgba(120, 136, 150, 0.16), transparent 26%),
      linear-gradient(180deg, rgba(28, 35, 43, 0.98), rgba(45, 55, 67, 0.96));
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
    background: #d5dde3;
  }

  .preview-placeholder {
    display: grid;
    place-content: center;
    gap: 8px;
    text-align: center;
    color: rgba(244, 247, 250, 0.82);
    padding: 32px;
  }

  .stage-head {
    position: absolute;
    inset: 16px 16px auto 16px;
    z-index: 2;
    display: flex;
    justify-content: flex-start;
    pointer-events: none;
  }

  .zone-box {
    position: absolute;
    z-index: 1;
    border: 2px solid rgba(246, 145, 79, 0.94);
    border-radius: 16px;
    box-shadow:
      0 0 0 2px rgba(246, 186, 138, 0.1),
      0 0 32px rgba(246, 145, 79, 0.16);
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
    background: rgba(255, 182, 126, 0.82);
    box-shadow: 0 0 14px rgba(246, 145, 79, 0.18);
  }

  .finger-dot.active {
    width: 16px;
    height: 16px;
    margin-left: -8px;
    margin-top: -8px;
    background: rgba(230, 103, 72, 0.96);
    box-shadow: 0 0 0 4px rgba(230, 103, 72, 0.18);
  }

  .stage-overlay {
    position: absolute;
    inset: auto 16px 16px 16px;
    display: flex;
    justify-content: flex-start;
    pointer-events: none;
  }

  .status-banner {
    max-width: min(92%, 520px);
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    border-radius: 16px;
    color: white;
    background: rgba(22, 29, 37, 0.78);
    backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .status-banner[data-state='active'] {
    background: rgba(19, 84, 66, 0.84);
  }

  .status-banner[data-state='paused'] {
    background: rgba(103, 77, 24, 0.84);
  }

  .status-banner[data-state='error'],
  .status-banner[data-state='warning'] {
    background: rgba(117, 46, 42, 0.86);
  }

  .status-banner strong {
    font-size: 0.94rem;
    line-height: 1.1;
  }

  .status-banner span {
    color: rgba(255, 255, 255, 0.78);
    font-size: 0.82rem;
  }

  .control-card {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 14px;
    padding: 18px;
  }

  .sections {
    display: grid;
    gap: 10px;
    align-content: start;
    min-height: 0;
    overflow: auto;
    padding-right: 6px;
  }

  .control-section {
    display: grid;
    gap: 10px;
    padding: 14px;
    border-radius: 16px;
    background: rgba(250, 252, 253, 0.9);
    border: 1px solid var(--fg-line);
  }

  .zone-tuning {
    display: grid;
    gap: 10px;
  }

  .zone-input-row {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    align-items: center;
    padding: 12px 13px;
    border-radius: 14px;
    background: var(--fg-panel);
    border: 1px solid rgba(35, 50, 63, 0.08);
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
    min-height: 42px;
    border-radius: 12px;
    border: 1px solid var(--fg-line);
    background: var(--fg-surface-strong);
    color: var(--fg-text);
    padding: 0 10px;
    font: inherit;
    text-align: right;
  }

  .toggle-list {
    display: grid;
    gap: 10px;
  }

  .toggle-row {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    align-items: start;
    padding: 14px;
    border-radius: 16px;
    background: var(--fg-panel);
    border: 1px solid rgba(35, 50, 63, 0.08);
  }

  .toggle-row span {
    display: block;
    margin-top: 6px;
    color: var(--fg-muted);
    font-size: 0.82rem;
  }

  input[type='checkbox'] {
    width: 22px;
    height: 22px;
    margin-top: 1px;
    accent-color: var(--fg-accent);
  }

  .bottom-bar {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .primary,
  .secondary {
    min-height: 44px;
    border-radius: 14px;
    font-weight: 800;
    transition:
      transform 140ms ease,
      opacity 140ms ease,
      box-shadow 140ms ease,
      background-color 140ms ease;
  }

  .primary {
    color: white;
    background: linear-gradient(135deg, #174c48 0%, #236b63 100%);
    box-shadow: 0 12px 24px rgba(28, 88, 81, 0.18);
  }

  .primary:disabled {
    opacity: 0.56;
    cursor: default;
    box-shadow: none;
  }

  .secondary {
    color: var(--fg-text);
    background: rgba(255, 255, 255, 0.72);
    border: 1px solid var(--fg-line);
  }

  .primary:not(:disabled):hover,
  .secondary:hover {
    transform: translateY(-1px);
  }

  .primary:not(:disabled):hover {
    box-shadow: 0 16px 26px rgba(28, 88, 81, 0.22);
  }

  @media (max-width: 1040px) {
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
