/**
 * TrackLens - Parser Utility
 *
 * Simplified markdown parser with YAML frontmatter support.
 * Extracts frontmatter and splits content into blocks for annotation.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import type { Block } from '../types';

/**
 * Parsed YAML frontmatter as key-value pairs.
 */
export interface Frontmatter {
  [key: string]: string | string[];
}

/**
 * Extract YAML frontmatter from markdown if present.
 */
export function extractFrontmatter(markdown: string): { frontmatter: Frontmatter | null; content: string } {
  const trimmed = markdown.trimStart();
  if (!trimmed.startsWith('---')) {
    return { frontmatter: null, content: markdown };
  }

  const endIndex = trimmed.indexOf('\n---', 3);
  if (endIndex === -1) {
    return { frontmatter: null, content: markdown };
  }

  const frontmatterRaw = trimmed.slice(4, endIndex).trim();
  const afterFrontmatter = trimmed.slice(endIndex + 4).trimStart();

  const frontmatter: Frontmatter = {};
  let currentKey: string | null = null;
  let currentArray: string[] | null = null;

  for (const line of frontmatterRaw.split('\n')) {
    const trimmedLine = line.trim();

    if (trimmedLine.startsWith('- ') && currentKey) {
      const value = trimmedLine.slice(2).trim();
      if (!currentArray) {
        currentArray = [];
        frontmatter[currentKey] = currentArray;
      }
      currentArray.push(value);
      continue;
    }

    const colonIndex = trimmedLine.indexOf(':');
    if (colonIndex > 0) {
      currentKey = trimmedLine.slice(0, colonIndex).trim();
      const value = trimmedLine.slice(colonIndex + 1).trim();
      currentArray = null;

      if (value) {
        frontmatter[currentKey] = value;
      }
    }
  }

  return { frontmatter, content: afterFrontmatter };
}

/**
 * Parse markdown content into blocks for annotation.
 */
export const parseMarkdownToBlocks = (markdown: string): Block[] => {
  const { content: cleanMarkdown } = extractFrontmatter(markdown);
  const lines = cleanMarkdown.split('\n');
  const blocks: Block[] = [];
  let currentId = 0;

  let buffer: string[] = [];
  let currentType: Block['type'] = 'paragraph';
  let currentLevel = 0;
  let bufferStartLine = 1;

  const flush = () => {
    if (buffer.length > 0) {
      const content = buffer.join('\n');
      blocks.push({
        id: `block-${currentId++}`,
        type: currentType,
        content: content,
        level: currentLevel,
        order: currentId,
        startLine: bufferStartLine
      });
      buffer = [];
    }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    const currentLineNum = i + 1;

    // Code blocks
    if (trimmed.startsWith('```')) {
      flush();
      const language = line.slice(3).trim() || 'plaintext';
      bufferStartLine = currentLineNum;
      buffer = [];
      
      for (let j = i + 1; j < lines.length; j++) {
        if (lines[j].trim() === '```') {
          const codeContent = lines.slice(i + 1, j).join('\n');
          blocks.push({
            id: `block-${currentId++}`,
            type: 'code',
            content: codeContent,
            language,
            order: currentId,
            startLine: bufferStartLine
          });
          i = j;
          buffer = [];
          break;
        }
      }
      continue;
    }

    // Headings
    const headingMatch = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (headingMatch) {
      flush();
      currentType = 'heading';
      currentLevel = headingMatch[1].length;
      bufferStartLine = currentLineNum;
      buffer = [headingMatch[2]];
      flush();
      continue;
    }

    // Horizontal rule
    if (/^[-*_]{3,}\s*$/.test(trimmed)) {
      flush();
      blocks.push({
        id: `block-${currentId++}`,
        type: 'hr',
        content: '',
        order: currentId,
        startLine: currentLineNum
      });
      continue;
    }

    // Blockquote
    if (trimmed.startsWith('>')) {
      if (currentType !== 'blockquote') {
        flush();
        currentType = 'blockquote';
        bufferStartLine = currentLineNum;
      }
      buffer.push(trimmed.slice(1).trim());
      continue;
    }

    // List items
    if (/^[*\-+]\s/.test(trimmed) || /^\d+\.\s/.test(trimmed)) {
      if (currentType !== 'list-item') {
        flush();
        currentType = 'list-item';
        bufferStartLine = currentLineNum;
      }
      buffer.push(trimmed.replace(/^[*\-+]\s|\d+\.\s/, ''));
      continue;
    }

    // Tables (simplified - single line detection)
    if (trimmed.includes('|')) {
      flush();
      blocks.push({
        id: `block-${currentId++}`,
        type: 'table',
        content: trimmed,
        order: currentId,
        startLine: currentLineNum
      });
      continue;
    }

    // Paragraphs (default)
    if (!trimmed) {
      flush();
      continue;
    }

    if (currentType !== 'paragraph') {
      flush();
      currentType = 'paragraph';
      bufferStartLine = currentLineNum;
    }
    buffer.push(line);
  }

  flush();

  return blocks;
};

/**
 * Export annotations to JSON format.
 */
export function exportAnnotations(blocks: unknown[], annotations: unknown[], globalAttachments: unknown[] = []): string {
  return JSON.stringify({ annotations, blocks }, null, 2);
}

/**
 * Export linked doc annotations to JSON format.
 */
export function exportLinkedDocAnnotations(docAnnotations: Map<string, unknown[]>): string {
  const obj = Object.fromEntries(docAnnotations);
  return JSON.stringify(obj, null, 2);
}
