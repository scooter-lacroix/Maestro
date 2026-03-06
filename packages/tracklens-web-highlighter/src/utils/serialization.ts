import type { HighlightSource } from '../types.js';

/**
 * Serialize a DOM Range into highlight source metadata
 */
export function serializeRange(
  range: Range,
  root: HTMLElement,
  id: string
): HighlightSource | null {
  const text = range.toString();
  if (!text.trim()) return null;

  const startContainer = getTextNodeInfo(range.startContainer, range.startOffset, root);
  const endContainer = getTextNodeInfo(range.endContainer, range.endOffset, root);

  if (!startContainer || !endContainer) return null;

  return {
    id,
    text,
    startMeta: startContainer,
    endMeta: endContainer,
  };
}

/**
 * Deserialize highlight source back into a DOM Range
 */
export function deserializeRange(
  source: HighlightSource,
  root: HTMLElement
): Range | null {
  try {
    const startNode = findTextNode(root, source.startMeta);
    const endNode = findTextNode(root, source.endMeta);

    if (!startNode || !endNode) return null;

    const range = document.createRange();
    range.setStart(startNode.node, startNode.offset);
    range.setEnd(endNode.node, endNode.offset);

    return range;
  } catch (e) {
    console.warn('Failed to deserialize range:', e);
    return null;
  }
}

interface TextNodeInfo {
  parentTagName: string;
  parentIndex: number;
  textOffset: number;
}

function getTextNodeInfo(
  container: Node,
  offset: number,
  root: HTMLElement
): TextNodeInfo | null {
  let textNode: Text;
  
  if (container.nodeType === Node.TEXT_NODE) {
    textNode = container as Text;
  } else if (container.nodeType === Node.ELEMENT_NODE) {
    // If container is an element, find the text node at the offset
    const walker = document.createTreeWalker(
      container,
      NodeFilter.SHOW_TEXT,
      null
    );
    let node: Text | null;
    let count = 0;
    while ((node = walker.nextNode() as Text | null)) {
      const length = node.textContent?.length || 0;
      if (count + length >= offset) {
        textNode = node;
        offset = offset - count;
        break;
      }
      count += length;
    }
    if (!textNode!) return null;
  } else {
    return null;
  }

  // Find the parent element that is a direct child of a block-level element
  let parent = textNode.parentElement;
  while (parent && parent !== root) {
    if (isBlockElement(parent) || parent.parentElement === root) {
      break;
    }
    parent = parent.parentElement;
  }

  if (!parent) return null;

  // Get index among siblings with same tag name
  const siblings = Array.from(parent.parentElement?.children || [])
    .filter(child => child.tagName === parent!.tagName);
  const parentIndex = siblings.indexOf(parent);

  // Calculate text offset within the parent
  let textOffset = 0;
  const walker = document.createTreeWalker(
    parent,
    NodeFilter.SHOW_TEXT,
    null
  );
  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    if (node === textNode) {
      textOffset += offset;
      break;
    }
    textOffset += node.textContent?.length || 0;
  }

  return {
    parentTagName: parent.tagName.toLowerCase(),
    parentIndex: Math.max(0, parentIndex),
    textOffset,
  };
}

function findTextNode(
  root: HTMLElement,
  meta: TextNodeInfo
): { node: Text; offset: number } | null {
  const elements = Array.from(root.getElementsByTagName(meta.parentTagName));
  const parent = elements[meta.parentIndex];
  
  if (!parent) return null;

  let currentOffset = 0;
  const walker = document.createTreeWalker(
    parent,
    NodeFilter.SHOW_TEXT,
    null
  );
  
  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    const length = node.textContent?.length || 0;
    if (currentOffset + length >= meta.textOffset) {
      return {
        node,
        offset: meta.textOffset - currentOffset,
      };
    }
    currentOffset += length;
  }

  return null;
}

function isBlockElement(element: Element): boolean {
  const blockTags = ['P', 'DIV', 'SECTION', 'ARTICLE', 'MAIN', 'HEADER', 'FOOTER', 'LI', 'TD', 'TH'];
  return blockTags.includes(element.tagName);
}
