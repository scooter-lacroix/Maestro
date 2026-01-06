import { useState } from 'react';
import { CodeSearchResult, LineMatch } from '../types';
import './CodeSearchResults.css';

interface CodeSearchResultsProps {
  results: CodeSearchResult[];
  query: string;
  total: number;
}

export const CodeSearchResults: React.FC<CodeSearchResultsProps> = ({
  results,
  query,
  total,
}) => {
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set());
  const [expandedMatches, setExpandedMatches] = useState<Set<string>>(new Set());
  const [filterRepo, setFilterRepo] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<'score' | 'file' | 'repo'>('score');

  // Get unique repositories for filtering
  const repositories = Array.from(new Set(results.map(r => r.repository)));

  // Filter and sort results
  const filteredResults = results
    .filter(r => !filterRepo || r.repository === filterRepo)
    .sort((a, b) => {
      switch (sortBy) {
        case 'score':
          return b.score - a.score;
        case 'file':
          return a.file_path.localeCompare(b.file_path);
        case 'repo':
          return a.repository.localeCompare(b.repository);
        default:
          return 0;
      }
    });

  const toggleFileExpand = (filePath: string) => {
    setExpandedFiles(prev => {
      const newSet = new Set(prev);
      if (newSet.has(filePath)) {
        newSet.delete(filePath);
      } else {
        newSet.add(filePath);
      }
      return newSet;
    });
  };

  const toggleMatchExpand = (filePath: string, lineNumber: number) => {
    const key = `${filePath}:${lineNumber}`;
    setExpandedMatches(prev => {
      const newSet = new Set(prev);
      if (newSet.has(key)) {
        newSet.delete(key);
      } else {
        newSet.add(key);
      }
      return newSet;
    });
  };

  const highlightMatch = (line: string, query: string) => {
    // Simple highlighting - remove special Zoekt syntax chars
    const cleanQuery = query.replace(/[()|:]/g, ' ').trim().split(/\s+/)[0];
    if (!cleanQuery) return line;

    const regex = new RegExp(`(${cleanQuery})`, 'gi');
    const parts = line.split(regex);

    return parts.map((part, i) =>
      regex.test(part) ? (
        <mark key={i} className="search-highlight">
          {part}
        </mark>
      ) : (
        part
      )
    );
  };

  const renderLineMatch = (result: CodeSearchResult, match: LineMatch) => {
    const key = `${result.file_path}:${match.line_number}`;
    const isExpanded = expandedMatches.has(key);

    return (
      <div key={key} className="line-match">
        {/* Summary line - always visible */}
        <div
          className="line-summary"
          onClick={() => toggleMatchExpand(result.file_path, match.line_number)}
        >
          <div className="line-number">L{match.line_number}</div>
          <div className="line-content">
            {highlightMatch(match.line, query)}
          </div>
          <button className="expand-toggle">
            {isExpanded ? '−' : '+'}
          </button>
        </div>

        {/* Context lines - expandable */}
        {isExpanded && (match.before.length > 0 || match.after.length > 0) && (
          <div className="line-context">
            {match.before.map((line, i) => (
              <div key={`before-${i}`} className="context-line before">
                <span className="context-line-number">L{match.line_number - match.before.length + i}</span>
                <span className="context-line-content">{line}</span>
              </div>
            ))}
            {match.after.map((line, i) => (
              <div key={`after-${i}`} className="context-line after">
                <span className="context-line-number">L{match.line_number + 1 + i}</span>
                <span className="context-line-content">{line}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="code-search-results">
      {/* Search summary and filters */}
      <div className="search-summary">
        <div className="summary-info">
          <h3>Search Results</h3>
          <p className="result-count">
            Found <strong>{total}</strong> result{total !== 1 ? 's' : ''} for "<code>{query}</code>"
          </p>
        </div>

        {/* Interactive filters */}
        <div className="search-filters">
          {/* Sort options */}
          <div className="filter-group">
            <label htmlFor="sort-select">Sort by:</label>
            <select
              id="sort-select"
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as 'score' | 'file' | 'repo')}
              className="sort-select"
            >
              <option value="score">Relevance</option>
              <option value="file">File Path</option>
              <option value="repo">Repository</option>
            </select>
          </div>

          {/* Repository filter */}
          {repositories.length > 1 && (
            <div className="filter-group">
              <label htmlFor="repo-select">Repository:</label>
              <select
                id="repo-select"
                value={filterRepo || 'all'}
                onChange={(e) => setFilterRepo(e.target.value === 'all' ? null : e.target.value)}
                className="repo-select"
              >
                <option value="all">All Repositories</option>
                {repositories.map(repo => (
                  <option key={repo} value={repo}>{repo}</option>
                ))}
              </select>
            </div>
          )}

          {/* Results count */}
          <div className="results-count">
            {filteredResults.length} of {total} shown
          </div>
        </div>
      </div>

      {/* Results list with progressive disclosure */}
      <div className="results-list">
        {filteredResults.length === 0 ? (
          <div className="empty-state">
            <p>No results found</p>
            {filterRepo && (
              <button onClick={() => setFilterRepo(null)} className="clear-filter">
                Clear repository filter
              </button>
            )}
          </div>
        ) : (
          filteredResults.map((result) => {
            const isExpanded = expandedFiles.has(result.file_path);
            const matchCount = result.line_matches.length;

            return (
              <div key={result.file_path} className="file-result">
                {/* File header - always visible, shows summary */}
                <div
                  className="file-header"
                  onClick={() => toggleFileExpand(result.file_path)}
                >
                  <div className="file-info">
                    <h4 className="file-path">
                      <i className="fas fa-file-code" />
                      {result.file_path}
                    </h4>
                    <div className="file-meta">
                      <span className="match-count">{matchCount} match{matchCount !== 1 ? 'es' : ''}</span>
                      <span className="repository">{result.repository}</span>
                      <span className="score">
                        {(result.score * 100).toFixed(0)}% relevant
                      </span>
                    </div>
                  </div>
                  <button className="expand-toggle">
                    {isExpanded ? '−' : '+'}
                  </button>
                </div>

                {/* Line matches - expandable, progressive disclosure */}
                {isExpanded && (
                  <div className="file-matches">
                    {/* Show first 3 matches immediately, rest are expandable too */}
                    {result.line_matches.slice(0, 3).map(match =>
                      renderLineMatch(result, match)
                    )}

                    {/* If more than 3 matches, show expandable "load more" */}
                    {matchCount > 3 && (
                      <div className="more-matches">
                        <button
                          className="load-more-button"
                          onClick={(e) => {
                            e.stopPropagation();
                            // Expand all remaining matches
                            const remainingMatches = result.line_matches.slice(3);
                            remainingMatches.forEach(m =>
                              toggleMatchExpand(result.file_path, m.line_number)
                            );
                          }}
                        >
                          + {matchCount - 3} more match{matchCount - 3 !== 1 ? 'es' : ''}
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
