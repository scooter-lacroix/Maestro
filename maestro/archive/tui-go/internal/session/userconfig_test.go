package session

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/BurntSushi/toml"
)

func TestUserConfig_ClaudeConfigDir(t *testing.T) {
	// Create temp config file
	tmpDir := t.TempDir()
	configContent := `
[claude]
config_dir = "~/.claude-work"

[tools.test]
command = "test"
`
	configPath := filepath.Join(tmpDir, "config.toml")
	if err := os.WriteFile(configPath, []byte(configContent), 0600); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	// Test parsing
	var config UserConfig
	_, err := toml.DecodeFile(configPath, &config)
	if err != nil {
		t.Fatalf("Failed to decode: %v", err)
	}

	if config.Claude.ConfigDir != "~/.claude-work" {
		t.Errorf("Claude.ConfigDir = %s, want ~/.claude-work", config.Claude.ConfigDir)
	}
}

func TestUserConfig_ClaudeConfigDirEmpty(t *testing.T) {
	// Test with no Claude section
	tmpDir := t.TempDir()
	configContent := `
[tools.test]
command = "test"
`
	configPath := filepath.Join(tmpDir, "config.toml")
	if err := os.WriteFile(configPath, []byte(configContent), 0600); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	var config UserConfig
	_, err := toml.DecodeFile(configPath, &config)
	if err != nil {
		t.Fatalf("Failed to decode: %v", err)
	}

	if config.Claude.ConfigDir != "" {
		t.Errorf("Claude.ConfigDir = %s, want empty string", config.Claude.ConfigDir)
	}
}

func TestGlobalSearchConfig(t *testing.T) {
	// Create temp config with global search settings
	tmpDir := t.TempDir()
	configContent := `
[global_search]
enabled = true
tier = "auto"
memory_limit_mb = 150
recent_days = 60
index_rate_limit = 30
`
	configPath := filepath.Join(tmpDir, "config.toml")
	if err := os.WriteFile(configPath, []byte(configContent), 0600); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	// Test parsing
	var config UserConfig
	_, err := toml.DecodeFile(configPath, &config)
	if err != nil {
		t.Fatalf("Failed to decode: %v", err)
	}

	if !config.GlobalSearch.Enabled {
		t.Error("Expected GlobalSearch.Enabled to be true")
	}
	if config.GlobalSearch.Tier != "auto" {
		t.Errorf("Expected tier 'auto', got %q", config.GlobalSearch.Tier)
	}
	if config.GlobalSearch.MemoryLimitMB != 150 {
		t.Errorf("Expected MemoryLimitMB 150, got %d", config.GlobalSearch.MemoryLimitMB)
	}
	if config.GlobalSearch.RecentDays != 60 {
		t.Errorf("Expected RecentDays 60, got %d", config.GlobalSearch.RecentDays)
	}
	if config.GlobalSearch.IndexRateLimit != 30 {
		t.Errorf("Expected IndexRateLimit 30, got %d", config.GlobalSearch.IndexRateLimit)
	}
}

func TestGlobalSearchConfigDefaults(t *testing.T) {
	// Config without global_search section should parse with zero values
	// (defaults are applied by LoadUserConfig, not parsing)
	tmpDir := t.TempDir()
	configContent := `default_tool = "claude"`
	configPath := filepath.Join(tmpDir, "config.toml")
	if err := os.WriteFile(configPath, []byte(configContent), 0600); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	var config UserConfig
	_, err := toml.DecodeFile(configPath, &config)
	if err != nil {
		t.Fatalf("Failed to decode: %v", err)
	}

	// When parsing directly without LoadUserConfig, values should be zero
	if config.GlobalSearch.Enabled {
		t.Error("GlobalSearch.Enabled should be false when not specified (zero value)")
	}
	if config.GlobalSearch.MemoryLimitMB != 0 {
		t.Errorf("Expected default MemoryLimitMB 0 (zero value), got %d", config.GlobalSearch.MemoryLimitMB)
	}
}

func TestGlobalSearchConfigDisabled(t *testing.T) {
	// Test explicitly disabling global search
	tmpDir := t.TempDir()
	configContent := `
[global_search]
enabled = false
tier = "disabled"
`
	configPath := filepath.Join(tmpDir, "config.toml")
	if err := os.WriteFile(configPath, []byte(configContent), 0600); err != nil {
		t.Fatalf("Failed to write config file: %v", err)
	}

	var config UserConfig
	_, err := toml.DecodeFile(configPath, &config)
	if err != nil {
		t.Fatalf("Failed to decode: %v", err)
	}

	if config.GlobalSearch.Enabled {
		t.Error("Expected GlobalSearch.Enabled to be false")
	}
	if config.GlobalSearch.Tier != "disabled" {
		t.Errorf("Expected tier 'disabled', got %q", config.GlobalSearch.Tier)
	}
}
