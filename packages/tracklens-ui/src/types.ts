/**
 * TrackLens UI - Type Definitions
 *
 * Core types for annotations, blocks, code review, and vault browsing.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

export enum AnnotationType {
  DELETION = 'DELETION',
  INSERTION = 'INSERTION',
  REPLACEMENT = 'REPLACEMENT',
  COMMENT = 'COMMENT',
  GLOBAL_COMMENT = 'GLOBAL_COMMENT',
}

export type EditorMode = 'selection' | 'comment' | 'redline';

export interface ImageAttachment {
  path: string;
  name: string;
}

export interface Annotation {
  id: string;
  blockId: string;
  startOffset: number;
  endOffset: number;
  type: AnnotationType;
  text?: string;
  originalText: string;
  createdA: number;
  author?: string;
  images?: ImageAttachment[];
  startMeta?: {
    parentTagName: string;
    parentIndex: number;
    textOffset: number;
  };
  endMeta?: {
    parentTagName: string;
    parentIndex: number;
    textOffset: number;
  };
}

export interface Block {
  id: string;
  type: 'paragraph' | 'heading' | 'blockquote' | 'list-item' | 'code' | 'hr' | 'table';
  content: string;
  level?: number;
  language?: string;
  checked?: boolean;
  order: number;
  startLine: number;
}

export interface DiffResult {
  original: string;
  modified: string;
  diffText: string;
}

export type CodeAnnotationType = 'comment' | 'suggestion' | 'concern';

export interface CodeAnnotation {
  id: string;
  type: CodeAnnotationType;
  filePath: string;
  lineStart: number;
  lineEnd: number;
  side: 'old' | 'new';
  text?: string;
  suggestedCode?: string;
  originalCode?: string;
  createdAt: number;
  author?: string;
}

export interface DiffAnnotationMetadata {
  annotationId: string;
  type: CodeAnnotationType;
  text?: string;
  suggestedCode?: string;
  originalCode?: string;
  author?: string;
}

export interface SelectedLineRange {
  start: number;
  end: number;
  side: 'deletions' | 'additions';
  endSide?: 'deletions' | 'additions';
}

export interface VaultNode {
  name: string;
  path: string;
  type: "file" | "folder";
  children?: VaultNode[];
}
