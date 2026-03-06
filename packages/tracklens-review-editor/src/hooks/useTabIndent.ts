/**
 * TrackLens Review Editor - Tab Indent Hook
 *
 * Custom hook that inserts two spaces on Tab press in a textarea.
 * Useful for code suggestion inputs.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { useCallback } from 'react';

/**
 * Returns an onKeyDown handler that inserts two spaces on Tab press in a textarea.
 * Pass the state setter for the textarea value.
 */
export function useTabIndent(setValue: (updater: (prev: string) => string) => void) {
  return useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key !== 'Tab') return;

    e.preventDefault();
    const target = e.target as HTMLTextAreaElement;
    const start = target.selectionStart;
    const end = target.selectionEnd;

    setValue((prev) => prev.substring(0, start) + '  ' + prev.substring(end));
    requestAnimationFrame(() => {
      target.selectionStart = target.selectionEnd = start + 2;
    });
  }, [setValue]);
}
