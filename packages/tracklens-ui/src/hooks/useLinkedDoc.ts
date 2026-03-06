/**
 * TrackLens - Linked Document Hook
 *
 * Manages same-view navigation to local .md files.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { useState, useCallback, useRef } from 'react';
import type { Annotation, ImageAttachment } from '../types';
import type { ViewerHandle } from '../components/Viewer';

export interface UseLinkedDocOptions {
  markdown: string;
  annotations: Annotation[];
  selectedAnnotationId: string | null;
  globalAttachments: ImageAttachment[];
  setMarkdown: (md: string) => void;
  setAnnotations: (anns: Annotation[]) => void;
  setSelectedAnnotationId: (id: string | null) => void;
  setGlobalAttachments: (att: ImageAttachment[]) => void;
  viewerRef: React.RefObject<ViewerHandle | null>;
  sidebar: { open: (tab: string) => void };
}

interface SavedPlanState {
  markdown: string;
  annotations: Annotation[];
  selectedAnnotationId: string | null;
  globalAttachments: ImageAttachment[];
}

export interface CachedDocState {
  annotations: Annotation[];
  globalAttachments: ImageAttachment[];
}

export interface UseLinkedDocReturn {
  isActive: boolean;
  filepath: string | null;
  error: string | null;
  isLoading: boolean;
  openDoc: (docPath: string, buildUrl?: (path: string) => string) => Promise<void>;
  back: () => void;
  linkedDoc: { filepath: string; onBack: () => void } | null;
}

const HIGHLIGHT_REAPPLY_DELAY = 100;

export function useLinkedDoc(options: UseLinkedDocOptions): UseLinkedDocReturn {
  const {
    markdown,
    annotations,
    selectedAnnotationId,
    globalAttachments,
    setMarkdown,
    setAnnotations,
    setSelectedAnnotationId,
    setGlobalAttachments,
    viewerRef,
    sidebar,
  } = options;

  const [linkedDoc, setLinkedDoc] = useState<{ filepath: string; onBack: () => void } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const savedPlanState = useRef<SavedPlanState | null>(null);
  const docCache = useRef<Map<string, CachedDocState>>(new Map());

  const defaultBuildUrl = useCallback(
    (path: string) => `/api/doc?path=${encodeURIComponent(path)}`,
    []
  );

  const openDoc = useCallback(
    async (docPath: string, buildUrl?: (path: string) => string) => {
      setIsLoading(true);
      setError(null);

      try {
        const url = (buildUrl ?? defaultBuildUrl)(docPath);
        const res = await fetch(url);
        const data = (await res.json()) as {
          markdown?: string;
          filepath?: string;
          error?: string;
        };

        if (!res.ok || data.error) {
          setError(data.error || 'Failed to load document');
          return;
        }

        viewerRef.current?.clearAllHighlights();

        if (!savedPlanState.current) {
          savedPlanState.current = {
            markdown,
            annotations: [...annotations],
            selectedAnnotationId,
            globalAttachments: [...globalAttachments],
          };
        } else if (linkedDoc) {
          docCache.current.set(linkedDoc.filepath, {
            annotations: [...annotations],
            globalAttachments: [...globalAttachments],
          });
        }

        const cached = docCache.current.get(data.filepath!);

        setMarkdown(data.markdown!);
        setAnnotations(cached?.annotations ?? []);
        setGlobalAttachments(cached?.globalAttachments ?? []);
        setSelectedAnnotationId(null);
        setLinkedDoc({ 
          filepath: data.filepath!, 
          onBack: () => {
            back();
            setLinkedDoc(null);
          }
        });
        sidebar.open('toc');

        if (cached?.annotations.length) {
          setTimeout(() => {
            viewerRef.current?.clearAllHighlights();
            viewerRef.current?.applySharedAnnotations(cached.annotations);
          }, HIGHLIGHT_REAPPLY_DELAY);
        }
      } catch {
        setError('Failed to connect to server');
      } finally {
        setIsLoading(false);
      }
    },
    [markdown, annotations, selectedAnnotationId, globalAttachments, linkedDoc, setMarkdown, setAnnotations, setSelectedAnnotationId, setGlobalAttachments, viewerRef, sidebar, defaultBuildUrl]
  );

  const back = useCallback(() => {
    if (!savedPlanState.current) return;

    viewerRef.current?.clearAllHighlights();

    if (linkedDoc) {
      docCache.current.set(linkedDoc.filepath, {
        annotations: [...annotations],
        globalAttachments: [...globalAttachments],
      });
    }

    const saved = savedPlanState.current;
    setMarkdown(saved.markdown);
    setAnnotations(saved.annotations);
    setGlobalAttachments(saved.globalAttachments);
    setSelectedAnnotationId(saved.selectedAnnotationId);
    setError(null);
    savedPlanState.current = null;

    if (saved.annotations.length) {
      setTimeout(() => {
        viewerRef.current?.clearAllHighlights();
        viewerRef.current?.applySharedAnnotations(saved.annotations);
      }, HIGHLIGHT_REAPPLY_DELAY);
    }
  }, [linkedDoc, annotations, globalAttachments, setMarkdown, setAnnotations, setSelectedAnnotationId, setGlobalAttachments, viewerRef]);

  return {
    isActive: linkedDoc !== null,
    filepath: linkedDoc?.filepath ?? null,
    error,
    isLoading,
    openDoc,
    back,
    linkedDoc,
  };
}
