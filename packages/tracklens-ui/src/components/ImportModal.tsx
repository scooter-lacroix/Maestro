/**
 * TrackLens UI - Import Modal Component
 *
 * Modal for importing reviews from share URLs.
 * Removed: External sharing URL references.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useState, useRef } from 'react';

export interface ImportResult {
  success: boolean;
  count: number;
  planTitle?: string;
  error?: string;
}

interface ImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onImport: (url: string) => Promise<ImportResult>;
}

export const ImportModal: React.FC<ImportModalProps> = ({
  isOpen,
  onClose,
  onImport,
}) => {
  const [url, setUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const autoCloseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  if (!isOpen) return null;

  const handleImport = async () => {
    if (!url.trim()) return;
    setLoading(true);
    setResult(null);
    const res = await onImport(url.trim());
    setResult(res);
    setLoading(false);
    if (res.success && res.count > 0) {
      autoCloseTimer.current = setTimeout(() => {
        autoCloseTimer.current = null;
        setUrl('');
        setResult(null);
        onClose();
      }, 1500);
    }
  };

  const handleClose = () => {
    if (autoCloseTimer.current) {
      clearTimeout(autoCloseTimer.current);
      autoCloseTimer.current = null;
    }
    setUrl('');
    setResult(null);
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !loading) {
      handleImport();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-xl p-4">
      <div
        className="bg-background rounded-[32px] w-full max-w-lg flex flex-col shadow-neu-extruded border border-border/10 overflow-hidden"
        onClick={e => e.stopPropagation()}
      >
        <div className="p-6 border-b border-border/50">
          <div className="flex justify-between items-center">
            <h3 className="font-bold font-display text-lg">Import Review</h3>
            <button
              onClick={handleClose}
              className="p-2 rounded-xl bg-background shadow-neu-extruded-small text-muted-foreground hover:text-foreground active:shadow-neu-inset transition-all"
            >
              <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        <div className="p-4 space-y-4">
          <div className="mt-2">
            <label className="block text-sm font-medium text-muted-foreground mb-3">
              Share Link
            </label>
            <input
              type="text"
              value={url}
              onChange={e => setUrl(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Paste share link..."
              className="w-full bg-background rounded-xl px-4 py-3 text-sm shadow-neu-inset focus:outline-none focus:ring-2 focus:ring-primary/50"
              disabled={loading}
              autoFocus
            />
          </div>

          <p className="text-xs text-muted-foreground">
            Paste a share link to import annotations into the current review.
          </p>

          {result && (
            <div className={`rounded-xl px-4 py-3 text-sm shadow-neu-inset-small mt-4 ${result.success && result.count > 0
              ? 'bg-green-500/10 text-green-600 dark:text-green-400'
              : result.success && result.count === 0
                ? 'bg-yellow-500/10 text-yellow-600 dark:text-yellow-400'
                : 'bg-destructive/10 text-destructive'
              }`}>
              {result.success && result.count > 0 && (
                <span>Imported {result.count} annotation{result.count !== 1 ? 's' : ''}{result.planTitle ? ` from "${result.planTitle}"` : ''}</span>
              )}
              {result.success && result.count === 0 && (
                <span>{result.error || 'No new annotations to import (all already exist)'}</span>
              )}
              {!result.success && (
                <span>{result.error || 'Failed to import'}</span>
              )}
            </div>
          )}
        </div>

        <div className="p-6 flex justify-end gap-4 mt-2">
          <button
            onClick={handleClose}
            className="px-5 py-2.5 rounded-xl text-sm font-medium bg-background text-foreground shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300"
          >
            Cancel
          </button>
          <button
            onClick={handleImport}
            disabled={!url.trim() || loading}
            className="px-5 py-2.5 rounded-xl text-sm font-medium bg-primary text-primary-foreground shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? 'Importing...' : 'Import'}
          </button>
        </div>
      </div>
    </div>
  );
};
