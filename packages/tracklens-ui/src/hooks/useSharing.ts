/**
 * TrackLens - Sharing Hook (Placeholder)
 *
 * This is a placeholder for the sharing hook.
 * Actual sharing functionality has been removed from TrackLens.
 *
 * REBRANDED: Plannotator → TrackLens
 * REMOVED: All sharing/paste/marketing features
 */

import { useState, useCallback } from 'react';

export interface UseSharingReturn {
  shareUrl: string;
  shareUrlSize: string;
  shortShareUrl: string;
  isGeneratingShortUrl: boolean;
  shortUrlError: string;
  onGenerateShortUrl: () => void;
}

export function useSharing(): UseSharingReturn {
  // Sharing features removed - return empty values
  const [shortShareUrl, setShortShareUrl] = useState('');
  const [isGeneratingShortUrl, setIsGeneratingShortUrl] = useState(false);
  const [shortUrlError, setShortUrlError] = useState('');

  const onGenerateShortUrl = useCallback(() => {
    // No-op - sharing removed
    console.warn('Sharing features removed from TrackLens');
  }, []);

  return {
    shareUrl: '',
    shareUrlSize: '0 B',
    shortShareUrl,
    isGeneratingShortUrl,
    shortUrlError,
    onGenerateShortUrl,
  };
}
