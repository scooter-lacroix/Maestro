package profile

import (
	"os"
	"testing"
)

func TestDetectCurrentProfile(t *testing.T) {
	// Save original env vars
	origAgentdeckProfile := os.Getenv("MAESTRO_PROFILE")
	origClaudeConfigDir := os.Getenv("CLAUDE_CONFIG_DIR")
	defer func() {
		if origAgentdeckProfile != "" {
			os.Setenv("MAESTRO_PROFILE", origAgentdeckProfile)
		} else {
			os.Unsetenv("MAESTRO_PROFILE")
		}
		if origClaudeConfigDir != "" {
			os.Setenv("CLAUDE_CONFIG_DIR", origClaudeConfigDir)
		} else {
			os.Unsetenv("CLAUDE_CONFIG_DIR")
		}
	}()

	tests := []struct {
		name              string
		maestroProfile  string
		claudeConfigDir   string
		expectedContains  string // Expected profile (or substring for default case)
	}{
		{
			name:              "explicit MAESTRO_PROFILE takes priority",
			maestroProfile:  "work",
			claudeConfigDir:   "/Users/test/.claude-personal",
			expectedContains:  "work",
		},
		{
			name:              "CLAUDE_CONFIG_DIR .claude-work suffix",
			maestroProfile:  "",
			claudeConfigDir:   "/Users/test/.claude-work",
			expectedContains:  "work",
		},
		{
			name:              "CLAUDE_CONFIG_DIR .claude-personal suffix",
			maestroProfile:  "",
			claudeConfigDir:   "/Users/test/.claude-personal",
			expectedContains:  "personal",
		},
		{
			name:              "CLAUDE_CONFIG_DIR with hyphen pattern",
			maestroProfile:  "",
			claudeConfigDir:   "/opt/claude-production",
			expectedContains:  "production",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Clear env vars
			os.Unsetenv("MAESTRO_PROFILE")
			os.Unsetenv("CLAUDE_CONFIG_DIR")

			// Set test env vars
			if tt.maestroProfile != "" {
				os.Setenv("MAESTRO_PROFILE", tt.maestroProfile)
			}
			if tt.claudeConfigDir != "" {
				os.Setenv("CLAUDE_CONFIG_DIR", tt.claudeConfigDir)
			}

			result := DetectCurrentProfile()
			if result != tt.expectedContains {
				t.Errorf("DetectCurrentProfile() = %q, want %q", result, tt.expectedContains)
			}
		})
	}
}

func TestDetectCurrentProfile_DefaultFallback(t *testing.T) {
	// Save original env vars
	origAgentdeckProfile := os.Getenv("MAESTRO_PROFILE")
	origClaudeConfigDir := os.Getenv("CLAUDE_CONFIG_DIR")
	defer func() {
		if origAgentdeckProfile != "" {
			os.Setenv("MAESTRO_PROFILE", origAgentdeckProfile)
		} else {
			os.Unsetenv("MAESTRO_PROFILE")
		}
		if origClaudeConfigDir != "" {
			os.Setenv("CLAUDE_CONFIG_DIR", origClaudeConfigDir)
		} else {
			os.Unsetenv("CLAUDE_CONFIG_DIR")
		}
	}()

	// Clear all env vars
	os.Unsetenv("MAESTRO_PROFILE")
	os.Unsetenv("CLAUDE_CONFIG_DIR")

	result := DetectCurrentProfile()
	// Should return either the config default or "default"
	if result == "" {
		t.Error("DetectCurrentProfile() should not return empty string")
	}
}
