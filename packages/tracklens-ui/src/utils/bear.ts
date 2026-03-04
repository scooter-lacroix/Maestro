/**
 * TrackLens - Bear Notes Integration Utility
 *
 * Manages settings for auto-saving reviews to Bear.
 * Uses x-callback-url protocol - no vault detection needed.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { storage } from './storage';

const STORAGE_KEY_ENABLED = 'tracklens-bear-enabled';

/**
 * Bear integration settings
 */
export interface BearSettings {
  enabled: boolean;
}

/**
 * Get current Bear settings from storage
 */
export function getBearSettings(): BearSettings {
  return {
    enabled: storage.getItem(STORAGE_KEY_ENABLED) === 'true',
  };
}

/**
 * Save Bear settings to storage
 */
export function saveBearSettings(settings: BearSettings): void {
  storage.setItem(STORAGE_KEY_ENABLED, String(settings.enabled));
}
