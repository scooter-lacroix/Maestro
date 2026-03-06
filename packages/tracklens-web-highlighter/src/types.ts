/**
 * Metadata for the start/end of a highlight
 * Used for serializing and deserializing selection positions
 */
export interface HighlightMeta {
  /** Tag name of the parent element */
  parentTagName: string;
  /** Index of the parent among siblings with the same tag name */
  parentIndex: number;
  /** Character offset within the parent element's text content */
  textOffset: number;
}

/**
 * Source data for a highlight
 * Contains all information needed to recreate a highlight
 */
/**
 * Metadata for the start/end of a highlight
 * Used for serializing and deserializing selection positions
 */
export interface HighlightMeta {
  /** Tag name of the parent element */
  parentTagName: string;
  /** Index of the parent among siblings with the same tag name */
  parentIndex: number;
  /** Character offset within the parent element's text content */
  textOffset: number;
}

/**
 * Source data for a highlight
 * Contains all information needed to recreate a highlight
 */
export interface HighlightSource {
  /** Unique identifier for the highlight */
  id: string;
  /** The selected text content */
  text: string;
  /** Metadata for the start position */
  startMeta: HighlightMeta;
  /** Metadata for the end position */
  endMeta: HighlightMeta;
}

/**
 * Options for creating a Highlighter instance
 */
export interface HighlighterOptions {
  /** Root container element to monitor for selections */
  $root: HTMLElement;
  /** CSS selectors for elements to ignore during selection */
  exceptSelectors?: string[];
  /** HTML tag to use for highlight wrapping (default: 'mark') */
  wrapTag?: string;
  /** Styling options */
  style?: {
    /** CSS class name for highlight elements */
    className?: string;
  };
}

/**
 * Data passed to CREATE event listeners
 */
export interface SelectionData {
  /** Array of highlight sources that were created */
  sources: HighlightSource[];
}

/**
 * Data passed to CLICK event listeners
 */
export interface ClickData {
  /** ID of the clicked highlight */
  id: string;
}

/**
 * Data passed to REMOVE event listeners
 */
export interface RemoveData {
  /** ID of the removed highlight */
  id: string;
}

/** Highlighter event types */
export type HighlighterEvent = 'CREATE' | 'CLICK' | 'REMOVE';

/** Generic event callback type */
export type EventCallback<T> = (data: T) => void;

/**
 * Selection info returned from onSelect callback
 */
export interface SelectionInfo {
  /** The selected text */
  text: string;
  /** Unique ID assigned to this highlight */
  id: string;
  /** The DOM Range representing the selection */
  range: Range;
}

/**
 * Selection info returned from onSelect callback
 */
export interface SelectionInfo {
  /** The selected text */
  text: string;
  /** Unique ID assigned to this highlight */
  id: string;
  /** The DOM Range representing the selection */
  range: Range;
}
