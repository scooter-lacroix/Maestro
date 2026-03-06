/**
 * TrackLens - Annotation Helpers
 *
 * Utilities for building TOC hierarchy and counting annotations.
 *
 * REBRANDED: Plannotator → TrackLens
 */

import type { Block, Annotation } from '../types';

export interface TocItem {
  id: string;
  content: string;
  level: number;
  annotationCount: number;
  children: TocItem[];
}

export function buildTocHierarchy(
  blocks: Block[],
  annotationCounts: Map<string, number>
): TocItem[] {
  const items: TocItem[] = [];
  const stack: TocItem[] = [];

  for (const block of blocks) {
    if (block.type !== 'heading') continue;

    const item: TocItem = {
      id: block.id,
      content: block.content,
      level: block.level || 1,
      annotationCount: annotationCounts.get(block.id) || 0,
      children: [],
    };

    // Find parent level
    while (stack.length > 0 && stack[stack.length - 1].level >= item.level) {
      stack.pop();
    }

    if (stack.length === 0) {
      items.push(item);
    } else {
      stack[stack.length - 1].children.push(item);
    }

    stack.push(item);
  }

  return items;
}

export function getAnnotationCountBySection(
  blocks: Block[],
  annotations: Annotation[]
): Map<string, number> {
  const counts = new Map<string, number>();

  // Initialize all blocks with 0
  for (const block of blocks) {
    counts.set(block.id, 0);
  }

  // Count annotations per block
  for (const ann of annotations) {
    const current = counts.get(ann.blockId) || 0;
    counts.set(ann.blockId, current + 1);
  }

  return counts;
}
