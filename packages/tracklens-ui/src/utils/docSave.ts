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
 * Consolidated: planSave.ts → docSave.ts (single canonical save API)
 * Storage keys: Uses planSave keys for backward compatibility
 * Default path: ~/.maestro/tracklens/reviews/ (matches planSave behavior)
 *
 * @packageDocumentation
 */

import { storage } from './storage';

const STORAGE_KEY_ENABLED = 'tracklens-save-enabled'; // Legacy planSave key
const STORAGE_KEY_PATH = 'tracklens-save-path'; // Legacy planSave key

export interface DocSaveSettings {
  enabled: boolean;
  customPath: string | null;
}

// Legacy type alias for backward compatibility
export type PlanSaveSettings = DocSaveSettings;

const DEFAULT_SETTINGS: DocSaveSettings = {
  enabled: true,
  customPath: null, // null means use default ~/.maestro/tracklens/reviews/
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
 * Legacy alias for backward compatibility
 */
export const getPlanSaveSettings = getDocSaveSettings;

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
 * Legacy alias for backward compatibility
 */
export const savePlanSaveSettings = saveDocSaveSettings;

/**
 * Get the effective save path (respects custom path or returns default)
 */
export function getEffectiveSavePath(): string {
  const settings = getDocSaveSettings();
  if (settings.customPath) {
    return settings.customPath;
  }
  // Default path: ~/.maestro/tracklens/reviews/
  // @ts-ignore - HOME is available in Node.js environment
  const home = typeof window === 'undefined' ? process.env.HOME : '';
  return `${home}/.maestro/tracklens/reviews/`;
}
