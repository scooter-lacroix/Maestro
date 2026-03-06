/**
 * TrackLens - Plan Diff Marketing Dialog Utility
 *
 * Manages whether the user has seen the plan diff marketing dialog.
 * Uses localStorage for persistence.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { storage } from './storage';

const STORAGE_KEY = 'tracklens-plan-diff-marketing-seen';

/**
 * Check if the plan diff marketing dialog should be shown
 * @returns true if the dialog needs to be shown (user hasn't seen it)
 */
export function needsPlanDiffMarketingDialog(): boolean {
  return storage.getItem(STORAGE_KEY) !== 'true';
}

/**
 * Mark the plan diff marketing dialog as seen
 * Call this after showing the dialog to prevent it from showing again
 */
export function markPlanDiffMarketingSeen(): void {
  storage.setItem(STORAGE_KEY, 'true');
}
