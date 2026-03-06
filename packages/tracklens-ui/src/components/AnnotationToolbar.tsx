/**
 * TrackLens UI - Annotation Toolbar Component
 *
 * Floating toolbar for annotation actions (comment, suggestion, deletion).
 * Appears near text selection.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import React from 'react';

interface AnnotationToolbarProps {
  position: { x: number; y: number };
  onComment: () => void;
  onSuggestion: () => void;
  onDeletion: () => void;
  onClose: () => void;
}

export const AnnotationToolbar: React.FC<AnnotationToolbarProps> = ({
  position,
  onComment,
  onSuggestion,
  onDeletion,
  onClose,
}) => {
  return (
    <div
      className="fixed z-50 bg-card border border-border rounded-lg shadow-lg flex items-center gap-1 p-1"
      style={{ left: position.x, top: position.y }}
    >
      <button
        onClick={onComment}
        className="p-1.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
        title="Add comment"
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
        </svg>
      </button>
      <button
        onClick={onSuggestion}
        className="p-1.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
        title="Add suggestion"
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
        </svg>
      </button>
      <button
        onClick={onDeletion}
        className="p-1.5 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors"
        title="Mark for deletion"
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      </button>
      <button
        onClick={onClose}
        className="p-1.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
        title="Close"
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  );
};
