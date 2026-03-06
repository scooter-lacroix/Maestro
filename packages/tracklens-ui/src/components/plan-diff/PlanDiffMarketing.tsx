/**
 * TrackLens UI - Plan Diff Marketing
 *
 * Marketing modal for Plan Diff feature.
 * Removed: External video URLs, plannotator.ai references.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useState } from 'react';
import { createPortal } from 'react-dom';

interface PlanDiffMarketingProps {
  isOpen: boolean;
  onComplete: () => void;
}

export const PlanDiffMarketing: React.FC<PlanDiffMarketingProps> = ({
  isOpen,
  onComplete,
}) => {
  if (!isOpen) return null;

  const handleDismiss = () => {
    onComplete();
  };

  return createPortal(
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-background/90 backdrop-blur-sm p-4">
      <div className="bg-card border border-border rounded-xl w-full max-w-2xl shadow-2xl max-h-full flex flex-col">
        <div className="p-5 border-b border-border">
          <div className="flex items-center gap-2 mb-2">
            <div className="p-1.5 rounded-lg bg-primary/15">
              <svg className="w-5 h-5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
              </svg>
            </div>
            <h3 className="font-semibold text-base">Plan Diff Mode</h3>
          </div>
          <p className="text-sm text-muted-foreground">
            See exactly what changed when a coding agent revises your plan.
          </p>
        </div>

        <div className="p-4 space-y-4 overflow-y-auto min-h-0">
          <div className="space-y-3 text-sm text-foreground/90">
            <div className="flex gap-2.5">
              <div className="shrink-0 mt-0.5">
                <div className="w-1.5 h-1.5 rounded-full bg-success mt-1.5" />
              </div>
              <p>
                <span className="font-medium">Two view modes</span>{' '}
                <span className="text-muted-foreground">— a rendered visual diff with color-coded borders for quick scanning, and a raw markdown diff for precision.</span>
              </p>
            </div>
            <div className="flex gap-2.5">
              <div className="shrink-0 mt-0.5">
                <div className="w-1.5 h-1.5 rounded-full bg-primary mt-1.5" />
              </div>
              <p>
                <span className="font-medium">Version history</span>{' '}
                <span className="text-muted-foreground">— compare against any previous version from the sidebar. Plans are automatically versioned as your agent iterates.</span>
              </p>
            </div>
          </div>
        </div>

        <div className="p-4 border-t border-border flex justify-end">
          <button
            onClick={handleDismiss}
            className="px-4 py-2 bg-primary text-primary-foreground rounded-lg text-sm font-medium hover:opacity-90 transition-opacity"
          >
            Got it
          </button>
        </div>
      </div>
    </div>,
    document.body
  );
};
