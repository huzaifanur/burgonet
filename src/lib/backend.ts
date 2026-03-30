import {
  DEFAULT_CONFIG,
  fromBackendConfig,
  toBackendConfig,
  type BackendBurgonetConfig,
  type BurgonetConfig,
} from './config'

export type RuntimeStatus = 'active' | 'paused' | 'error'

export interface AppStatePayload {
  status: RuntimeStatus
  session_alerts: number
}

export interface PreviewZonePayload {
  x_min: number
  y_min: number
  x_max: number
  y_max: number
}

export interface PreviewPointPayload {
  x: number
  y: number
  active: boolean
}

export interface SidecarEventPayload {
  event: string
  state?: RuntimeStatus
  fps?: number
  process?: string
  reason?: string
  message?: string
  timestamp?: string
  sound?: string
  jpeg?: string
  width?: number
  height?: number
  has_face?: boolean
  has_hands?: boolean
  hand_in_zone?: boolean
  zone?: PreviewZonePayload
  fingertips?: PreviewPointPayload[]
}

async function tryInvoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke<T>(command, payload)
  } catch {
    throw new Error(`Tauri invoke failed for ${command}`)
  }
}

export async function loadConfigFromBackend(): Promise<BurgonetConfig> {
  try {
    const config = await tryInvoke<BackendBurgonetConfig>('get_config')
    return fromBackendConfig(config)
  } catch {
    return structuredClone(DEFAULT_CONFIG)
  }
}

export async function saveConfigToBackend(config: BurgonetConfig): Promise<BurgonetConfig> {
  const backendConfig = toBackendConfig(config)
  const saved = await tryInvoke<BackendBurgonetConfig>('save_config', { config: backendConfig })
  return fromBackendConfig(saved)
}

export async function testSoundInBackend(sound: string): Promise<void> {
  try {
    await tryInvoke<void>('test_sound', { sound })
  } catch {
    // No-op in plain browser mode.
  }
}

export async function pickAudioFileInBackend(): Promise<string | null> {
  try {
    return await tryInvoke<string | null>('pick_audio_file')
  } catch {
    return null
  }
}

export async function attachRuntimeListeners(handlers: {
  onAppState?: (payload: AppStatePayload) => void
  onSidecarEvent?: (payload: SidecarEventPayload) => void
}): Promise<() => void> {
  try {
    const { listen } = await import('@tauri-apps/api/event')

    const unlistenAppState = await listen<AppStatePayload>('app-state', (event) => {
      handlers.onAppState?.(event.payload)
    })

    const unlistenSidecar = await listen<SidecarEventPayload>('sidecar-event', async (event) => {
      const payload = event.payload
      handlers.onSidecarEvent?.(payload)
    })

    return () => {
      unlistenAppState()
      unlistenSidecar()
    }
  } catch {
    return () => {}
  }
}
