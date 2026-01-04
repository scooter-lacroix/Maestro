# Maestro Memory Dashboard - Frontend

## Overview

This is the frontend for the Maestro Memory Dashboard, built with React, TypeScript, and Vite. It features a brutalist-inspired design with advanced visual effects.

## Visual Effects

### Effect 1: Magical Card Hover Effect
- 3x2 grid of cards with mouse-following radial gradient glow
- Inner card nested 1px inside wrapper card
- Outer glow appears on all cards when any card is hovered
- Creates illusion of glow extending across neighboring cards

### Effect 2: Futuristic Text Glitch Hover Effect
- Square card with fa-plus icons in corners
- White borders extending to screen edges
- Randomized alphanumeric text on hover
- Radial gradient mask (sea blue → aqua green → white) follows mouse
- Text and gradient fade out on mouse leave

### Effect 3: Infinite Looping Image Echo Effect
- Used in memories section
- 8 image copies paired into 4 sets
- Each pair animates: scale 1→0.9, translate 0→±25%, opacity 1→0
- Creates effect of moving backward and away while fading
- Infinite loop with staggered animations

### Effect 4: Fantastical Mouse Trailer Effect
- Soft neon pink glow follows mouse
- Falling stars using Font Awesome fa-star icons
- Stars alternate between neon pink and white
- Stars spawn randomly around mouse and fall downward
- Stars rotate and fade out over ~1 second

## Features

- **Project Management**: View all Maestro projects with details
- **Track Visualization**: See tracks, progress, and status
- **Memory Browser**: Browse recent memories with full formatting
- **Search**: Semantic search across all memories
- **Statistics**: Overview of memory system usage
- **Expandable Cards**: Click cards to reveal more information

## Development

### Prerequisites

- Node.js 18+
- npm or yarn

### Installation

```bash
cd /home/stan/Prod/maestro/maestro/memory/frontend
npm install
```

### Development Server

```bash
npm run dev
```

The dashboard will be available at `http://localhost:3000`

### Build for Production

```bash
npm run build
```

Built files will be in the `dist/` directory.

## API Integration

The frontend communicates with the FastAPI backend via these endpoints:

- `GET /health` - Health check
- `GET /api/v1/memories` - List memories
- `GET /api/v1/projects` - List projects
- `GET /api/v1/tracks` - List tracks
- `GET /api/v1/stats` - Get statistics
- `GET /api/v1/search` - Search memories
- `POST /api/v1/store` - Store new memory

## Design Philosophy

**Brutalist Aesthetic:**
- Black background (#0a0a0a)
- High contrast borders
- Monospace typography (Courier New)
- Raw, functional design
- Consistent visual language

**Accessibility:**
- WCAG AAA compliant where possible
- Keyboard navigation support
- Focus indicators
- Semantic HTML
- Screen reader friendly

**Performance:**
- CSS animations (GPU accelerated)
- Minimal JavaScript overhead
- Optimized re-renders
- Lazy loading where appropriate

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool
- **Axios** - HTTP client
- **Font Awesome** - Icons
- **CSS** - Custom styles (no framework)

## License

Part of the Maestro 2.0 unified development framework.
