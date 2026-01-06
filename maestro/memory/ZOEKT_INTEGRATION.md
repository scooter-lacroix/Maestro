# Zoekt Integration for Maestro Memory System

## Overview

This document describes the integration of [Zoekt](https://github.com/sourcegraph/zoekt) - a fast trigram-based code search engine - into the Maestro Memory System. This integration enables:

1. **Fast indexed code search** across all Maestro projects
2. **Efficient project discovery** using Zoekt's powerful query syntax
3. **Progressive disclosure UI** for displaying search results without overwhelming users
4. **Graceful fallback** to filesystem traversal when Zoekt is unavailable

## Architecture

### Backend Components

#### 1. Zoekt Client (`search/zoekt_client.py`)

The core Zoekt integration provides:

- **ZoektClient**: Python async client for Zoekt's JSON API
  - Search queries with full Zoekt query syntax support
  - Health checks for Zoekt server availability
  - Maestro-specific project discovery methods

- **ZoektIndexer**: Manages indexing of codebases
  - Creates and updates Zoekt indexes
  - Handles incremental and full re-indexing

- **ZoektConfig**: Configuration dataclass
  - Server URL, index directory
  - Maestro-specific file patterns
  - Search options (max results, context lines)

```python
from maestro.memory.search.zoekt_client import ZoektClient, ZoektConfig

async def search_code():
    config = ZoektConfig(server_url="http://127.0.0.1:6070")
    async with ZoektClient(config) as client:
        results = await client.search("maestro product")
        return results
```

#### 2. Enhanced Scanner (`scanner.py`)

The scanner now uses Zoekt when available:

1. **Zoekt-first approach**: Tries Zoekt for fast indexed search
2. **Automatic fallback**: Falls back to filesystem traversal if Zoekt unavailable
3. **Dual scan methods**: Returns scan method in results (`"zoekt"` or `"filesystem"`)

```python
scanner = MaestroScanner(service, use_zoekt=True)
results = await scanner.scan_directories(["/home/stan/Prod"])
# results["scan_method"] will be "zoekt" or "filesystem"
```

#### 3. API Endpoints (`dashboard.py`)

New endpoints added:

- `POST /api/v1/search/code` - Fast indexed code search
- `GET /api/v1/search/zoekt/health` - Zoekt availability check

### Frontend Components

#### 1. Code Search Results (`CodeSearchResults.tsx`)

Progressive disclosure component featuring:

- **File-level expansion**: Click to expand/collapse file results
- **Match-level expansion**: Click to show context lines for each match
- **Interactive filters**:
  - Sort by relevance, file path, or repository
  - Filter by repository
- **"Load more" pattern**: Shows first 3 matches, expandable for more
- **Search highlighting**: Highlights matching terms in results

```tsx
<CodeSearchResults
  results={codeResults}
  query={searchQuery}
  total={codeResults.length}
/>
```

#### 2. Enhanced Dashboard (`Dashboard.tsx`)

- **Search mode toggle**: Switch between Memory and Code search
- **Dual search results**: Displays appropriate component based on mode
- **Loading states**: Shows spinner during Zoekt searches

#### 3. Type Definitions (`types/index.ts`)

New TypeScript interfaces:

```typescript
interface LineMatch {
  line_number: number;
  line: string;
  before: string[];
  after: string[];
}

interface CodeSearchResult {
  file_path: string;
  repository: string;
  line_matches: LineMatch[];
  score: number;
}
```

## Zoekt Query Syntax

Zoekt supports a powerful query language:

### Basic Queries

```
# Simple text search
maestro

# Phrase search
"maestro setup"

# AND (implicit)
maestro track

# OR
maestro OR track

# NOT
maestro -test
```

### Field-Specific Searches

```
# File pattern
file:*.md

# Path restriction
path:maestro/

# Content in files
content:product

# Case-sensitive
case:maestro
```

### Combining Operators

```
# Complex query
(file:*.md OR file:*.py) maestro path:/home/stan/Prod/
```

See [Zoekt Query Syntax](https://sourcegraph.com/github.com/sourcegraph/zoekt/-/blob/doc/query_syntax.md) for details.

## Usage

### Setting Up Zoekt

1. **Install Zoekt**:
   ```bash
   go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest
   go install github.com/sourcegraph/zoekt/cmd/zoekt-indexer@latest
   ```

2. **Start Zoekt server**:
   ```bash
   zoekt-webserver -rpc -index ~/.maestro/zoekt_index
   ```

3. **Index your code**:
   ```bash
   zoekt-indexer -index ~/.maestro/zoekt_index -repo_name maestro /home/stan/Prod/maestro
   ```

### Using the API

#### Search Code

```bash
curl -XPOST http://localhost:18765/api/v1/search/code \
  -H "Content-Type: application/json" \
  -d '{
    "query": "maestro product",
    "file_patterns": ["*.md"],
    "max_results": 50,
    "context_lines": 3
  }'
```

#### Check Zoekt Health

```bash
curl http://localhost:18765/api/v1/search/zoekt/health
```

### Using from Python

```python
from maestro.memory.search.zoekt_client import search_maestro_projects

# Find all Maestro projects
projects = await search_maestro_projects("maestro")
for project in projects:
    print(f"Found: {project['path']} ({project['type']})")
```

## Progressive Disclosure Implementation

The frontend implements progressive disclosure at multiple levels:

### 1. Search Summary

Shows high-level information:
- Total results count
- Active filters
- Sort options

### 2. File Results (Expanded by Default)

Shows for each file:
- File path
- Match count
- Repository name
- Relevance score

Click to expand/collapse individual files.

### 3. Line Matches (First 3 Shown)

Shows:
- Line number
- Highlighted matching line

Click to show context lines (before/after).

### 4. Context Lines (Hidden by Default)

Shows:
- Before lines (in gray)
- After lines (in gray)
- Full file context

### 5. "Load More" Pattern

For files with many matches:
- First 3 matches shown immediately
- "+ X more matches" button
- Click to expand all remaining matches

## Configuration

### Environment Variables

```bash
# Zoekt server URL (default: http://127.0.0.1:6070)
export ZOEKT_SERVER_URL="http://localhost:6070"

# Enable/disable Zoekt (default: true)
export MAESTRO_USE_ZOEKT="true"

# Index directory (default: ~/.maestro/zoekt_index)
export ZOEKT_INDEX_DIR="/path/to/index"
```

### Python Configuration

```python
from maestro.memory.search.zoekt_client import ZoektConfig

config = ZoektConfig(
    server_url="http://localhost:6070",
    enabled=True,
    index_dir=Path.home() / ".maestro" / "zoekt_index",
    max_results=100,
    context_lines=3,
)
```

## Performance Considerations

### Zoekt Advantages

1. **Index-based**: Pre-indexed code searches are O(1) vs O(n) for grep
2. **Trigram indexing**: Fast substring matching
3. **Parallel queries**: Can search multiple repos simultaneously
4. **Context retrieval**: Efficient context line retrieval

### Best Practices

1. **Index regularly**: Re-index after code changes
2. **Use specific queries**: More specific = faster results
3. **Limit results**: Use `max_results` to avoid overwhelming responses
4. **Cache queries**: Client-side caching for repeated queries

## Troubleshooting

### Zoekt Not Available

If Zoekt is unavailable, the system automatically falls back to filesystem traversal. Check:

1. Is Zoekt running?
   ```bash
   curl http://localhost:6070/
   ```

2. Is the index up to date?
   ```bash
   zoekt-indexer -index ~/.maestro/zoekt_index /path/to/code
   ```

3. Check health endpoint:
   ```bash
   curl http://localhost:18765/api/v1/search/zoekt/health
   ```

### Performance Issues

1. **Index size**: Large indexes may need more memory
2. **Query complexity**: Complex queries with many ORs are slower
3. **Network latency**: Local Zoekt server recommended

## Future Enhancements

Potential improvements:

1. **Incremental indexing**: Auto-reindex on file changes
2. **Query suggestions**: Autocomplete based on indexed content
3. **Result caching**: Cache frequently-run queries
4. **Multi-repo search**: Search across all indexed repos at once
5. **Regex support**: Use Zoekt's regex capabilities
6. **Symbol search**: Leverage Zoekt's symbol search for definitions

## References

- [Zoekt GitHub Repository](https://github.com/sourcegraph/zoekt)
- [Zoekt JSON API Documentation](https://github.com/sourcegraph/zoekt/blob/main/doc/json-api.md)
- [Zoekt Query Syntax Guide](https://sourcegraph.com/github.com/sourcegraph/zoekt/-/blob/doc/query_syntax.md)
- [Sourcegraph Code Search](https://sourcegraph.com/docs/code-search/queries/language)

## Summary

This integration brings fast, powerful code search to Maestro while maintaining:

- **Backward compatibility**: Works without Zoekt using fallback
- **User experience**: Progressive disclosure prevents information overload
- **Performance**: Indexed searches are significantly faster than traversal
- **Flexibility**: Full Zoekt query syntax available for power users

The result is a professional-grade code search experience that scales to large codebases while remaining responsive and user-friendly.
