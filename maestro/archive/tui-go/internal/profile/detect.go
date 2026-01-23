package profile

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/scooter-lacroix/maestro/tui/internal/session"
)

// DetectCurrentProfile attempts to detect the active profile from environment.
// Priority order:
// 1. MAESTRO_PROFILE environment variable (explicit)
// 2. CLAUDE_CONFIG_DIR environment variable (inferred from Claude profile)
// 3. Config default profile
// 4. Fallback to "default"
func DetectCurrentProfile() string {
	// Priority 1: Explicit environment variable
	if profile := os.Getenv("MAESTRO_PROFILE"); profile != "" {
		return profile
	}

	// Priority 2: Parse from CLAUDE_CONFIG_DIR
	// ~/.claude-work/ -> "work"
	// ~/.claude/ -> "default"
	if configDir := os.Getenv("CLAUDE_CONFIG_DIR"); configDir != "" {
		baseName := filepath.Base(configDir)
		// Handle ~/.claude-work -> work, ~/.claude-foo -> foo
		if strings.HasPrefix(baseName, ".claude-") {
			suffix := strings.TrimPrefix(baseName, ".claude-")
			if suffix != "" {
				return suffix
			}
		}
		// Handle standard patterns like claude-work, claude-foo
		if strings.Contains(baseName, "-") {
			parts := strings.Split(baseName, "-")
			if len(parts) > 1 {
				return parts[len(parts)-1]
			}
		}
	}

	// Priority 3 & 4: Delegate to session package's GetEffectiveProfile
	// which handles config default and fallback to "default"
	return session.GetEffectiveProfile("")
}
