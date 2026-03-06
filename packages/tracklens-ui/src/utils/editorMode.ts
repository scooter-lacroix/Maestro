/**
 * TrackLens - Editor Mode Settings Utility
 *
 * REBRANDED: Plannotator → TrackLens
 */

import { storage } from './storage';
import type { EditorMode } from '../types';

const STORAGE_KEY = 'tracklens-editor-mode';
const DEFAULT_MODE: EditorMode = 'selection';

export function getEditorMode(): EditorMode {
  const stored = storage.getItem(STORAGE_KEY);
  if (stored === 'selection' || stored === 'comment' || stored === 'redline') {
    return stored;
  }
  return DEFAULT_MODE;
}

export function saveEditorMode(mode: EditorMode): void {
  storage.setItem(STORAGE_KEY, mode);
}
