/**
 * TrackLens UI - Confirm Dialog Component
 *
 * Reusable confirmation dialog component for user confirmations.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import React from 'react';

export interface ConfirmDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm?: () => void;
  title: string;
  message: React.ReactNode;
  subMessage?: React.ReactNode;
  confirmText?: string;
  cancelText?: string;
  variant?: 'info' | 'warning';
  showCancel?: boolean;
}

export const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  subMessage,
  confirmText = 'Got it',
  cancelText = 'Cancel',
  variant = 'info',
  showCancel = false,
}) => {
  if (!isOpen) return null;

  const iconColors = {
    info: 'bg-accent/20 text-accent',
    warning: 'bg-warning/20 text-warning',
  };

  const buttonColors = {
    info: 'bg-primary text-primary-foreground hover:opacity-90',
    warning: 'bg-warning text-warning-foreground hover:opacity-90',
  };

  const icons = {
    info: (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
      </svg>
    ),
    warning: (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
    ),
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-xl p-4">
      <div className="bg-background rounded-[32px] w-full max-w-sm shadow-neu-extruded p-8 border border-border/10">
        <div className="flex items-center gap-3 mb-6">
          <div className={`w-10 h-10 rounded-full flex items-center justify-center ${iconColors[variant]}`}>
            {icons[variant]}
          </div>
          <h3 className="font-bold font-display text-lg">{title}</h3>
        </div>
        <div className="text-sm text-muted-foreground mb-2">
          {message}
        </div>
        {subMessage && (
          <div className="text-xs text-muted-foreground mb-6">
            {subMessage}
          </div>
        )}
        {!subMessage && <div className="mb-4" />}
        <div className="flex justify-end gap-3 mt-8">
          {showCancel && (
            <button
              onClick={onClose}
              className="px-5 py-2.5 rounded-xl text-sm font-medium bg-background text-muted-foreground shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300"
            >
              {cancelText}
            </button>
          )}
          <button
            onClick={() => {
              if (onConfirm) {
                onConfirm();
              } else {
                onClose();
              }
            }}
            className={`px-5 py-2.5 rounded-xl text-sm font-medium shadow-neu-extruded hover:-translate-y-px hover:shadow-neu-hover active:translate-y-[0.5px] active:shadow-neu-inset transition-all duration-300 ${buttonColors[variant]}`}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
};
