/**
 * TrackLens - Portable Sharing Utilities (Local Only)
 *
 * Enables sharing plan + annotations via URL hash using:
 * - Native CompressionStream/DecompressionStream (deflate-raw)
 * - Base64url encoding for URL safety
 *
 * NOTE: External paste service removed - local hash-based sharing only.
 * This aligns with TrackLens's local-first architecture.
 *
 * REBRANDED: Plannotator → TrackLens
 * REMOVED: Short URL / paste service integration
 *
 * @packageDocumentation
 */

import { Annotation, AnnotationType, type ImageAttachment } from '../types';

// Image in shareable format: plain string (old) or [path, name] tuple (new)
type ShareableImage = string | [string, string];

// Minimal shareable annotation format: [type, originalText, text?, author?, images?]
export type ShareableAnnotation =
  | ['D', string, string | null, ShareableImage[]?]             // Deletion: type, original, author, images
  | ['R', string, string, string | null, ShareableImage[]?]     // Replacement: type, original, replacement, author, images
  | ['C', string, string, string | null, ShareableImage[]?]     // Comment: type, original, comment, author, images
  | ['I', string, string, string | null, ShareableImage[]?]     // Insertion: type, context, new text, author, images
  | ['G', string, string | null, ShareableImage[]?];            // Global Comment: type, comment, author, images

export interface SharePayload {
  p: string;  // plan markdown
  a: ShareableAnnotation[];
  g?: ShareableImage[];  // global attachments (path strings or [path, name] tuples)
}

/**
 * Convert ShareableImage[] to ImageAttachment[] (handles old plain-string format)
 */
export function parseShareableImages(raw: ShareableImage[] | undefined): ImageAttachment[] | undefined {
  if (!raw?.length) return undefined;
  return raw.map(img => {
    if (typeof img === 'string') {
      // Old format: plain path string — derive name from filename
      const name = img.split('/').pop()?.replace(/\.[^.]+$/, '') || 'image';
      return { path: img, name };
    }
    return { path: img[0], name: img[1] };
  });
}

/**
 * Convert ImageAttachment[] to ShareableImage[] for compact serialization
 */
export function toShareableImages(images: ImageAttachment[] | undefined): ShareableImage[] | undefined {
  if (!images?.length) return undefined;
  return images.map(img => [img.path, img.name]);
}

/**
 * Compress data using CompressionStream (deflate-raw)
 */
export async function compress(data: unknown): Promise<string> {
  const json = JSON.stringify(data);
  const encoder = new TextEncoder();
  const bytes = encoder.encode(json);

  const stream = new CompressionStream('deflate-raw');
  const writer = stream.writable.getWriter();
  writer.write(bytes);
  writer.close();

  const compressed = await new Response(stream.readable).arrayBuffer();

  // Base64url encode (URL-safe, no padding)
  const base64 = btoa(String.fromCharCode(...new Uint8Array(compressed)));
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/**
 * Decompress data using DecompressionStream (deflate-raw)
 */
export async function decompress<T>(compressed: string): Promise<T> {
  // Base64url decode
  const base64 = compressed.replace(/-/g, '+').replace(/_/g, '/');
  const padded = base64 + '='.repeat((4 - (base64.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  const stream = new DecompressionStream('deflate-raw');
  const writer = stream.writable.getWriter();
  writer.write(bytes);
  writer.close();

  const decompressed = await new Response(stream.readable).arrayBuffer();
  const decoder = new TextDecoder();
  const json = decoder.decode(decompressed);

  return JSON.parse(json);
}

/**
 * Convert full Annotation objects to minimal shareable format
 */
export function toShareable(annotations: Annotation[]): ShareableAnnotation[] {
  return annotations.map(ann => {
    const author = ann.author || null;
    const images = toShareableImages(ann.images);

    // Handle GLOBAL_COMMENT specially - it starts with 'G' (from GLOBAL_COMMENT)
    if (ann.type === AnnotationType.GLOBAL_COMMENT) {
      return ['G', ann.text || '', author, images] as ShareableAnnotation;
    }

    const type = ann.type[0] as 'D' | 'R' | 'C' | 'I';

    if (type === 'D') {
      return ['D', ann.originalText, author, images] as ShareableAnnotation;
    }

    // R, C, I all have text
    return [type, ann.originalText, ann.text || '', author, images] as ShareableAnnotation;
  });
}

/**
 * Convert shareable format back to full Annotation objects
 * Note: blockId, offsets, and meta will need to be populated separately
 * by finding the text in the rendered document.
 */
export function fromShareable(data: ShareableAnnotation[]): Annotation[] {
  const typeMap: Record<string, AnnotationType> = {
    'D': AnnotationType.DELETION,
    'R': AnnotationType.REPLACEMENT,
    'C': AnnotationType.COMMENT,
    'I': AnnotationType.INSERTION,
    'G': AnnotationType.GLOBAL_COMMENT,
  };

  return data.map((item, index) => {
    const type = item[0];

    // Handle global comments specially: ['G', text, author, images?]
    if (type === 'G') {
      const text = item[1] as string;
      const author = item[2] as string | null;
      const rawImages = item[3] as ShareableImage[] | undefined;

      return {
        id: `shared-${index}-${Date.now()}`,
        blockId: '',
        startOffset: 0,
        endOffset: 0,
        type: AnnotationType.GLOBAL_COMMENT,
        text: text || undefined,
        originalText: '',
        createdA: Date.now() + index,
        author: author || undefined,
        images: parseShareableImages(rawImages),
      };
    }

    const originalText = item[1];
    // For deletion: [type, original, author, images?]
    // For others: [type, original, text, author, images?]
    const text = type === 'D' ? undefined : item[2] as string;
    const author = type === 'D' ? item[2] as string | null : item[3] as string | null;
    const rawImages = type === 'D' ? item[3] as ShareableImage[] | undefined : item[4] as ShareableImage[] | undefined;

    return {
      id: `shared-${index}-${Date.now()}`,
      blockId: '',  // Will be populated during highlight restoration
      startOffset: 0,
      endOffset: 0,
      type: typeMap[type],
      text: text || undefined,
      originalText,
      createdA: Date.now() + index,  // Preserve order
      author: author || undefined,
      images: parseShareableImages(rawImages),
      // startMeta/endMeta will be set by web-highlighter
    };
  });
}

/**
 * Generate a full shareable URL from plan and annotations
 */
export async function generateShareUrl(
  markdown: string,
  annotations: Annotation[],
  globalAttachments?: ImageAttachment[],
  baseUrl: string = window.location.origin + window.location.pathname
): Promise<string> {
  const payload: SharePayload = {
    p: markdown,
    a: toShareable(annotations),
    g: globalAttachments?.length ? toShareableImages(globalAttachments) : undefined,
  };

  const hash = await compress(payload);
  return `${baseUrl}#${hash}`;
}

/**
 * Parse a share URL hash and return the payload
 * Returns null if no valid hash or parsing fails
 */
export async function parseShareHash(): Promise<SharePayload | null> {
  const hash = window.location.hash.slice(1); // Remove leading #

  if (!hash) {
    return null;
  }

  try {
    return await decompress(hash);
  } catch (e) {
    console.warn('Failed to parse share hash:', e);
    return null;
  }
}

/**
 * Get the size of a URL in a human-readable format
 */
export function formatUrlSize(url: string): string {
  const bytes = new Blob([url]).size;
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

// NOTE: Short URL / paste service functionality removed.
// TrackLens uses local hash-based sharing only for privacy.
// The following functions from Plannotator are NOT ported:
// - createShortShareUrl()
// - loadFromPasteId()
