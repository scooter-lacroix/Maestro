import type { HighlightSource } from '../types.js';
import { serializeRange } from '../utils/serialization.js';
import { isExceptElement } from '../utils/dom.js';

/**
 * Manages text selection detection and processing
 */
export class SelectionHandler {
  private root: HTMLElement;
  private exceptSelectors: string[];

  constructor(root: HTMLElement, exceptSelectors: string[] = []) {
    this.root = root;
    this.exceptSelectors = exceptSelectors;
  }

  /**
   * Get the current selection within the root container
   */
  getSelection(): Selection | null {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return null;
    
    const range = selection.getRangeAt(0);
    
    // Check if selection is within root
    if (!this.root.contains(range.commonAncestorContainer)) {
      return null;
    }

    // Check if selection is in an excepted element
    const container = range.commonAncestorContainer;
    const element = container.nodeType === Node.ELEMENT_NODE 
      ? container as Element 
      : container.parentElement;
    
    if (element && isExceptElement(element, this.exceptSelectors)) {
      return null;
    }

    return selection;
  }

  /**
   * Check if there's a valid text selection
   */
  hasValidSelection(): boolean {
    const selection = this.getSelection();
    if (!selection) return false;
    
    const range = selection.getRangeAt(0);
    const text = range.toString().trim();
    return text.length > 0;
  }

  /**
   * Get the selected range
   */
  getRange(): Range | null {
    const selection = this.getSelection();
    if (!selection || selection.rangeCount === 0) return null;
    return selection.getRangeAt(0);
  }

  /**
   * Serialize the current selection into a HighlightSource
   */
  serialize(id: string): HighlightSource | null {
    const range = this.getRange();
    if (!range) return null;
    return serializeRange(range, this.root, id);
  }

  /**
   * Clear the current selection
   */
  clear(): void {
    window.getSelection()?.removeAllRanges();
  }
}
