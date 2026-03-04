/**
 * TrackLens Web Highlighter
 *
 * Lightweight text selection library for annotation.
 *
 * REBRANDED: Plannotator → TrackLens
 */

export interface Range {
  startContainer: Node;
  startOffset: number;
  endContainer: Node;
  endOffset: number;
}

export class Highlighter {
  private container: HTMLElement;
  private highlights: Map<string, HTMLElement> = new Map();

  constructor(container: HTMLElement) {
    this.container = container;
  }

  /**
   * Get the current text selection as a range
   */
  getSelection(): Range | null {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return null;

    const range = selection.getRangeAt(0);
    
    // Check if selection is within our container
    if (!this.container.contains(range.commonAncestorContainer)) {
      return null;
    }

    return {
      startContainer: range.startContainer,
      startOffset: range.startOffset,
      endContainer: range.endContainer,
      endOffset: range.endOffset,
    };
  }

  /**
   * Clear the current selection
   */
  clearSelection(): void {
    const selection = window.getSelection();
    selection?.removeAllRanges();
  }

  /**
   * Apply a highlight to the document
   */
  applyHighlight(id: string, range: Range): void {
    const span = document.createElement('span');
    span.id = id;
    span.className = 'tracklens-highlight';
    span.style.backgroundColor = 'rgba(250, 204, 21, 0.3)';
    span.style.borderBottom = '2px solid rgba(250, 204, 21, 0.8)';

    const domRange = document.createRange();
    domRange.setStart(range.startContainer, range.startOffset);
    domRange.setEnd(range.endContainer, range.endOffset);

    try {
      domRange.surroundContents(span);
      this.highlights.set(id, span);
    } catch (e) {
      // Selection crosses block boundaries, use alternative approach
      console.warn('Could not surround contents, selection may cross block boundaries');
    }
  }

  /**
   * Remove a highlight by ID
   */
  removeHighlight(id: string): void {
    const highlight = this.highlights.get(id);
    if (highlight) {
      const parent = highlight.parentNode;
      if (parent) {
        while (highlight.firstChild) {
          parent.insertBefore(highlight.firstChild, highlight);
        }
        parent.removeChild(highlight);
      }
      this.highlights.delete(id);
    }
  }

  /**
   * Clear all highlights
   */
  clearAllHighlights(): void {
    for (const id of this.highlights.keys()) {
      this.removeHighlight(id);
    }
  }

  /**
   * Get all highlight IDs
   */
  getHighlightIds(): string[] {
    return Array.from(this.highlights.keys());
  }
}

export default Highlighter;
