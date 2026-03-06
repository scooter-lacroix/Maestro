import type { 
  HighlightSource, 
  HighlighterOptions, 
  SelectionData, 
  ClickData,
  RemoveData,
  SelectionInfo,
  EventCallback 
} from './types.js';
import { SelectionHandler } from './selection/index.js';
import { HighlightRenderer } from './highlights/index.js';
import { generateId } from './utils/index.js';

export * from './types.js';
export * from './selection/index.js';
export * from './highlights/index.js';
export * from './utils/index.js';

/**
 * TrackLens Web Highlighter
 * 
 * A lightweight library for text selection and annotation highlighting.
 * Compatible with the @plannotator/web-highlighter API.
 * 
 * @example
 * ```typescript
 * const highlighter = new Highlighter({
 *   $root: containerRef.current,
 *   exceptSelectors: ['.annotation-toolbar', 'button'],
 *   wrapTag: 'mark',
 *   style: { className: 'annotation-highlight' }
 * });
 * 
 * highlighter.on(Highlighter.event.CREATE, ({ sources }) => {
 *   console.log('New highlight:', sources[0]);
 * });
 * 
 * highlighter.run();
 * ```
 */
export default class Highlighter {
  static event = {
    CREATE: 'CREATE' as const,
    CLICK: 'CLICK' as const,
    REMOVE: 'REMOVE' as const,
  };

  private root: HTMLElement;
  private selectionHandler: SelectionHandler;
  private renderer: HighlightRenderer;
  private eventListeners: Map<string, Set<EventCallback<any>>> = new Map();
  private isRunning = false;
  private exceptSelectors: string[];

  constructor(options: HighlighterOptions) {
    this.root = options.$root;
    this.exceptSelectors = options.exceptSelectors || [];
    
    this.selectionHandler = new SelectionHandler(
      this.root,
      this.exceptSelectors
    );
    
    this.renderer = new HighlightRenderer(
      this.root,
      options.wrapTag || 'mark',
      options.style?.className || 'highlight'
    );

    // Bind methods to preserve context
    this.handleMouseUp = this.handleMouseUp.bind(this);
    this.handleHighlightClick = this.handleHighlightClick.bind(this);
  }

  /**
   * Create a Highlighter instance from a container element
   * Convenience method for React refs
   */
  static from(container: HTMLElement | null, options?: Partial<HighlighterOptions>): Highlighter | null {
    if (!container) return null;
    return new Highlighter({
      $root: container,
      ...options,
    });
  }

  /**
   * Start listening for selection events
   */
  run(): void {
    if (this.isRunning) return;
    this.isRunning = true;
    
    document.addEventListener('mouseup', this.handleMouseUp);
    this.root.addEventListener('click', this.handleHighlightClick);
  }

  /**
   * Stop listening for events and cleanup
   */
  dispose(): void {
    this.isRunning = false;
    document.removeEventListener('mouseup', this.handleMouseUp);
    this.root.removeEventListener('click', this.handleHighlightClick);
  }

  /**
   * Register an event listener
   */
  on<T>(event: string, callback: EventCallback<T>): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }
    this.eventListeners.get(event)!.add(callback);
  }

  /**
   * Remove an event listener
   */
  off<T>(event: string, callback: EventCallback<T>): void {
    this.eventListeners.get(event)?.delete(callback);
  }

  /**
   * Set a callback for text selection (convenience method)
   * Matches the plannotator API: highlighter.onSelect((selection) => {...})
   */
  onSelect(callback: (selection: SelectionInfo) => void): void {
    this.on(Highlighter.event.CREATE, ({ sources }: SelectionData) => {
      if (sources.length > 0) {
        const source = sources[0];
        const range = this.selectionHandler.getRange();
        if (range) {
          callback({ text: source.text, id: source.id, range });
        }
      }
    });
  }

  /**
   * Create a highlight from the current selection
   */
  highlightSelection(): HighlightSource | null {
    if (!this.selectionHandler.hasValidSelection()) {
      return null;
    }

    const id = generateId();
    const source = this.selectionHandler.serialize(id);
    
    if (!source) {
      return null;
    }

    // Render the highlight
    const elements = this.renderer.renderRange(
      this.selectionHandler.getRange()!,
      id
    );

    if (elements.length === 0) {
      return null;
    }

    // Clear the selection
    this.selectionHandler.clear();

    // Emit CREATE event
    this.emit(Highlighter.event.CREATE, { sources: [source] });

    return source;
  }

  /**
   * Create a highlight from a Range
   * Matches the plannotator API: highlighter.highlight(range, annotation)
   */
  highlight(range: Range, source?: Partial<HighlightSource>): HighlightSource {
    const id = source?.id || generateId();
    
    const elements = this.renderer.renderRange(range, id);
    
    const highlightSource: HighlightSource = {
      id,
      text: source?.text || range.toString(),
      startMeta: source?.startMeta || {
        parentTagName: 'p',
        parentIndex: 0,
        textOffset: 0,
      },
      endMeta: source?.endMeta || {
        parentTagName: 'p',
        parentIndex: 0,
        textOffset: 0,
      },
    };

    return highlightSource;
  }

  /**
   * Remove a highlight by ID
   * Matches the plannotator API: highlighter.remove(id)
   */
  remove(id: string): void {
    this.renderer.remove(id);
    this.emit(Highlighter.event.REMOVE, { id });
  }

  /**
   * Remove a highlight by ID (alias for remove)
   * Matches the plannotator API: highlighter.removeHighlight(id)
   */
  removeHighlight(id: string): void {
    this.remove(id);
  }

  /**
   * Get DOM elements for a highlight
   * Matches the plannotator API: highlighter.getDoms(id)
   */
  getDoms(id: string): Element[] {
    return this.renderer.getDoms(id);
  }

  /**
   * Add a class to a highlight
   * Matches the plannotator API: highlighter.addClass(className, id)
   */
  addClass(className: string, id: string): void {
    this.renderer.addClass(className, id);
  }

  /**
   * Remove a class from a highlight
   */
  removeClass(className: string, id: string): void {
    this.renderer.removeClass(className, id);
  }

  /**
   * Check if a highlight exists
   */
  has(id: string): boolean {
    return this.renderer.has(id);
  }

  /**
   * Clear all highlights
   * Matches the plannotator API: highlighter.clearAllHighlights()
   */
  clearAllHighlights(): void {
    this.renderer.clearAll();
  }

  /**
   * Get all highlight IDs
   */
  getAllHighlightIds(): string[] {
    const highlights = this.root.querySelectorAll('[data-highlight-id]');
    const ids = new Set<string>();
    highlights.forEach(el => {
      const id = el.getAttribute('data-highlight-id');
      if (id) ids.add(id);
    });
    return Array.from(ids);
  }

  /**
   * Apply highlights from shared annotation sources
   * Matches the plannotator API: highlighter.applySharedAnnotations(annotations)
   */
  applySharedAnnotations(sources: HighlightSource[]): void {
    for (const source of sources) {
      // Skip if already highlighted
      if (this.has(source.id)) continue;
      
      this.renderer.renderSource(source);
    }
  }

  private handleMouseUp(): void {
    // Small delay to allow selection to complete
    setTimeout(() => {
      if (!this.selectionHandler.hasValidSelection()) return;

      const id = generateId();
      const source = this.selectionHandler.serialize(id);
      
      if (!source) return;

      // Render the highlight
      const elements = this.renderer.renderRange(
        this.selectionHandler.getRange()!,
        id
      );

      if (elements.length === 0) {
        this.selectionHandler.clear();
        return;
      }

      // Emit CREATE event
      this.emit(Highlighter.event.CREATE, { sources: [source] });
    }, 10);
  }

  private handleHighlightClick(event: MouseEvent): void {
    const target = event.target as HTMLElement;
    const highlightEl = target.closest('[data-highlight-id]');
    
    if (!highlightEl) return;

    const id = highlightEl.getAttribute('data-highlight-id');
    if (!id) return;

    this.emit(Highlighter.event.CLICK, { id });
  }

  private emit<T>(event: string, data: T): void {
    this.eventListeners.get(event)?.forEach(callback => {
      try {
        callback(data);
      } catch (e) {
        console.error(`Error in ${event} handler:`, e);
      }
    });
  }
}
