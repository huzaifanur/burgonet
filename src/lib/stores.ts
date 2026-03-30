import { derived, get, writable } from 'svelte/store'
import { DEFAULT_CONFIG, type BurgonetConfig } from './config'

function cloneConfig(config: BurgonetConfig): BurgonetConfig {
  return JSON.parse(JSON.stringify(config)) as BurgonetConfig
}

export const savedConfig = writable<BurgonetConfig>(cloneConfig(DEFAULT_CONFIG))
export const draftConfig = writable<BurgonetConfig>(cloneConfig(DEFAULT_CONFIG))
export const saveState = writable<'idle' | 'saving' | 'saved'>('idle')

export const isDirty = derived([savedConfig, draftConfig], ([$savedConfig, $draftConfig]) => {
  return JSON.stringify($savedConfig) !== JSON.stringify($draftConfig)
})

export function loadConfig(config: BurgonetConfig): void {
  const cloned = cloneConfig(config)
  savedConfig.set(cloned)
  draftConfig.set(cloneConfig(cloned))
  saveState.set('idle')
}

export function commitDraft(): void {
  savedConfig.set(cloneConfig(get(draftConfig)))
}
