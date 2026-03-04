/**
 * TrackLens - Autonomy Mode Settings Utility (Claude Code only)
 *
 * Manages the preferred autonomy mode to restore after review approval.
 * Claude Code 2.1.7+ supports updatedPermissions in hook responses.
 *
 * Available modes:
 * - bypassPermissions: Auto-approve all tool calls (full-auto)
 * - acceptEdits: Auto-approve file edits only (semi-auto)
 * - default: Manually approve each tool call (checkpoint)
 *
 * REBRANDED: Plannotator → TrackLens, Permission Mode → Autonomy Mode
 *
 * @packageDocumentation
 */

import { storage } from './storage';

const STORAGE_KEY_MODE = 'tracklens-autonomy-mode';
const STORAGE_KEY_CONFIGURED = 'tracklens-autonomy-mode-configured';

export type AutonomyMode = 'bypassPermissions' | 'acceptEdits' | 'default';

export interface AutonomyModeSettings {
  mode: AutonomyMode;
  configured: boolean; // Whether user has explicitly set this
}

export const AUTONOMY_MODE_OPTIONS: { value: AutonomyMode; label: string; description: string }[] = [
  {
    value: 'acceptEdits',
    label: 'Semi-Auto',
    description: 'Auto-approve file edits, ask for other tools',
  },
  {
    value: 'bypassPermissions',
    label: 'Full-Auto',
    description: 'Auto-approve all tool calls (equivalent to --dangerously-skip-permissions)',
  },
  {
    value: 'default',
    label: 'Checkpoint',
    description: 'Manually approve each tool call',
  },
];

const DEFAULT_MODE: AutonomyMode = 'acceptEdits';

/**
 * Get current autonomy mode settings from storage
 */
export function getAutonomyModeSettings(): AutonomyModeSettings {
  const mode = storage.getItem(STORAGE_KEY_MODE) as AutonomyMode | null;
  const configured = storage.getItem(STORAGE_KEY_CONFIGURED) === 'true';

  return {
    mode: mode || DEFAULT_MODE,
    configured,
  };
}

/**
 * Save autonomy mode settings to storage
 */
export function saveAutonomyModeSettings(mode: AutonomyMode): void {
  storage.setItem(STORAGE_KEY_MODE, mode);
  storage.setItem(STORAGE_KEY_CONFIGURED, 'true');
}

/**
 * Check if the user needs to configure their autonomy mode preference
 */
export function needsAutonomyModeSetup(): boolean {
  return storage.getItem(STORAGE_KEY_CONFIGURED) !== 'true';
}

// Legacy aliases for backward compatibility
export type PermissionMode = AutonomyMode;
export type PermissionModeSettings = AutonomyModeSettings;
export const PERMISSION_MODE_OPTIONS = AUTONOMY_MODE_OPTIONS;
export const getPermissionModeSettings = getAutonomyModeSettings;
export const savePermissionModeSettings = saveAutonomyModeSettings;
export const needsPermissionModeSetup = needsAutonomyModeSetup;
