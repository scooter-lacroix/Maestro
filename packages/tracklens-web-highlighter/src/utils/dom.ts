/**
 * Generate a unique ID for highlights
 */
export function generateId(): string {
  return `highlight-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Check if an element matches any of the except selectors
 */
export function isExceptElement(element: Element, exceptSelectors: string[]): boolean {
  return exceptSelectors.some(selector => {
    try {
      return element.matches(selector) || element.closest(selector) !== null;
    } catch {
      return false;
    }
  });
}

/**
 * Get the index of an element among its siblings with the same tag name
 */
export function getElementIndex(element: Element): number {
  const parent = element.parentElement;
  if (!parent) return 0;
  
  const siblings = Array.from(parent.children).filter(
    child => child.tagName === element.tagName
  );
  return siblings.indexOf(element);
}

/**
 * Find an element by tag name and index within a root container
 */
export function findElementByIndex(
  root: HTMLElement,
  tagName: string,
  index: number
): Element | null {
  const elements = Array.from(root.getElementsByTagName(tagName));
  const filtered = elements.filter(el => root.contains(el));
  return filtered[index] || null;
}

/**
 * Get text offset within an element's text content
 */
export function getTextOffset(container: Node, offset: number, root: HTMLElement): number {
  let count = 0;
  const walker = document.createTreeWalker(
    root,
    NodeFilter.SHOW_TEXT,
    null
  );
  
  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    if (node === container) {
      return count + offset;
    }
    count += node.textContent?.length || 0;
  }
  
  return count;
}
