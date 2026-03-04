/**
 * TrackLens - Review Save Settings Utility
 *
 * Manages settings for automatic review saving after approval/denial.
 * Users can configure custom save path or disable saving entirely.
 *
 * Uses localStorage for persistence.
 *
 * REBRANDED: Plannotator → TrackLens
 * Updated: Function names planSave → docSave (for "document save")
 *
 * @packageDocumentation
 */

import { storage } from './storage';

const STORAGE_KEY_ENABLED = 'tracklens-save-enabled';
const STORAGE_KEY_PATH = 'tracklens-save-path';

export interface DocSaveSettings {
  enabled: boolean;
  customPath: string | null;
}

const DEFAULT_SETTINGS: DocSaveSettings = {
  enabled: true,
  customPath: null, // null means use default ~/.maestro/tracklens/reviews/
};

/**
 * Get current review save settings from storage
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
 * Legacy alias for compatibility
 */
export function getPlanSaveSettings() {
  return getDocSaveSettings();
}

/**
 * Type alias for compatibility
 */
export type PlanSaveSettings = DocSaveSettings;

/**
 * Save review save settings to storage
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
 * Legacy alias for compatibility
 */
export function savePlanSaveSettings(settings: DocSaveSettings): void {
  saveDocSaveSettings(settings);
}

