/**
 * TrackLens UI - Mode Switcher Component
 *
 * Switches between Selection, Comment, and Redline annotation modes.
 * Removed: TaterSprite mascot, YouTube video.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useState } from 'react';
import type { EditorMode } from '../types';

interface ModeSwitcherProps {
  mode: EditorMode;
  onChange: (mode: EditorMode) => void;
}

export const ModeSwitcher: React.FC<ModeSwitcherProps> = ({ mode, onChange }) => {
  return (
    <div className="inline-flex items-center bg-background rounded-2xl p-1.5 shadow-neu-inset border-none gap-1">
      <ModeButton
        active={mode === 'selection'}
        onClick={() => onChange('selection')}
        icon={
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 20h-1a2 2 0 0 1-2-2 2 2 0 0 1-2 2H6"/>
            <path d="M13 8h7a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2h-7"/>
            <path d="M5 16H4a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2h1"/>
            <path d="M6 4h1a2 2 0 0 1 2 2 2 2 0 0 1 2-2h1"/>
            <path d="M9 6v12"/>
          </svg>
        }
        label="Selection"
      />
      <ModeButton
        active={mode === 'comment'}
        onClick={() => onChange('comment')}
        icon={
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
          </svg>
        }
        label="Comment"
      />
      <ModeButton
        active={mode === 'redline'}
        onClick={() => onChange('redline')}
        icon={
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 9.75L14.25 12m0 0l2.25 2.25M14.25 12l2.25-2.25M14.25 12L12 14.25m-2.58 4.92l-6.375-6.375a1.125 1.125 0 010-1.59L9.42 4.83c.211-.211.498-.33.796-.33H19.5a2.25 2.25 0 012.25 2.25v10.5a2.25 2.25 0 01-2.25 2.25h-9.284c-.298 0-.585-.119-.796-.33z" />
          </svg>
        }
        label="Redline"
        destructive
      />
    </div>
  );
};

const ModeButton: React.FC<{
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  destructive?: boolean;
}> = ({ active, onClick, icon, label, destructive }) => (
    <button
    onClick={onClick}
    className={`flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium transition-all duration-300 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
      active
        ? destructive
          ? 'bg-background text-destructive shadow-neu-small'
          : 'bg-background text-primary shadow-neu-small'
        : 'text-muted-foreground hover:text-foreground hover:bg-muted/10'
    }`}
  >
    {icon}
    {label}
  </button>
);
