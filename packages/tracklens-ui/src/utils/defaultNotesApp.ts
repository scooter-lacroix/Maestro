/**
 * TrackLens - Default Notes App Preference
 *
 * Stores the user's preferred notes app for the Cmd/Ctrl+S shortcut.
 * Uses localStorage for persistence.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { storage } from './storage';

const STORAGE_KEY = 'tracklens-default-notes-app';

export type DefaultNotesApp = 'obsidian' | 'bear' | 'download' | 'ask';

export function getDefaultNotesApp(): DefaultNotesApp {
  return (storage.getItem(STORAGE_KEY) as DefaultNotesApp) || 'ask';
}

export function saveDefaultNotesApp(app: DefaultNotesApp): void {
  storage.setItem(STORAGE_KEY, app);
}
