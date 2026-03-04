/**
 * TrackLens - Storage Utility
 *
 * Uses localStorage for settings persistence.
 * Rebranded from Plannotator with updated storage keys.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

const ONE_YEAR_SECONDS = 60 * 60 * 24 * 365;

export function getItem(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch (e) {
    return null;
  }
}

export function setItem(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch (e) {
    // localStorage not available
  }
}

export function removeItem(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch (e) {
    // localStorage not available
  }
}

const AUTO_CLOSE_KEY = 'tracklens-auto-close';

export type AutoCloseDelay = 'off' | '0' | '3' | '5';

export const AUTO_CLOSE_OPTIONS: { value: AutoCloseDelay; label: string; description: string }[] = [
  { value: 'off', label: 'Off', description: 'Tab stays open after submitting' },
  { value: '0', label: 'Immediately', description: 'Tab closes immediately after submitting' },
  { value: '3', label: 'After 3 seconds', description: 'Tab closes 3 seconds after submitting' },
  { value: '5', label: 'After 5 seconds', description: 'Tab closes 5 seconds after submitting' },
];

export function getAutoCloseDelay(): AutoCloseDelay {
  const val = getItem(AUTO_CLOSE_KEY);
  if (val === '0' || val === '3' || val === '5') return val;
  if (val === 'true') return '0';
  return 'off';
}

export function setAutoCloseDelay(delay: AutoCloseDelay): void {
  setItem(AUTO_CLOSE_KEY, delay);
}

export const storage = {
  getItem,
  setItem,
  removeItem,
};
