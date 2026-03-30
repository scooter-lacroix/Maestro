/**
 * TrackLens UI - Viewer Component
 *
 * Main markdown viewer with annotation support.
 * Simplified from Plannotator's 800+ line implementation.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import React, { useRef, useState, useEffect, forwardRef, useImperativeHandle } from 'react';
import Highlighter from '@maestro/tracklens-web-highlighter';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';
import type { Block, Annotation, AnnotationType, ImageAttachment, EditorMode } from '../types';
import type { Frontmatter } from '../utils/parser';
import { AttachmentsButton } from './AttachmentsButton';
import { MermaidBlock } from './MermaidBlock';
import { getIdentity } from '../utils/identity';
import { TableOfContents } from './TableOfContents';
import { ModeSwitcher } from './ModeSwitcher';

interface ViewerProps {
  blocks: Block[];
  markdown: string;
  frontmatter?: Frontmatter | null;
  annotations: Annotation[];
  onAddAnnotation: (ann: Annotation) => void;
  onSelectAnnotation: (id: string | null) => void;
  selectedAnnotationId: string | null;
  mode: EditorMode;
  onModeChange: (mode: EditorMode) => void;
  globalAttachments?: ImageAttachment[];
  onAddGlobalAttachment?: (image: ImageAttachment) => void;
  onRemoveGlobalAttachment?: (path: string) => void;
  stickyActions?: boolean;
  onOpenLinkedDoc?: (path: string) => void;
  linkedDocInfo?: { filepath: string; onBack: () => void } | null;
  showToc?: boolean;
  tocActiveId?: string | null;
  onTocNavigate?: (blockId: string) => void;
}

export interface ViewerHandle {
  removeHighlight: (id: string) => void;
  clearAllHighlights: () => void;
  applySharedAnnotations: (annotations: Annotation[]) => void;
}

const FrontmatterCard: React.FC<{ frontmatter: Frontmatter }> = ({ frontmatter }) => {
  return (
    <div className="mb-6 p-4 bg-muted/30 rounded-2xl border border-border">
      <h4 className="text-xs font-semibold text-muted-foreground mb-2">Frontmatter</h4>
      <pre className="text-xs font-mono text-muted-foreground overflow-x-auto">
        {JSON.stringify(frontmatter, null, 2)}
      </pre>
    </div>
  );
};

export const Viewer = forwardRef<ViewerHandle, ViewerProps>(({
  blocks,
  markdown,
  frontmatter,
  annotations,
  onAddAnnotation,
  onSelectAnnotation,
  selectedAnnotationId,
  mode,
  onModeChange,
  globalAttachments = [],
  onAddGlobalAttachment,
  onRemoveGlobalAttachment,
  stickyActions = false,
  onOpenLinkedDoc,
  linkedDocInfo,
  showToc = false,
  tocActiveId,
  onTocNavigate,
}, ref) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const highlighterRef = useRef<Highlighter | null>(null);
  const [showToolbar, setShowToolbar] = useState(false);
  const [toolbarPosition, setToolbarPosition] = useState({ x: 0, y: 0 });
  const [selectedText, setSelectedText] = useState('');
  const [selectedRange, setSelectedRange] = useState<{ startContainer: Node; startOffset: number; endContainer: Node; endOffset: number } | null>(null);

  useImperativeHandle(ref, () => ({
    removeHighlight: (id: string) => {
      highlighterRef.current?.removeHighlight(id);
    },
    clearAllHighlights: () => {
      highlighterRef.current?.clearAllHighlights();
    },
    applySharedAnnotations: (sharedAnnotations: Annotation[]) => {
      // Apply shared annotations to the document
      sharedAnnotations.forEach(ann => {
        const blockEl = containerRef.current?.querySelector(`[data-block-id="${ann.blockId}"]`);
        if (blockEl && ann.startMeta && ann.endMeta) {
          try {
            const range = document.createRange();
            const startNode = blockEl.childNodes[ann.startMeta.parentIndex];
            const endNode = blockEl.childNodes[ann.endMeta.parentIndex];
            if (startNode && endNode) {
              range.setStart(startNode, ann.startMeta.textOffset);
              range.setEnd(endNode, ann.endMeta.textOffset);
              highlighterRef.current?.highlight(range, {
                id: ann.id,
                text: ann.text,
                startMeta: ann.startMeta,
                endMeta: ann.endMeta,
              });
            }
          } catch (e) {
            console.warn('Failed to apply highlight:', e);
          }
        }
      });
    },
  }));

  useEffect(() => {
    if (containerRef.current) {
      highlighterRef.current = new Highlighter({ $root: containerRef.current });
    }
    return () => {
      highlighterRef.current?.clearAllHighlights();
    };
  }, []);

  useEffect(() => {
    // Apply all annotations on mount or change
    annotations.forEach(ann => {
      const blockEl = containerRef.current?.querySelector(`[data-block-id="${ann.blockId}"]`);
      if (blockEl) {
        // Simple approach: just highlight the block
        const span = document.createElement('span');
        span.id = `highlight-${ann.id}`;
        span.className = 'tracklens-highlight';
        span.style.backgroundColor = ann.type === 'DELETION' ? 'rgba(239, 68, 68, 0.2)' :
          ann.type === 'INSERTION' ? 'rgba(34, 197, 94, 0.2)' :
            ann.type === 'REPLACEMENT' ? 'rgba(59, 130, 246, 0.2)' :
              'rgba(250, 204, 21, 0.3)';
        span.style.borderBottom = ann.type === 'DELETION' ? '2px solid rgba(239, 68, 68, 0.8)' :
          ann.type === 'INSERTION' ? '2px solid rgba(34, 197, 94, 0.8)' :
            ann.type === 'REPLACEMENT' ? '2px solid rgba(59, 130, 246, 0.8)' :
              '2px solid rgba(250, 204, 21, 0.8)';
        span.style.padding = '2px 4px';
        span.style.borderRadius = '3px';
        // For now, we'll just wrap the block content
        // A more sophisticated implementation would use character offsets
      }
    });
  }, [annotations]);

  const handleSelection = () => {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;

    const range = selection.getRangeAt(0);
    const text = selection.toString();

    if (!text || !containerRef.current?.contains(range.commonAncestorContainer)) {
      setShowToolbar(false);
      return;
    }

    const rect = range.getBoundingClientRect();
    const toolbarPos = {
      x: rect.left + rect.width / 2 - 100,
      y: rect.top - 50 + window.scrollY,
    };

    setToolbarPosition(toolbarPos);
    setSelectedText(text);
    const capturedRange = {
      startContainer: range.startContainer,
      startOffset: range.startOffset,
      endContainer: range.endContainer,
      endOffset: range.endOffset,
    };
    setSelectedRange(capturedRange);

    if (mode === 'comment') {
      executeAddComment(capturedRange, text);
    } else if (mode === 'redline') {
      executeAddDeletion(capturedRange, text);
    } else {
      setShowToolbar(true);
    }
  };

  const executeAddComment = (range: any, selectedTxt: string) => {
    if (!range || !containerRef.current) return;

    const blockId = blocks.find(b => containerRef.current?.querySelector(`[data-block-id="${b.id}"]`)?.contains(range.startContainer))?.id || '';

    const newAnnotation: Annotation = {
      id: `ann-${Date.now()}`,
      blockId,
      startOffset: range.startOffset,
      endOffset: range.endOffset,
      type: 'COMMENT' as AnnotationType,
      text: '',
      originalText: selectedTxt,
      createdA: Date.now(),
      author: getIdentity(),
    };

    onAddAnnotation(newAnnotation);
    clearSelection();
  };

  const handleAddComment = () => {
    executeAddComment(selectedRange, selectedText);
  };

  const executeAddSuggestion = (range: any, selectedTxt: string) => {
    if (!range || !containerRef.current) return;

    const blockId = blocks.find(b => containerRef.current?.querySelector(`[data-block-id="${b.id}"]`)?.contains(range.startContainer))?.id || '';

    const suggestion = prompt('Enter your suggestion:');
    if (!suggestion) return;

    const newAnnotation: Annotation = {
      id: `ann-${Date.now()}`,
      blockId,
      startOffset: range.startOffset,
      endOffset: range.endOffset,
      type: 'REPLACEMENT' as AnnotationType,
      text: suggestion,
      originalText: selectedTxt,
      createdA: Date.now(),
      author: getIdentity(),
    };

    onAddAnnotation(newAnnotation);
    clearSelection();
  };

  const handleAddSuggestion = () => {
    executeAddSuggestion(selectedRange, selectedText);
  };

  const executeAddDeletion = (range: any, selectedTxt: string) => {
    if (!range || !containerRef.current) return;

    const blockId = blocks.find(b => containerRef.current?.querySelector(`[data-block-id="${b.id}"]`)?.contains(range.startContainer))?.id || '';

    const newAnnotation: Annotation = {
      id: `ann-${Date.now()}`,
      blockId,
      startOffset: range.startOffset,
      endOffset: range.endOffset,
      type: 'DELETION' as AnnotationType,
      text: '',
      originalText: selectedTxt,
      createdA: Date.now(),
      author: getIdentity(),
    };

    onAddAnnotation(newAnnotation);
    clearSelection();
  };

  const handleAddDeletion = () => {
    executeAddDeletion(selectedRange, selectedText);
  };

  const clearSelection = () => {
    setShowToolbar(false);
    setSelectedText('');
    setSelectedRange(null);
    window.getSelection()?.removeAllRanges();
  };

  const renderBlock = (block: Block): React.ReactNode => {
    const baseClass = 'mb-4';

    switch (block.type) {
      case 'heading':
        const level = block.level || 1;
        const Tag = `h${level}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';
        return (
          <Tag key={block.id} data-block-id={block.id} className={`${baseClass} font-bold font-display`}>
            {block.content}
          </Tag>
        );

      case 'code':
        if (block.language === 'mermaid') {
          return <MermaidBlock key={block.id} block={block} />;
        }
        const highlighted = hljs.highlight(block.content, { language: block.language || 'plaintext' }).value;
        return (
          <pre key={block.id} data-block-id={block.id} className={`${baseClass} bg-muted p-4 rounded-2xl overflow-x-auto`}>
            <code className="text-sm font-mono" dangerouslySetInnerHTML={{ __html: highlighted }} />
          </pre>
        );

      case 'blockquote':
        return (
          <blockquote key={block.id} data-block-id={block.id} className={`${baseClass} border-l-4 border-border pl-4 italic text-muted-foreground`}>
            {block.content}
          </blockquote>
        );

      case 'list-item':
        return (
          <li key={block.id} data-block-id={block.id} className="ml-6 list-disc">
            {block.content}
          </li>
        );

      case 'hr':
        return <hr key={block.id} className="my-6 border-border" />;

      case 'table':
        return (
          <div key={block.id} data-block-id={block.id} className={`${baseClass} overflow-x-auto`}>
            <table className="min-w-full border border-border">
              <tbody dangerouslySetInnerHTML={{ __html: block.content }} />
            </table>
          </div>
        );

      default: // paragraph
        return (
          <p key={block.id} data-block-id={block.id} className={baseClass}>
            {block.content}
          </p>
        );
    }
  };

  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Sidebar - TOC */}
      {showToc && onTocNavigate && (
        <div className="w-64 border-r border-border p-4 overflow-y-auto">
          <TableOfContents
            blocks={blocks}
            annotations={annotations}
            activeId={tocActiveId ?? null}
            onNavigate={onTocNavigate}
          />
        </div>
      )}

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto">
        <div
          ref={containerRef}
          className="max-w-4xl mx-auto p-8 lg:p-12 my-8 bg-card/10 backdrop-blur-md rounded-[32px] shadow-neu-inset-small border border-border/5"
          onMouseUp={handleSelection}
        >
          {/* Linked Doc Info */}
          {linkedDocInfo && (
            <div className="mb-4 p-4 bg-muted/50 rounded-2xl flex items-center justify-between border border-border/50">
              <div className="flex items-center gap-2">
                <svg className="w-4 h-4 text-muted-foreground" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                </svg>
                <span className="text-sm text-muted-foreground font-mono">{linkedDocInfo.filepath}</span>
              </div>
              <button
                onClick={linkedDocInfo.onBack}
                className="text-xs px-3 py-1 bg-background border border-border rounded hover:bg-muted transition-colors"
              >
                Back
              </button>
            </div>
          )}


          {/* Frontmatter */}
          {frontmatter && <FrontmatterCard frontmatter={frontmatter} />}

          {/* Blocks */}
          {blocks.map(renderBlock)}

          {/* Annotation Toolbar */}
          {showToolbar && (
            <div
              className="fixed z-50 bg-card border border-border rounded-2xl shadow-neu-hover flex items-center gap-1 p-1.5"
              style={{ left: toolbarPosition.x, top: toolbarPosition.y }}
            >
              <button
                onClick={handleAddComment}
                className="p-1.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
                title="Add comment"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
                </svg>
              </button>
              <button
                onClick={handleAddSuggestion}
                className="p-1.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
                title="Add suggestion"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                </svg>
              </button>
              <button
                onClick={handleAddDeletion}
                className="p-1.5 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors"
                title="Mark for deletion"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
              <button
                onClick={clearSelection}
                className="p-1.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
                title="Close"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          )}

        </div>

        {/* Attachments Button - Moved outside backdrop-blur container to fix positioning */}
        {onAddGlobalAttachment && onRemoveGlobalAttachment && (
          <div className="fixed bottom-6 right-6 z-[100] pointer-events-auto">
            <AttachmentsButton
              images={globalAttachments}
              onAdd={onAddGlobalAttachment}
              onRemove={onRemoveGlobalAttachment}
              variant="toolbar"
            />
          </div>
        )}
      </div>
    </div>
  );
});

Viewer.displayName = 'Viewer';
