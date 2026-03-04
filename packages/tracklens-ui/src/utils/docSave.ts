/**
 * TrackLens Document Save Settings Utility
 *
 * Manages settings for automatic document saving after approval/denial.
 * Users can configure custom save path or disable saving entirely.
 *
 * Uses cookies (not localStorage) because each hook invocation runs on a
 * random port, and localStorage is scoped by origin including port.
 *
 * REBRANDED: Renamed from planSave.ts to docSave.ts
 * Storage keys changed from 'plannotator-save-' to 'tracklens-doc-save-'
 * Default path changed from ~/.plannotator/plans/ to ~/.maestro/tracklens/docs/
 */

import { storage } from './storage';

const STORAGE_KEY_ENABLED = 'tracklens-doc-save-enabled';
const STORAGE_KEY_PATH = 'tracklens-doc-save-path';

export interface DocSaveSettings {
  enabled: boolean;
  customPath: string | null;
}

const DEFAULT_SETTINGS: DocSaveSettings = {
  enabled: true,
  customPath: null, // null means use default ~/.maestro/tracklens/docs/
};

/**
 * Get current document save settings from storage
 */
export function getDocSaveSettings(): DocSaveSettings {
  const enabled = storage.getItem(STORAGE_KEY_ENABLED);
  const customPath = storage.getItem(STORAGE_KEY_PATH);

  return {
    enabled: enabled !== 'false', // default to true
    customPath: customPath || null,
  };
}

/**
 * Save document save settings to storage
 */
export function saveDocSaveSettings(settings: DocSaveSettings): void {
  storage.setItem(STORAGE_KEY_ENABLED, String(settings.enabled));
  if (settings.customPath) {
    storage.setItem(STORAGE_KEY_PATH, settings.customPath);
  } else {
    storage.removeItem(STORAGE_KEY_PATH);
  }
}

/**
 * Get the effective save path (respects custom path or returns default)
 */
export function getEffectiveSavePath(): string {
  const settings = getDocSaveSettings();
  if (settings.customPath) {
    return settings.customPath;
  }
  // Default path: ~/.maestro/tracklens/docs/
  // @ts-ignore - HOME is available in Node.js environment
  const home = typeof window === 'undefined' ? process.env.HOME : '';
  return `${home}/.maestro/tracklens/docs/`;
}
