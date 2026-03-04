/**
 * TrackLens UI Preferences Settings
 *
 * Manages UI preference settings for Table of Contents, sticky actions,
 * and first-run feature setup completion.
 *
 * Uses cookies (not localStorage) because each hook invocation runs on a
 * random port, and localStorage is scoped by origin including port.
 *
 * REBRANDED: Storage keys changed from 'plannotator-' to 'tracklens-'
 */

import { storage } from './storage';

const STORAGE_KEY_TOC = 'tracklens-toc-enabled';
const STORAGE_KEY_STICKY_ACTIONS = 'tracklens-sticky-actions-enabled';
const STORAGE_KEY_UI_FEATURES_CONFIGURED = 'tracklens-ui-features-configured';

export interface UIPreferences {
  tocEnabled: boolean;
  stickyActionsEnabled: boolean;
}

export function getUIPreferences(): UIPreferences {
  return {
    tocEnabled: storage.getItem(STORAGE_KEY_TOC) !== 'false',
    stickyActionsEnabled: storage.getItem(STORAGE_KEY_STICKY_ACTIONS) !== 'false',
  };
}

export function saveUIPreferences(prefs: UIPreferences): void {
  storage.setItem(STORAGE_KEY_TOC, String(prefs.tocEnabled));
  storage.setItem(STORAGE_KEY_STICKY_ACTIONS, String(prefs.stickyActionsEnabled));
}

export function needsUIFeaturesSetup(): boolean {
  return storage.getItem(STORAGE_KEY_UI_FEATURES_CONFIGURED) !== 'true';
}

export function markUIFeaturesSetupDone(): void {
  storage.setItem(STORAGE_KEY_UI_FEATURES_CONFIGURED, 'true');
}
