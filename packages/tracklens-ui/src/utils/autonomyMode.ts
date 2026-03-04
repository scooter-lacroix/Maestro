/**
 * TrackLens Autonomy Mode Settings
 *
 * Manages the preferred autonomy mode to restore after document approval.
 * This is the merged version of Plannotator's permissionMode with Maestro Conductor's
 * autonomy levels, providing a unified interface across both systems.
 *
 * Available modes:
 * - full-auto: Auto-approve all decisions (bypassPermissions in Claude Code)
 * - semi-auto: Auto-approve file edits only (acceptEdits in Claude Code)
 * - checkpoint: Manually approve each decision (default in Claude Code)
 *
 * REBRANDED: Merged from permissionMode.ts + Maestro Conductor autonomy levels
 * Storage key changed from 'plannotator-permission-mode' to 'tracklens-autonomy-mode'
 */

import { storage } from './storage';

const STORAGE_KEY_MODE = 'tracklens-autonomy-mode';
const STORAGE_KEY_CONFIGURED = 'tracklens-autonomy-mode-configured';
const LEGACY_STORAGE_KEY = 'plannotator-permission-mode'; // For migration

export type AutonomyMode = 'full-auto' | 'semi-auto' | 'checkpoint';

/**
 * Mapping from legacy Plannotator permission modes to TrackLens autonomy modes
 */
const LEGACY_MAP: Record<string, AutonomyMode> = {
  bypassPermissions: 'full-auto',
  acceptEdits: 'semi-auto',
  default: 'checkpoint',
};

/**
 * Reverse mapping to Claude Code's PermissionRequest format
 */
const TO_CLAUDE_CODE_MAP: Record<AutonomyMode, string> = {
  'full-auto': 'bypassPermissions',
  'semi-auto': 'acceptEdits',
  'checkpoint': 'default',
};

export interface AutonomyModeSettings {
  mode: AutonomyMode;
  configured: boolean; // Whether user has explicitly set this
}

export const AUTONOMY_MODE_OPTIONS: { value: AutonomyMode; label: string; description: string }[] = [
  {
    value: 'semi-auto',
    label: 'Semi-Auto',
    description: 'Auto-approve file edits, ask for other tools',
  },
  {
    value: 'full-auto',
    label: 'Full Auto',
    description: 'Auto-approve all tool calls (equivalent to --dangerously-skip-permissions)',
  },
  {
    value: 'checkpoint',
    label: 'Checkpoint',
    description: 'Manually approve each tool call',
  },
];

const DEFAULT_MODE: AutonomyMode = 'checkpoint';

/**
 * Get current autonomy mode settings from storage
 * Handles migration from legacy Plannotator permission mode
 */
export function getAutonomyModeSettings(): AutonomyModeSettings {
  let mode = storage.getItem(STORAGE_KEY_MODE) as AutonomyMode | null;
  let configured = storage.getItem(STORAGE_KEY_CONFIGURED) === 'true';

  // Check legacy key for migration
  if (!mode && !configured) {
    const legacy = storage.getItem(LEGACY_STORAGE_KEY);
    if (legacy && LEGACY_MAP[legacy]) {
      const mapped = LEGACY_MAP[legacy];
      setAutonomyModeSettings(mapped);
      // Clean up legacy key
      storage.removeItem(LEGACY_STORAGE_KEY);
      return { mode: mapped, configured: false }; // Still needs proper configuration
    }
  }

  return {
    mode: mode || DEFAULT_MODE,
    configured,
  };
}

/**
 * Save autonomy mode settings to storage
 */
export function setAutonomyModeSettings(mode: AutonomyMode): void {
  storage.setItem(STORAGE_KEY_MODE, mode);
  storage.setItem(STORAGE_KEY_CONFIGURED, 'true');
}

/**
 * Check if the user needs to configure their autonomy mode preference
 */
export function needsAutonomyModeSetup(): boolean {
  return storage.getItem(STORAGE_KEY_CONFIGURED) !== 'true';
}

/**
 * Convert autonomy mode to Claude Code's PermissionRequest format
 */
export function modeToClaudeCodePermission(mode: AutonomyMode): string {
  return TO_CLAUDE_CODE_MAP[mode];
}

/**
 * Convert Claude Code permission to autonomy mode
 */
export function permissionFromClaudeCode(permission: string): AutonomyMode {
  if (permission === 'bypassPermissions') return 'full-auto';
  if (permission === 'acceptEdits') return 'semi-auto';
  return 'checkpoint';
}

/**
 * Validate if a string is a valid autonomy mode
 */
export function isValidAutonomyMode(mode: string): mode is AutonomyMode {
  return ['full-auto', 'semi-auto', 'checkpoint'].includes(mode);
}
