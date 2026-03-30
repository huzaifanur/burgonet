export type BuiltinAlertSound = 'none' | 'alarm' | 'beep' | 'vibrate' | 'whistle' | 'affirm'
export type ConfidencePreset = 'low' | 'medium' | 'high'

export interface BurgonetConfig {
  alert: {
    sound: string
    customSoundPaths: string[]
    delaySec: number
    confidence: ConfidencePreset
    confidenceValue: number
    volume: number
  }
  camera: {
    deviceIndex: number
    resolution: [number, number]
    fps: number
  }
  zone: {
    leftOffsetPct: number
    rightOffsetPct: number
    topOffsetPct: number
    bottomOffsetPct: number
  }
  cameraConflict: {
    mode: 'auto_resume'
    retryIntervalSec: number
    notifyOnPause: boolean
    notifyOnResume: boolean
  }
  app: {
    autostart: boolean
    startMinimized: boolean
    notificationsEnabled: boolean
  }
}

export interface BackendBurgonetConfig {
  alert: {
    sound: string
    custom_sound_paths: string[]
    delay_sec: number
    confidence: ConfidencePreset
    confidence_value: number
    volume: number
  }
  camera: {
    device_index: number
    resolution: [number, number]
    fps: number
  }
  zone: {
    left_offset_pct: number
    right_offset_pct: number
    top_offset_pct: number
    bottom_offset_pct: number
  }
  camera_conflict: {
    mode: 'auto_resume'
    retry_interval_sec: number
    notify_on_pause: boolean
    notify_on_resume: boolean
  }
  app: {
    autostart: boolean
    start_minimized: boolean
    notifications_enabled: boolean
  }
}

export const SOUND_OPTIONS: Array<{ value: BuiltinAlertSound; label: string }> = [
  { value: 'none', label: 'None' },
  { value: 'alarm', label: 'Alarm' },
  { value: 'beep', label: 'Beep' },
  { value: 'vibrate', label: 'Vibrate' },
  { value: 'whistle', label: 'Whistle' },
  { value: 'affirm', label: 'Affirm' },
]

export const CAMERA_FPS_OPTIONS = [15, 24, 30, 60] as const
export const ZONE_OFFSET_LIMITS = {
  min: -20,
  max: 40,
} as const

export const CONFIDENCE_PRESETS: Record<ConfidencePreset, number> = {
  low: 30,
  medium: 50,
  high: 70,
}

export const DEFAULT_CONFIG: BurgonetConfig = {
  alert: {
    sound: 'whistle',
    customSoundPaths: [],
    delaySec: 0,
    confidence: 'high',
    confidenceValue: 70,
    volume: 0.7,
  },
  camera: {
    deviceIndex: 0,
    resolution: [640, 480],
    fps: 30,
  },
  zone: {
    leftOffsetPct: 0,
    rightOffsetPct: 0,
    topOffsetPct: 0,
    bottomOffsetPct: 0,
  },
  cameraConflict: {
    mode: 'auto_resume',
    retryIntervalSec: 5,
    notifyOnPause: true,
    notifyOnResume: true,
  },
  app: {
    autostart: true,
    startMinimized: true,
    notificationsEnabled: true,
  },
}

export function presetForValue(value: number): ConfidencePreset | null {
  return (
    (Object.entries(CONFIDENCE_PRESETS).find(([, presetValue]) => presetValue === value)?.[0] as
      | ConfidencePreset
      | undefined) ?? null
  )
}

export function confidencePresetForValue(value: number): ConfidencePreset {
  if (value <= 40) {
    return 'low'
  }

  if (value <= 60) {
    return 'medium'
  }

  return 'high'
}

export function isBuiltinSound(value: string): value is BuiltinAlertSound {
  return SOUND_OPTIONS.some((option) => option.value === value)
}

function dedupeSoundPaths(paths: string[]): string[] {
  return Array.from(new Set(paths.filter((value) => value.trim().length > 0)))
}

export function soundLabel(value: string): string {
  const builtin = SOUND_OPTIONS.find((option) => option.value === value)
  if (builtin) {
    return builtin.label
  }

  const normalized = value.split(/[\\/]/).at(-1)?.trim()
  return normalized && normalized.length > 0 ? normalized : 'Custom audio'
}

export function fromBackendConfig(config: BackendBurgonetConfig): BurgonetConfig {
  const customSoundPaths = dedupeSoundPaths([
    ...(config.alert.custom_sound_paths ?? []),
    ...(isBuiltinSound(config.alert.sound) ? [] : [config.alert.sound]),
  ])

  return {
    alert: {
      sound: config.alert.sound,
      customSoundPaths,
      delaySec: config.alert.delay_sec,
      confidence: config.alert.confidence,
      confidenceValue: config.alert.confidence_value,
      volume: config.alert.volume,
    },
    camera: {
      deviceIndex: config.camera.device_index,
      resolution: config.camera.resolution,
      fps: config.camera.fps,
    },
    zone: {
      leftOffsetPct: config.zone.left_offset_pct,
      rightOffsetPct: config.zone.right_offset_pct,
      topOffsetPct: config.zone.top_offset_pct,
      bottomOffsetPct: config.zone.bottom_offset_pct,
    },
    cameraConflict: {
      mode: config.camera_conflict.mode,
      retryIntervalSec: config.camera_conflict.retry_interval_sec,
      notifyOnPause: config.camera_conflict.notify_on_pause,
      notifyOnResume: config.camera_conflict.notify_on_resume,
    },
    app: {
      autostart: config.app.autostart,
      startMinimized: config.app.start_minimized,
      notificationsEnabled: config.app.notifications_enabled,
    },
  }
}

export function toBackendConfig(config: BurgonetConfig): BackendBurgonetConfig {
  return {
    alert: {
      sound: config.alert.sound,
      custom_sound_paths: dedupeSoundPaths(config.alert.customSoundPaths),
      delay_sec: config.alert.delaySec,
      confidence: config.alert.confidence,
      confidence_value: config.alert.confidenceValue,
      volume: config.alert.volume,
    },
    camera: {
      device_index: config.camera.deviceIndex,
      resolution: config.camera.resolution,
      fps: config.camera.fps,
    },
    zone: {
      left_offset_pct: config.zone.leftOffsetPct,
      right_offset_pct: config.zone.rightOffsetPct,
      top_offset_pct: config.zone.topOffsetPct,
      bottom_offset_pct: config.zone.bottomOffsetPct,
    },
    camera_conflict: {
      mode: config.cameraConflict.mode,
      retry_interval_sec: config.cameraConflict.retryIntervalSec,
      notify_on_pause: config.cameraConflict.notifyOnPause,
      notify_on_resume: config.cameraConflict.notifyOnResume,
    },
    app: {
      autostart: config.app.autostart,
      start_minimized: config.app.startMinimized,
      notifications_enabled: config.app.notificationsEnabled,
    },
  }
}
