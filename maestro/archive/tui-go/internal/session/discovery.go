package session

import (
	"path/filepath"
	"strings"

	"github.com/scooter-lacroix/maestro/tui/internal/tmux"
)

// DiscoverExistingTmuxSessions finds all tmux sessions and converts them to instances
func DiscoverExistingTmuxSessions(existingInstances []*Instance) ([]*Instance, error) {
	// Get all tmux sessions
	tmuxSessions, err := tmux.DiscoverAllTmuxSessions()
	if err != nil {
		return nil, err
	}

	// Build a map of existing sessions by tmux name
	existingMap := make(map[string]bool)
	for _, inst := range existingInstances {
		if inst.GetTmuxSession() != nil {
			existingMap[inst.GetTmuxSession().Name] = true
		}
		// Also track by title
		existingMap[inst.Title] = true
	}

	var discovered []*Instance
	for _, sess := range tmuxSessions {
		// Skip if already tracked
		if existingMap[sess.Name] || existingMap[sess.DisplayName] {
			continue
		}

		// Skip maestro-tui sessions (they should already be tracked)
		if strings.HasPrefix(sess.Name, tmux.SessionPrefix) {
			continue
		}

		// Create instance for discovered session
		title := sess.DisplayName
		projectPath := sess.WorkDir
		if projectPath == "" {
			projectPath = "~"
		}

		// Enable mouse mode for proper scrolling in imported sessions
		// Ignore errors - non-fatal, older tmux versions may not support all options
		_ = sess.EnableMouseMode()

		inst := &Instance{
			ID:          generateID(),
			Title:       title,
			ProjectPath: projectPath,
			Status:      StatusIdle,
			Tool:        detectToolFromName(title),
			tmuxSession: sess,
		}
		_ = inst.UpdateStatus()
		discovered = append(discovered, inst)
	}

	return discovered, nil
}

// GroupByProject groups sessions by their parent project directory
func GroupByProject(instances []*Instance) map[string][]*Instance {
	groups := make(map[string][]*Instance)

	for _, inst := range instances {
		// Extract parent directory name
		projectName := extractProjectName(inst.ProjectPath)
		groups[projectName] = append(groups[projectName], inst)
	}

	return groups
}

// FilterByQuery filters sessions by title, project path, tool, or status
// Supports status filters: "waiting", "running", "idle", "error"
func FilterByQuery(instances []*Instance, query string) []*Instance {
	if query == "" {
		return instances
	}

	query = strings.ToLower(strings.TrimSpace(query))

	// Check for status filters
	statusFilters := map[string]Status{
		"waiting": StatusWaiting,
		"running": StatusRunning,
		"idle":    StatusIdle,
		"error":   StatusError,
	}

	// If query matches a status filter exactly, filter by status
	if status, ok := statusFilters[query]; ok {
		return filterByStatus(instances, status)
	}

	// Regular fuzzy search on title, path, tool
	filtered := make([]*Instance, 0)

	for _, inst := range instances {
		if strings.Contains(strings.ToLower(inst.Title), query) ||
			strings.Contains(strings.ToLower(inst.ProjectPath), query) ||
			strings.Contains(strings.ToLower(inst.Tool), query) {
			filtered = append(filtered, inst)
		}
	}

	return filtered
}

// filterByStatus returns only instances with the specified status
func filterByStatus(instances []*Instance, status Status) []*Instance {
	filtered := make([]*Instance, 0)
	for _, inst := range instances {
		if inst.Status == status {
			filtered = append(filtered, inst)
		}
	}
	return filtered
}

// detectToolFromName tries to detect tool type from session name
func detectToolFromName(name string) string {
	nameLower := strings.ToLower(name)

	if strings.Contains(nameLower, "claude") {
		return "claude"
	}
	if strings.Contains(nameLower, "gemini") {
		return "gemini"
	}
	if strings.Contains(nameLower, "opencode") || strings.Contains(nameLower, "open-code") {
		return "opencode"
	}
	if strings.Contains(nameLower, "codex") {
		return "codex"
	}

	return "shell"
}

// extractProjectName extracts the parent directory name from a path
func extractProjectName(projectPath string) string {
	// Clean the path and split into parts
	cleanPath := filepath.Clean(projectPath)
	parts := strings.Split(cleanPath, string(filepath.Separator))

	// Filter out empty parts
	var filteredParts []string
	for _, part := range parts {
		if part != "" {
			filteredParts = append(filteredParts, part)
		}
	}

	// For /home/user/projects/devops, we want "projects" (second-to-last)
	if len(filteredParts) >= 2 {
		return filteredParts[len(filteredParts)-2]
	}

	// Fallback to the last part
	if len(filteredParts) > 0 {
		return filteredParts[len(filteredParts)-1]
	}

	return "unknown"
}
