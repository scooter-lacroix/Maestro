# @maestro/tracklens-web-highlighter

Text selection and annotation highlighting library for TrackLens. Ported from `@plannotator/web-highlighter`.

## Features

- Text selection detection with cross-element highlighting
- Highlight persistence via startMeta/endMeta serialization
- Highlight rendering with configurable wrap tags
- Event-based API (CREATE, CLICK, REMOVE)
- Compatible with React and other frameworks

## Installation

```bash
npm install @maestro/tracklens-web-highlighter
```

## API Usage

### Basic Usage

```typescript
import Highlighter from '@maestro/tracklens-web-highlighter';

const highlighter = new Highlighter({
  $root: containerRef.current,
  exceptSelectors: ['.annotation-toolbar', 'button'],
  wrapTag: 'mark',
  style: { className: 'annotation-highlight' }
});

highlighter.on(Highlighter.event.CREATE, ({ sources }) => {
  console.log('New highlight:', sources[0]);
});

highlighter.on(Highlighter.event.CLICK, ({ id }) => {
  console.log('Clicked highlight:', id);
});

highlighter.run();
```

### React Integration (TrackLens Viewer Pattern)

```typescript
import Highlighter from '@maestro/tracklens-web-highlighter';

// In your component
const containerRef = useRef<HTMLDivElement>(null);
const highlighterRef = useRef<Highlighter | null>(null);

useEffect(() => {
  if (!containerRef.current) return;

  const highlighter = new Highlighter({
    $root: containerRef.current,
    exceptSelectors: ['.annotation-toolbar', 'button'],
    wrapTag: 'mark',
    style: { className: 'annotation-highlight' }
  });

  highlighterRef.current = highlighter;

  // Listen for text selection
  highlighter.on(Highlighter.event.CREATE, ({ sources }) => {
    if (sources.length > 0) {
      const source = sources[0];
      // Handle new selection
      console.log('Selected:', source.text);
    }
  });

  // Listen for highlight clicks
  highlighter.on(Highlighter.event.CLICK, ({ id }) => {
    onSelectAnnotation(id);
  });

  highlighter.run();

  return () => highlighter.dispose();
}, [onSelectAnnotation]);
```

### Convenience Method

```typescript
// Create from container (returns null if container is null)
const highlighter = Highlighter.from(containerRef.current, {
  wrapTag: 'mark',
  style: { className: 'annotation-highlight' }
});

// Or with onSelect callback
highlighter?.onSelect((selection) => {
  console.log('Text:', selection.text);
  console.log('ID:', selection.id);
  console.log('Range:', selection.range);
});
```

### Highlight Management

```typescript
// Create highlight from a Range
const source = highlighter.highlight(range, {
  id: 'annotation-1',
  text: 'selected text',
  startMeta: { parentTagName: 'p', parentIndex: 0, textOffset: 10 },
  endMeta: { parentTagName: 'p', parentIndex: 0, textOffset: 23 }
});

// Remove a highlight
highlighter.remove(id);
highlighter.removeHighlight(id); // alias

// Add CSS class to highlight
highlighter.addClass('deletion', id);
highlighter.addClass('comment', id);

// Check if highlight exists
const exists = highlighter.has(id);

// Get DOM elements for a highlight
const elements = highlighter.getDoms(id);

// Clear all highlights
highlighter.clearAllHighlights();

// Get all highlight IDs
const ids = highlighter.getAllHighlightIds();

// Apply shared annotations
highlighter.applySharedAnnotations(annotationSources);
```

## Types

```typescript
interface HighlightSource {
  id: string;
  text: string;
  startMeta: {
    parentTagName: string;
    parentIndex: number;
    textOffset: number;
  };
  endMeta: {
    parentTagName: string;
    parentIndex: number;
    textOffset: number;
  };
}

interface HighlighterOptions {
  $root: HTMLElement;
  exceptSelectors?: string[];
  wrapTag?: string;
  style?: {
    className?: string;
  };
}
```

## Events

- `Highlighter.event.CREATE` - Emitted when text is selected and highlighted
- `Highlighter.event.CLICK` - Emitted when a highlight is clicked
- `Highlighter.event.REMOVE` - Emitted when a highlight is removed

## License

MIT
