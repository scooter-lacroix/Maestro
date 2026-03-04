/**
 * TrackLens UI - Annotation Panel Component
 *
 * Displays list of annotations with editing and quick share.
 * Removed: Sharing features.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useState, useRef, useEffect } from 'react';
import type { Annotation, AnnotationType, Block } from '../types';

interface PanelProps {
  isOpen: boolean;
  annotations: Annotation[];
  blocks: Block[];
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  onEdit?: (id: string, updates: Partial<Annotation>) => void;
  selectedId: string | null;
  width?: number;
}

export const AnnotationPanel: React.FC<PanelProps> = ({
  isOpen,
  annotations,
  blocks,
  onSelect,
  onDelete,
  onEdit,
  selectedId,
  width,
}) => {
  const [copied, setCopied] = useState(false);
  const sortedAnnotations = [...annotations].sort((a, b) => a.createdA - b.createdA);

  if (!isOpen) return null;

  return (
    <aside className="border-l border-border/50 bg-card/30 backdrop-blur-sm flex flex-col flex-shrink-0" style={{ width: width ?? 288 }}>
      <div className="p-3 border-b border-border/50">
        <div className="flex items-center justify-between">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Annotations
          </h2>
          <span className="text-[10px] font-mono bg-muted px-1.5 py-0.5 rounded text-muted-foreground">
            {annotations.length}
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-1.5">
        {sortedAnnotations.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-40 text-center px-4">
            <div className="w-10 h-10 rounded-full bg-muted/50 flex items-center justify-center mb-3">
              <svg className="w-5 h-5 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
              </svg>
            </div>
            <p className="text-xs text-muted-foreground">
              Select text to add annotations
            </p>
          </div>
        ) : (
          sortedAnnotations.map(ann => (
            <AnnotationCard
              key={ann.id}
              annotation={ann}
              isSelected={selectedId === ann.id}
              onSelect={() => onSelect(ann.id)}
              onDelete={() => onDelete(ann.id)}
              onEdit={onEdit ? (updates: Partial<Annotation>) => onEdit(ann.id, updates) : undefined}
            />
          ))
        )}
      </div>
    </aside>
  );
};

function formatTimestamp(ts: number): string {
  const now = Date.now();
  const diff = now - ts;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (seconds < 60) return 'now';
  if (minutes < 60) return `${minutes}m`;
  if (hours < 24) return `${hours}h`;
  if (days < 7) return `${days}d`;

  return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

const AnnotationCard: React.FC<{
  annotation: Annotation;
  isSelected: boolean;
  onSelect: () => void;
  onDelete: () => void;
  onEdit?: (updates: Partial<Annotation>) => void;
}> = ({ annotation, isSelected, onSelect, onDelete, onEdit }) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editText, setEditText] = useState(annotation.text || '');
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (isEditing && textareaRef.current) {
      textareaRef.current.focus();
      textareaRef.current.select();
    }
  }, [isEditing]);

  useEffect(() => {
    if (!isEditing) {
      setEditText(annotation.text || '');
    }
  }, [annotation.text, isEditing]);

  const handleStartEdit = (e: React.MouseEvent) => {
    e.stopPropagation();
    setEditText(annotation.text || '');
    setIsEditing(true);
  };

  const handleSaveEdit = () => {
    if (onEdit) {
      onEdit({ text: editText });
    }
    setIsEditing(false);
  };

  const handleCancelEdit = () => {
    setEditText(annotation.text || '');
    setIsEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !e.nativeEvent.isComposing) {
      e.preventDefault();
      handleSaveEdit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancelEdit();
    }
  };

  const typeConfig: Record<AnnotationType, { label: string; color: string; bg: string; icon: React.ReactNode }> = {
    DELETION: {
      label: 'Delete',
      color: 'text-destructive',
      bg: 'bg-destructive/10',
      icon: <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
    },
    INSERTION: {
      label: 'Insert',
      color: 'text-secondary',
      bg: 'bg-secondary/10',
      icon: <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg>
    },
    REPLACEMENT: {
      label: 'Replace',
      color: 'text-primary',
      bg: 'bg-primary/10',
      icon: <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" /></svg>
    },
    COMMENT: {
      label: 'Comment',
      color: 'text-accent',
      bg: 'bg-accent/10',
      icon: <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" /></svg>
    },
    GLOBAL_COMMENT: {
      label: 'Global',
      color: 'text-muted-foreground',
      bg: 'bg-muted',
      icon: <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L4.5 21.75l1.5-1.5m0-6.75h-6" /></svg>
    },
  };

  const config = typeConfig[annotation.type];

  return (
    <div
      onClick={onSelect}
      className={`p-3 rounded-lg border cursor-pointer transition-all ${
        isSelected ? 'bg-accent/20 border-accent shadow-sm' : 'bg-card border-border hover:border-accent hover:shadow-sm'
      }`}
    >
      <div className="flex items-start justify-between mb-2">
        <div className={`flex items-center gap-1.5 ${config.color}`}>
          <span className={`${config.bg} p-1 rounded`}>{config.icon}</span>
          <span className="text-[10px] font-semibold uppercase">{config.label}</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="text-[10px] text-muted-foreground">{formatTimestamp(annotation.createdA)}</span>
          {onEdit && !isEditing && (
            <button
              onClick={handleStartEdit}
              className="p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
              title="Edit annotation"
            >
              <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
            </button>
          )}
          <button
            onClick={(e) => { e.stopPropagation(); onDelete(); }}
            className="p-1 rounded hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-colors"
            title="Remove annotation"
          >
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {annotation.type !== 'GLOBAL_COMMENT' && (
        <div className="mb-2 px-2 py-1 bg-muted/50 rounded text-xs font-mono text-muted-foreground truncate">
          {annotation.originalText}
        </div>
      )}

      {isEditing && onEdit ? (
        <textarea
          ref={textareaRef}
          value={editText}
          onChange={(e) => setEditText(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full px-2 py-1.5 text-sm bg-background border border-border rounded focus:outline-none focus:ring-1 focus:ring-primary resize-none"
          rows={3}
          onClick={(e) => e.stopPropagation()}
        />
      ) : annotation.text ? (
        <div className="text-sm text-foreground pl-2 border-l-2 border-accent">
          {annotation.text}
        </div>
      ) : null}

      {annotation.images && annotation.images.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1">
          {annotation.images.map((img) => (
            <div key={img.path} className="w-12 h-12 rounded bg-muted overflow-hidden">
              <img src={img.path} alt={img.name} className="w-full h-full object-cover" />
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
