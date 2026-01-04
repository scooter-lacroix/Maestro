package session

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"io"
	"log"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/fsnotify/fsnotify"
	"github.com/sahilm/fuzzy"
	"golang.org/x/time/rate"
)

// SearchTier represents the search strategy tier
type SearchTier int

const (
	TierInstant  SearchTier = iota // < 100MB, full in-memory
	TierBalanced                   // 100MB-500MB, LRU cache
)

// TierThresholdInstant is the max size for instant tier (100MB)
const TierThresholdInstant = 100 * 1024 * 1024

// TierThresholdBalanced is the max size for balanced tier (500MB)
const TierThresholdBalanced = 500 * 1024 * 1024

// SearchEntry represents a searchable Claude session
type SearchEntry struct {
	SessionID    string    // Claude session UUID
	FilePath     string    // Path to .jsonl file
	CWD          string    // Project working directory
	Content      string    // Full conversation content (original case)
	ContentLower string    // Lowercased for search
	Summary      string    // First user message or summary
	ModTime      time.Time // File modification time
	FileSize     int64     // File size in bytes
}

// MatchRange represents a match position in content
type MatchRange struct {
	Start int
	End   int
}

// Match searches for query in entry content (case-insensitive)
// Returns match positions for highlighting
func (e *SearchEntry) Match(query string) []MatchRange {
	queryLower := strings.ToLower(query)
	var matches []MatchRange

	start := 0
	for {
		idx := strings.Index(e.ContentLower[start:], queryLower)
		if idx == -1 {
			break
		}
		absIdx := start + idx
		matches = append(matches, MatchRange{
			Start: absIdx,
			End:   absIdx + len(query),
		})
		start = absIdx + len(query)
	}

	return matches
}

// GetSnippet extracts a context window around the first match
// Uses rune-based indexing to safely handle UTF-8 content
// Optimized: Single rune conversion instead of triple conversion
func (e *SearchEntry) GetSnippet(query string, windowSize int) string {
	matches := e.Match(query)
	runes := []rune(e.Content)

	if len(matches) == 0 {
		// No match, return beginning of content
		if len(runes) > windowSize*2 {
			return string(runes[:windowSize*2]) + "..."
		}
		return e.Content
	}

	match := matches[0]
	// Convert byte indices to rune indices efficiently (single pass)
	// Count runes up to each byte position without creating substring copies
	runeStart := byteIndexToRuneIndex(e.Content, match.Start)
	runeEnd := byteIndexToRuneIndex(e.Content, match.End)

	start := runeStart - windowSize
	if start < 0 {
		start = 0
	}
	end := runeEnd + windowSize
	if end > len(runes) {
		end = len(runes)
	}

	// Expand to word boundaries using runes
	for start > 0 && runes[start-1] != ' ' && runes[start-1] != '\n' {
		start--
	}
	for end < len(runes) && runes[end] != ' ' && runes[end] != '\n' {
		end++
	}

	snippet := string(runes[start:end])
	prefix := ""
	suffix := ""
	if start > 0 {
		prefix = "..."
	}
	if end < len(runes) {
		suffix = "..."
	}

	return prefix + strings.TrimSpace(snippet) + suffix
}

// byteIndexToRuneIndex converts a byte index to a rune index efficiently
// This avoids creating substring copies which was causing O(n) allocations
func byteIndexToRuneIndex(s string, byteIdx int) int {
	if byteIdx <= 0 {
		return 0
	}
	if byteIdx >= len(s) {
		return len([]rune(s))
	}
	// Count runes up to the byte index without creating a substring
	runeCount := 0
	for i := range s {
		if i >= byteIdx {
			break
		}
		runeCount++
	}
	return runeCount
}

// claudeJSONLRecord represents a single line in Claude's JSONL files
type claudeJSONLRecord struct {
	SessionID string          `json:"sessionId"`
	Type      string          `json:"type"`
	Message   json.RawMessage `json:"message"`
	Timestamp string          `json:"timestamp"`
	CWD       string          `json:"cwd"`
	Summary   string          `json:"summary"`
}

// claudeMessage represents the message field in a record
type claudeMessage struct {
	Role    string          `json:"role"`
	Content json.RawMessage `json:"content"`
}

// parseClaudeJSONL parses a Claude JSONL file into a SearchEntry
func parseClaudeJSONL(filePath string, data []byte) (*SearchEntry, error) {
	entry := &SearchEntry{
		FilePath: filePath,
	}

	var contentBuilder strings.Builder
	scanner := bufio.NewScanner(bytes.NewReader(data))
	// Handle large lines
	buf := make([]byte, 0, 64*1024)
	scanner.Buffer(buf, 1024*1024)

	for scanner.Scan() {
		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}

		var record claudeJSONLRecord
		if err := json.Unmarshal(line, &record); err != nil {
			continue // Skip malformed lines
		}

		// Extract session ID from first valid record
		if entry.SessionID == "" && record.SessionID != "" {
			entry.SessionID = record.SessionID
		}

		// Extract CWD
		if entry.CWD == "" && record.CWD != "" {
			entry.CWD = record.CWD
		}

		// Extract summary if available
		if entry.Summary == "" && record.Summary != "" {
			entry.Summary = record.Summary
		}

		// Extract message content with role prefix
		if len(record.Message) > 0 {
			var msg claudeMessage
			if err := json.Unmarshal(record.Message, &msg); err == nil {
				// Determine role prefix for better preview display
				var rolePrefix string
				switch msg.Role {
				case "user":
					rolePrefix = "User: "
				case "assistant":
					rolePrefix = "Assistant: "
				default:
					rolePrefix = ""
				}

				// Content can be string or array
				var contentStr string
				if err := json.Unmarshal(msg.Content, &contentStr); err == nil {
					if rolePrefix != "" && contentStr != "" {
						contentBuilder.WriteString(rolePrefix)
					}
					contentBuilder.WriteString(contentStr)
					contentBuilder.WriteString("\n")
				} else {
					// Try as array of content blocks
					var blocks []map[string]interface{}
					if err := json.Unmarshal(msg.Content, &blocks); err == nil {
						for i, block := range blocks {
							if text, ok := block["text"].(string); ok {
								// Only add prefix to first block of message
								if i == 0 && rolePrefix != "" && text != "" {
									contentBuilder.WriteString(rolePrefix)
								}
								contentBuilder.WriteString(text)
								contentBuilder.WriteString("\n")
							}
						}
					}
				}
			}
		}

		// Use first user message as summary if no summary field
		if entry.Summary == "" && record.Type == "user" && len(record.Message) > 0 {
			var msg claudeMessage
			if err := json.Unmarshal(record.Message, &msg); err == nil {
				var contentStr string
				if err := json.Unmarshal(msg.Content, &contentStr); err == nil {
					if len(contentStr) > 200 {
						entry.Summary = contentStr[:200] + "..."
					} else {
						entry.Summary = contentStr
					}
				}
			}
		}
	}

	entry.Content = contentBuilder.String()
	entry.ContentLower = strings.ToLower(entry.Content)

	return entry, nil
}

// DetectTier determines the appropriate search tier based on data size
func DetectTier(totalSize int64) SearchTier {
	if totalSize < TierThresholdInstant {
		return TierInstant
	}
	return TierBalanced
}

// TierName returns a human-readable name for the tier
func TierName(tier SearchTier) string {
	switch tier {
	case TierInstant:
		return "instant"
	case TierBalanced:
		return "balanced"
	default:
		return "unknown"
	}
}

// SearchResult represents a search result with match info
type SearchResult struct {
	Entry   *SearchEntry
	Matches []MatchRange
	Score   int
	Snippet string
}

// GlobalSearchIndex manages the searchable session index
type GlobalSearchIndex struct {
	// Configuration
	config    GlobalSearchSettings
	claudeDir string

	// Index data (protected by atomic pointer for lock-free reads)
	entries atomic.Pointer[[]SearchEntry]

	// File tracking for incremental updates
	fileTrackers map[string]*FileTracker
	trackerMu    sync.RWMutex

	// File watcher
	watcher *fsnotify.Watcher

	// Rate limiter for background indexing
	limiter *rate.Limiter

	// Tier
	tier SearchTier

	// Loading state
	loading atomic.Bool

	// Lifecycle
	ctx    context.Context
	cancel context.CancelFunc
	wg     sync.WaitGroup
}

// FileTracker tracks file state for incremental updates
type FileTracker struct {
	Path       string
	LastOffset int64
	LastSize   int64
	LastMod    time.Time
}

// NewGlobalSearchIndex creates a new search index
func NewGlobalSearchIndex(claudeDir string, config GlobalSearchSettings) (*GlobalSearchIndex, error) {
	if !config.Enabled {
		return nil, nil
	}

	// Apply defaults if not set
	if config.IndexRateLimit == 0 {
		config.IndexRateLimit = 20
	}

	ctx, cancel := context.WithCancel(context.Background())

	idx := &GlobalSearchIndex{
		config:       config,
		claudeDir:    claudeDir,
		fileTrackers: make(map[string]*FileTracker),
		limiter:      rate.NewLimiter(rate.Limit(config.IndexRateLimit), 5),
		ctx:          ctx,
		cancel:       cancel,
	}

	// Initialize empty entries
	emptyEntries := make([]SearchEntry, 0)
	idx.entries.Store(&emptyEntries)

	// Measure data size and determine tier
	projectsDir := filepath.Join(claudeDir, "projects")
	totalSize, err := measureDataSize(projectsDir, config.RecentDays)
	if err != nil {
		// Don't fail if projects dir doesn't exist, just use empty index
		if !os.IsNotExist(err) {
			cancel()
			return nil, err
		}
	}

	// Determine tier (respect config override)
	switch config.Tier {
	case "instant":
		idx.tier = TierInstant
	case "balanced":
		idx.tier = TierBalanced
	case "disabled":
		cancel()
		return nil, nil
	default:
		idx.tier = DetectTier(totalSize)
	}

	// Start file watcher
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		cancel()
		return nil, err
	}
	idx.watcher = watcher

	// Watch the projects directory (create if doesn't exist check)
	if _, err := os.Stat(projectsDir); err == nil {
		if err := watcher.Add(projectsDir); err != nil {
			log.Printf("GlobalSearch: failed to watch projects dir: %v", err)
		}

		// Also watch subdirectories
		_ = filepath.WalkDir(projectsDir, func(path string, d os.DirEntry, err error) error {
			if err != nil || !d.IsDir() {
				return nil
			}
			_ = watcher.Add(path) // Ignore error - best effort watching
			return nil
		})
	}

	// Set loading state
	idx.loading.Store(true)

	// Start background workers
	idx.wg.Add(2)
	go idx.watcherLoop()
	go idx.initialLoad()

	return idx, nil
}

// measureDataSize calculates total size of JSONL files
func measureDataSize(projectsDir string, recentDays int) (int64, error) {
	var totalSize int64
	cutoff := time.Time{}
	if recentDays > 0 {
		cutoff = time.Now().AddDate(0, 0, -recentDays)
	}

	err := filepath.WalkDir(projectsDir, func(path string, d os.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return nil
		}
		if !strings.HasSuffix(path, ".jsonl") {
			return nil
		}
		info, err := d.Info()
		if err != nil {
			return nil
		}
		if !cutoff.IsZero() && info.ModTime().Before(cutoff) {
			return nil
		}
		totalSize += info.Size()
		return nil
	})

	return totalSize, err
}

// initialLoad loads all session files on startup
func (idx *GlobalSearchIndex) initialLoad() {
	defer idx.wg.Done()

	projectsDir := filepath.Join(idx.claudeDir, "projects")
	cutoff := time.Time{}
	if idx.config.RecentDays > 0 {
		cutoff = time.Now().AddDate(0, 0, -idx.config.RecentDays)
	}

	var entries []SearchEntry

	_ = filepath.WalkDir(projectsDir, func(path string, d os.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return nil
		}
		if !strings.HasSuffix(path, ".jsonl") {
			return nil
		}

		// Check cancellation
		select {
		case <-idx.ctx.Done():
			return filepath.SkipAll
		default:
		}

		// Rate limit
		_ = idx.limiter.Wait(idx.ctx)

		info, err := d.Info()
		if err != nil {
			return nil
		}

		// Check recency
		if !cutoff.IsZero() && info.ModTime().Before(cutoff) {
			return nil
		}

		// Only UUID-named files (skip agent-*.jsonl)
		baseName := filepath.Base(path)
		if !isUUIDFileName(baseName) {
			return nil
		}

		// Parse file
		data, err := os.ReadFile(path)
		if err != nil {
			return nil
		}

		entry, err := parseClaudeJSONL(path, data)
		if err != nil || entry.SessionID == "" {
			return nil
		}

		entry.ModTime = info.ModTime()
		entry.FileSize = info.Size()
		entries = append(entries, *entry)

		// Track file for incremental updates
		idx.trackerMu.Lock()
		idx.fileTrackers[path] = &FileTracker{
			Path:       path,
			LastOffset: info.Size(),
			LastSize:   info.Size(),
			LastMod:    info.ModTime(),
		}
		idx.trackerMu.Unlock()

		return nil
	})

	// Store entries and mark loading complete
	idx.entries.Store(&entries)
	idx.loading.Store(false)
}

// isUUIDFileName checks if filename matches UUID pattern
var uuidFilePattern = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\.jsonl$`)

func isUUIDFileName(name string) bool {
	return uuidFilePattern.MatchString(name)
}

// watcherLoop handles file system events
func (idx *GlobalSearchIndex) watcherLoop() {
	defer idx.wg.Done()

	// Debounce map
	debounce := make(map[string]*time.Timer)
	debounceMu := sync.Mutex{}

	for {
		select {
		case <-idx.ctx.Done():
			return
		case event, ok := <-idx.watcher.Events:
			if !ok {
				return
			}

			// Only care about writes and creates for .jsonl files
			if !strings.HasSuffix(event.Name, ".jsonl") {
				continue
			}
			if event.Op&(fsnotify.Write|fsnotify.Create) == 0 {
				continue
			}

			// Debounce: wait 300ms after last event for this file
			debounceMu.Lock()
			if timer, exists := debounce[event.Name]; exists {
				timer.Stop()
			}
			debounce[event.Name] = time.AfterFunc(300*time.Millisecond, func() {
				idx.updateFile(event.Name)
				debounceMu.Lock()
				delete(debounce, event.Name)
				debounceMu.Unlock()
			})
			debounceMu.Unlock()

		case err, ok := <-idx.watcher.Errors:
			if !ok {
				return
			}
			log.Printf("GlobalSearch watcher error: %v", err)
		}
	}
}

// updateFile handles incremental update for a single file
func (idx *GlobalSearchIndex) updateFile(path string) {
	if !isUUIDFileName(filepath.Base(path)) {
		return
	}

	info, err := os.Stat(path)
	if err != nil {
		return // File deleted, ignore for now
	}

	idx.trackerMu.RLock()
	tracker, exists := idx.fileTrackers[path]
	idx.trackerMu.RUnlock()

	if exists && info.Size() < tracker.LastSize {
		// File was truncated/replaced, do full reload of this file
		tracker = nil
	}

	// Read file (or just new portion for append-only)
	var data []byte
	if tracker != nil && info.Size() > tracker.LastOffset {
		// Incremental read
		f, err := os.Open(path)
		if err != nil {
			return
		}
		defer f.Close()
		_, _ = f.Seek(tracker.LastOffset, 0)
		data, _ = io.ReadAll(f)
	} else {
		// Full read
		data, _ = os.ReadFile(path)
	}

	if len(data) == 0 {
		return
	}

	// Parse and update
	entry, err := parseClaudeJSONL(path, data)
	if err != nil || entry.SessionID == "" {
		return
	}
	entry.ModTime = info.ModTime()
	entry.FileSize = info.Size()

	// Update entries atomically
	oldEntries := idx.entries.Load()
	newEntries := make([]SearchEntry, 0, len(*oldEntries)+1)

	found := false
	for _, e := range *oldEntries {
		if e.FilePath == path {
			// Merge content for incremental update
			if tracker != nil {
				entry.Content = e.Content + entry.Content
				entry.ContentLower = strings.ToLower(entry.Content)
			}
			newEntries = append(newEntries, *entry)
			found = true
		} else {
			newEntries = append(newEntries, e)
		}
	}
	if !found {
		newEntries = append(newEntries, *entry)
	}

	idx.entries.Store(&newEntries)

	// Update tracker
	idx.trackerMu.Lock()
	idx.fileTrackers[path] = &FileTracker{
		Path:       path,
		LastOffset: info.Size(),
		LastSize:   info.Size(),
		LastMod:    info.ModTime(),
	}
	idx.trackerMu.Unlock()
}

// Search performs a simple substring search
func (idx *GlobalSearchIndex) Search(query string) []*SearchResult {
	if query == "" {
		return nil
	}

	entries := idx.entries.Load()
	if entries == nil {
		return nil
	}

	queryLower := strings.ToLower(query)
	var results []*SearchResult

	for i := range *entries {
		entry := &(*entries)[i]
		if strings.Contains(entry.ContentLower, queryLower) {
			matches := entry.Match(query)
			results = append(results, &SearchResult{
				Entry:   entry,
				Matches: matches,
				Score:   len(matches) * 10,
				Snippet: entry.GetSnippet(query, 60),
			})
		}
	}

	// Sort by score (more matches = higher score) - O(n log n)
	sort.Slice(results, func(i, j int) bool {
		return results[i].Score > results[j].Score
	})

	return results
}

// fuzzySearchSource implements fuzzy.Source for our entries
type fuzzySearchSource struct {
	entries *[]SearchEntry
}

func (s fuzzySearchSource) String(i int) string {
	entry := &(*s.entries)[i]
	// Use summary + first part of content for fuzzy matching
	contentPreview := entry.Content
	if len(contentPreview) > 500 {
		contentPreview = contentPreview[:500]
	}
	return entry.Summary + " " + contentPreview
}

func (s fuzzySearchSource) Len() int {
	return len(*s.entries)
}

// FuzzySearch performs fuzzy matching with typo tolerance
func (idx *GlobalSearchIndex) FuzzySearch(query string) []*SearchResult {
	if query == "" {
		return nil
	}

	entries := idx.entries.Load()
	if entries == nil {
		return nil
	}

	// Create fuzzy source
	source := fuzzySearchSource{entries: entries}

	// Fuzzy match
	matches := fuzzy.FindFrom(query, source)

	var results []*SearchResult
	for _, match := range matches {
		entry := &(*entries)[match.Index]
		results = append(results, &SearchResult{
			Entry:   entry,
			Score:   match.Score,
			Snippet: entry.GetSnippet(query, 60),
		})
	}

	return results
}

// GetTier returns the current search tier
func (idx *GlobalSearchIndex) GetTier() SearchTier {
	return idx.tier
}

// EntryCount returns the number of indexed entries
func (idx *GlobalSearchIndex) EntryCount() int {
	entries := idx.entries.Load()
	if entries == nil {
		return 0
	}
	return len(*entries)
}

// IsLoading returns true if the index is still loading
func (idx *GlobalSearchIndex) IsLoading() bool {
	return idx.loading.Load()
}

// Close shuts down the index
func (idx *GlobalSearchIndex) Close() {
	idx.cancel()
	if idx.watcher != nil {
		idx.watcher.Close()
	}
	idx.wg.Wait()
}
