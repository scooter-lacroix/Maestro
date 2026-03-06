/**
 * TrackLens - Hook for URL-based State Sharing (Local Only)
 *
 * Handles:
 * - Loading shared state from URL hash on mount
 * - Generating shareable URLs (hash-based only)
 * - Tracking whether current session is from a shared link
 * - Importing annotations from teammate's share URL
 *
 * NOTE: External paste service removed - local hash-based sharing only.
 *
 * REBRANDED: Plannotator → TrackLens
 * REMOVED: Short URL / paste service integration
 *
 * @packageDocumentation
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Annotation, type ImageAttachment } from '../types';
import {
  type SharePayload,
  parseShareHash,
  generateShareUrl,
  decompress,
  fromShareable,
  parseShareableImages,
  formatUrlSize,
} from '../utils/sharing';

export interface ImportResult {
  success: boolean;
  count: number;
  planTitle: string;
  error?: string;
}

interface UseSharingResult {
  /** Whether the current session was loaded from a shared URL */
  isSharedSession: boolean;

  /** Whether we're currently loading from a shared URL */
  isLoadingShared: boolean;

  /** The current shareable URL (updates when annotations change) */
  shareUrl: string;

  /** Human-readable size of the share URL */
  shareUrlSize: string;

  /** Annotations loaded from share that need to be applied to DOM */
  pendingSharedAnnotations: Annotation[] | null;

  /** Global attachments loaded from share */
  sharedGlobalAttachments: ImageAttachment[] | null;

  /** Call after applying shared annotations to clear the pending state */
  clearPendingSharedAnnotations: () => void;

  /** Manually trigger share URL generation */
  refreshShareUrl: () => Promise<void>;

  /** Import annotations from a teammate's share URL */
  importFromShareUrl: (url: string) => Promise<ImportResult>;

  /** Error message when a shared URL failed to load on mount */
  shareLoadError: string;

  /** Clear the share load error */
  clearShareLoadError: () => void;
}


export function useSharing(
  markdown: string,
  annotations: Annotation[],
  globalAttachments: ImageAttachment[],
  setMarkdown: (m: string) => void,
  setAnnotations: React.Dispatch<React.SetStateAction<Annotation[]>>,
  setGlobalAttachments: React.Dispatch<React.SetStateAction<ImageAttachment[]>>,
  onSharedLoad?: () => void,
  shareBaseUrl?: string
): UseSharingResult {
  const [isSharedSession, setIsSharedSession] = useState(false);
  const [isLoadingShared, setIsLoadingShared] = useState(true);
  const [shareUrl, setShareUrl] = useState('');
  const [shareUrlSize, setShareUrlSize] = useState('');
  const [pendingSharedAnnotations, setPendingSharedAnnotations] = useState<Annotation[] | null>(null);
  const [sharedGlobalAttachments, setSharedGlobalAttachments] = useState<ImageAttachment[] | null>(null);
  const [shareLoadError, setShareLoadError] = useState('');

  const clearPendingSharedAnnotations = useCallback(() => {
    setPendingSharedAnnotations(null);
    setSharedGlobalAttachments(null);
  }, []);

  const clearShareLoadError = useCallback(() => setShareLoadError(''), []);

  // Load shared state from URL hash
  const loadFromHash = useCallback(async () => {
    try {
      const hash = window.location.hash.slice(1);
      const payload = await parseShareHash();

      if (payload) {
        // Set plan content
        setMarkdown(payload.p);

        // Convert shareable annotations to full annotations
        const restoredAnnotations = fromShareable(payload.a);
        setAnnotations(restoredAnnotations);

        // Restore global attachments if present
        if (payload.g?.length) {
          const parsed = parseShareableImages(payload.g) ?? [];
          setGlobalAttachments(parsed);
          setSharedGlobalAttachments(parsed);
        }

        // Store for later application to DOM
        setPendingSharedAnnotations(restoredAnnotations);

        setIsSharedSession(true);

        // Notify parent that we loaded from a share
        onSharedLoad?.();

        // Clear the hash from URL to prevent re-loading on refresh
        // but keep the state in memory
        window.history.replaceState(
          {},
          '',
          window.location.pathname
        );

        return true;
      }

      // Hash was present but failed to decompress (likely truncated by browser)
      if (hash) {
        setShareLoadError('Failed to load shared plan — the URL may have been truncated by your browser.');
      }
      return false;
    } catch (e) {
      console.error('Failed to load from share hash:', e);
      setShareLoadError('Failed to load shared plan — an unexpected error occurred.');
      return false;
    }
  }, [setMarkdown, setAnnotations, setGlobalAttachments, onSharedLoad]);

  // Load from hash on mount
  useEffect(() => {
    loadFromHash().finally(() => setIsLoadingShared(false));
  }, []); // Only run on mount

  // Listen for hash changes (when user pastes a new share URL)
  useEffect(() => {
    const handleHashChange = () => {
      if (window.location.hash.length > 1) {
        loadFromHash();
      }
    };

    window.addEventListener('hashchange', handleHashChange);
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, [loadFromHash]);

  // Generate share URL when markdown or annotations change
  const refreshShareUrl = useCallback(async () => {
    try {
      const url = await generateShareUrl(markdown, annotations, globalAttachments, shareBaseUrl);
      setShareUrl(url);
      setShareUrlSize(formatUrlSize(url));
    } catch (e) {
      console.error('Failed to generate share URL:', e);
      setShareUrl('');
      setShareUrlSize('');
    }
  }, [markdown, annotations, globalAttachments, shareBaseUrl]);

  // Auto-refresh share URL when dependencies change
  useEffect(() => {
    refreshShareUrl();
  }, [refreshShareUrl]);

  // Import annotations from a teammate's share URL (hash-based only)
  const importFromShareUrl = useCallback(async (url: string): Promise<ImportResult> => {
    try {
      // Hash-based URL only (short URL / paste service removed)
      const hashIndex = url.indexOf('#');
      if (hashIndex === -1) {
        return { success: false, count: 0, planTitle: '', error: 'Invalid share URL: no hash fragment found' };
      }
      const hash = url.slice(hashIndex + 1);
      if (!hash) {
        return { success: false, count: 0, planTitle: '', error: 'Invalid share URL: empty hash' };
      }

      const payload: SharePayload = await decompress(hash);

      // Extract plan title from embedded plan text
      const lines = (payload.p || '').trim().split('\n');
      const titleLine = lines.find(l => l.startsWith('#'));
      const planTitle = titleLine ? titleLine.replace(/^#+\s*/, '').trim() : 'Unknown Plan';

      // Convert to full annotations
      const importedAnnotations = fromShareable(payload.a);

      if (importedAnnotations.length === 0) {
        return { success: true, count: 0, planTitle, error: 'No annotations found in share link' };
      }

      // Estimate count from current closure (may be slightly stale, but
      // the actual merge below uses the latest state via functional updater)
      const estimatedNew = importedAnnotations.filter(imp =>
        !annotations.some(existing =>
          existing.originalText === imp.originalText &&
          existing.type === imp.type &&
          existing.text === imp.text
        )
      );

      if (estimatedNew.length > 0) {
        // Merge using functional updater to avoid stale closure
        setAnnotations(prev => {
          const newAnnotations = importedAnnotations.filter(imp =>
            !prev.some(existing =>
              existing.originalText === imp.originalText &&
              existing.type === imp.type &&
              existing.text === imp.text
            )
          );
          if (newAnnotations.length === 0) return prev;
          const merged = [...prev, ...newAnnotations];
          // Set ALL annotations as pending so DOM highlights include originals
          setPendingSharedAnnotations(merged);
          return merged;
        });

        // Handle global attachments (deduplicate by path)
        if (payload.g?.length) {
          const parsed = parseShareableImages(payload.g) ?? [];
          setGlobalAttachments(prev => {
            const existingPaths = new Set(prev.map(g => g.path));
            const newAttachments = parsed.filter(p => !existingPaths.has(p.path));
            return newAttachments.length > 0 ? [...prev, ...newAttachments] : prev;
          });
          setSharedGlobalAttachments(parsed);
        }
      }

      return { success: true, count: estimatedNew.length, planTitle };
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : 'Failed to decompress share URL';
      return { success: false, count: 0, planTitle: '', error: errorMessage };
    }
  }, [annotations, globalAttachments, setAnnotations, setGlobalAttachments]);

  return {
    isSharedSession,
    isLoadingShared,
    shareUrl,
    shareUrlSize,
    pendingSharedAnnotations,
    sharedGlobalAttachments,
    clearPendingSharedAnnotations,
    refreshShareUrl,
    importFromShareUrl,
    shareLoadError,
    clearShareLoadError,
  };
}
