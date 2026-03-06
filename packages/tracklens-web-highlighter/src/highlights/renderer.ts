import type { HighlightSource } from '../types.js';
import { deserializeRange } from '../utils/serialization.js';

export interface HighlightRenderOptions {
  wrapTag: string;
  className: string;
  id: string;
  onClick?: (id: string) => void;
}

/**
 * Renders highlights in the DOM
 */
export class HighlightRenderer {
  private root: HTMLElement;
  private wrapTag: string;
  private className: string;

  constructor(root: HTMLElement, wrapTag = 'mark', className = 'highlight') {
    this.root = root;
    this.wrapTag = wrapTag;
    this.className = className;
  }

  /**
   * Create a highlight from a Range
   */
  renderRange(range: Range, id: string): Element[] {
    const elements: Element[] = [];
    
    // Collect all text nodes within the range
    const textNodes = this.getTextNodesInRange(range);
    
    // Wrap each text node portion
    for (const { node, start, end } of textNodes) {
      try {
        const wrapper = this.createWrapper(id);
        const nodeRange = document.createRange();
        nodeRange.setStart(node, start);
        nodeRange.setEnd(node, end);
        
        // Use surroundContents for single text node ranges
        nodeRange.surroundContents(wrapper);
        elements.push(wrapper);
      } catch (e) {
        // Fallback: split text and wrap
        const text = node.textContent || '';
        const before = text.slice(0, start);
        const selected = text.slice(start, end);
        const after = text.slice(end);
        
        const wrapper = this.createWrapper(id);
        wrapper.textContent = selected;
        
        const parent = node.parentNode;
        if (parent) {
          if (after) {
            parent.insertBefore(document.createTextNode(after), node.nextSibling);
          }
          parent.insertBefore(wrapper, node.nextSibling);
          if (before) {
            node.textContent = before;
          } else {
            parent.removeChild(node);
          }
          elements.push(wrapper);
        }
      }
    }
    
    return elements;
  }

  /**
   * Create a highlight from a serialized source
   */
  renderSource(source: HighlightSource): Element[] {
    const range = deserializeRange(source, this.root);
    if (!range) return [];
    return this.renderRange(range, source.id);
  }

  /**
   * Remove a highlight by ID
   */
  remove(id: string): void {
    const highlights = this.getDoms(id);
    highlights.forEach(el => this.unwrap(el));
  }

  /**
   * Get all DOM elements for a highlight ID
   * Supports both data-highlight-id (internal) and data-bind-id (Viewer compatibility)
   */
  getDoms(id: string): Element[] {
    const byHighlightId = Array.from(this.root.querySelectorAll(`[data-highlight-id="${id}"]`));
    const byBindId = Array.from(this.root.querySelectorAll(`[data-bind-id="${id}"]`));
    return [...byHighlightId, ...byBindId];
  }

  /**
   * Add a class to a highlight
   */
  addClass(className: string, id: string): void {
    const highlights = this.getDoms(id);
    highlights.forEach(el => el.classList.add(className));
  }

  /**
   * Remove a class from a highlight
   */
  removeClass(className: string, id: string): void {
    const highlights = this.getDoms(id);
    highlights.forEach(el => el.classList.remove(className));
  }

  /**
   * Check if a highlight exists
   */
  has(id: string): boolean {
    return this.getDoms(id).length > 0;
  }

  /**
   * Clear all highlights
   */
  clearAll(): void {
    const highlights = this.root.querySelectorAll(`[data-highlight-id]`);
    highlights.forEach(el => this.unwrap(el));
  }

  /**
   * Create a wrapper element
   */
  private createWrapper(id: string): HTMLElement {
    const wrapper = document.createElement(this.wrapTag);
    wrapper.className = this.className;
    wrapper.dataset.highlightId = id;
    return wrapper;
  }

  /**
   * Unwrap a highlight element
   */
  private unwrap(element: Element): void {
    const parent = element.parentNode;
    if (!parent) return;

    // Move all children to parent
    while (element.firstChild) {
      parent.insertBefore(element.firstChild, element);
    }
    parent.removeChild(element);
    
    // Normalize text nodes
    parent.normalize();
  }

  /**
   * Get all text nodes within a range with their offsets
   */
  private getTextNodesInRange(range: Range): Array<{ node: Text; start: number; end: number }> {
    const nodes: Array<{ node: Text; start: number; end: number }> = [];
    
    const container = range.commonAncestorContainer.nodeType === Node.TEXT_NODE
      ? range.commonAncestorContainer.parentNode!
      : range.commonAncestorContainer;

    const walker = document.createTreeWalker(
      container,
      NodeFilter.SHOW_TEXT,
      null
    );

    let node: Text | null;
    while ((node = walker.nextNode() as Text | null)) {
      // Check if this node is within the range
      const nodeRange = document.createRange();
      nodeRange.selectNodeContents(node);
      
      const isStartNode = node === range.startContainer;
      const isEndNode = node === range.endContainer;
      const isBeforeRange = nodeRange.compareBoundaryPoints(Range.END_TO_START, range) < 0;
      const isAfterRange = nodeRange.compareBoundaryPoints(Range.START_TO_END, range) > 0;
      
      if (isBeforeRange || isAfterRange) continue;
      
      let start = 0;
      let end = node.textContent?.length || 0;
      
      if (isStartNode) {
        start = range.startOffset;
      }
      if (isEndNode) {
        end = range.endOffset;
      }
      
      if (end > start) {
        nodes.push({ node, start, end });
      }
    }
    
    return nodes;
  }
}
