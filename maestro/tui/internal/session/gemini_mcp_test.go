package session

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestParseGeminiSettings(t *testing.T) {
	// VERIFIED: Actual settings.json structure (simplified)
	settingsJSON := `{
  "security": {
    "auth": {
      "selectedType": "oauth-personal"
    }
  },
  "mcpServers": {
    "exa": {
      "command": "npx",
      "args": ["-y", "exa-mcp-server"],
      "env": {"EXA_API_KEY": "$EXA_API_KEY"}
    },
    "firecrawl": {
      "command": "npx",
      "args": ["-y", "@mendable/firecrawl-mcp"]
    }
  }
}`

	tmpDir := t.TempDir()
	settingsFile := filepath.Join(tmpDir, "settings.json")
	_ = os.WriteFile(settingsFile, []byte(settingsJSON), 0644)

	var config GeminiMCPConfig
	data, _ := os.ReadFile(settingsFile)
	err := json.Unmarshal(data, &config)

	if err != nil {
		t.Fatalf("Failed to parse settings: %v", err)
	}

	if len(config.MCPServers) != 2 {
		t.Errorf("Expected 2 MCP servers, got %d", len(config.MCPServers))
	}

	// VERIFIED: No mcp.allowed/excluded in actual Gemini settings.json
	// (Our research was wrong - actual file doesn't have this)
}

func TestGetGeminiMCPInfo(t *testing.T) {
	tmpDir := t.TempDir()
	// geminiConfigDirOverride replaces the full ~/.gemini path
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	// Settings file is directly under the config dir
	settingsFile := filepath.Join(tmpDir, "settings.json")

	settingsJSON := `{
  "mcpServers": {
    "exa": {"command": "npx", "args": ["-y", "exa-mcp-server"]},
    "firecrawl": {"command": "npx", "args": ["-y", "@mendable/firecrawl-mcp"]}
  }
}`
	_ = os.WriteFile(settingsFile, []byte(settingsJSON), 0644)

	info := GetGeminiMCPInfo("/any/path")

	if len(info.Global) != 2 {
		t.Errorf("Expected 2 global MCPs, got %d: %v", len(info.Global), info.Global)
	}

	if !contains(info.Global, "exa") || !contains(info.Global, "firecrawl") {
		t.Errorf("Expected exa and firecrawl in Global, got %v", info.Global)
	}

	// Gemini has no project or local MCPs
	if len(info.Project) != 0 {
		t.Error("Gemini should have no Project MCPs")
	}
	if len(info.LocalMCPs) != 0 {
		t.Error("Gemini should have no Local MCPs")
	}
}

func TestGetGeminiMCPInfo_NoSettingsFile(t *testing.T) {
	tmpDir := t.TempDir()
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	// Don't create settings.json

	info := GetGeminiMCPInfo("/any/path")

	// Should return empty MCPInfo, not nil
	if info == nil {
		t.Fatal("GetGeminiMCPInfo should return empty MCPInfo, not nil")
	}
	if len(info.Global) != 0 {
		t.Errorf("Expected 0 global MCPs when no settings file, got %d", len(info.Global))
	}
}

func TestGetGeminiMCPInfo_EmptyMCPServers(t *testing.T) {
	tmpDir := t.TempDir()
	// geminiConfigDirOverride replaces the full ~/.gemini path
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	// Settings file is directly under the config dir
	settingsFile := filepath.Join(tmpDir, "settings.json")

	// Settings file with no mcpServers
	settingsJSON := `{"security": {"auth": {"selectedType": "oauth"}}}`
	_ = os.WriteFile(settingsFile, []byte(settingsJSON), 0644)

	info := GetGeminiMCPInfo("/any/path")

	if len(info.Global) != 0 {
		t.Errorf("Expected 0 global MCPs when mcpServers empty, got %d", len(info.Global))
	}
}

func TestWriteGeminiMCPSettings(t *testing.T) {
	tmpDir := t.TempDir()
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	// Clear user config cache to avoid interference
	userConfigCacheMu.Lock()
	userConfigCache = nil
	userConfigCacheMu.Unlock()

	settingsFile := filepath.Join(tmpDir, "settings.json")

	// Create initial settings with other config
	initialSettings := `{"security": {"auth": {"selectedType": "oauth"}}}`
	_ = os.WriteFile(settingsFile, []byte(initialSettings), 0644)

	// WriteGeminiMCPSettings requires MCPs in config.toml
	// For this test, we'll just test with empty MCPs (valid case)
	err := WriteGeminiMCPSettings([]string{})
	if err != nil {
		t.Fatalf("WriteGeminiMCPSettings() error = %v", err)
	}

	// Verify settings file updated
	data, _ := os.ReadFile(settingsFile)
	var config map[string]interface{}
	_ = json.Unmarshal(data, &config)

	// Should preserve security section
	if config["security"] == nil {
		t.Error("Should preserve existing security config")
	}

	// Should have mcpServers (empty)
	if config["mcpServers"] == nil {
		t.Error("Should have mcpServers section")
	}
}

func TestWriteGeminiMCPSettings_PreservesExistingConfig(t *testing.T) {
	tmpDir := t.TempDir()
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	// Clear user config cache
	userConfigCacheMu.Lock()
	userConfigCache = nil
	userConfigCacheMu.Unlock()

	settingsFile := filepath.Join(tmpDir, "settings.json")

	// Create settings with multiple sections
	initialSettings := `{
  "security": {"auth": {"selectedType": "oauth"}},
  "theme": "dark",
  "language": "en",
  "mcpServers": {"old-mcp": {"command": "test"}}
}`
	_ = os.WriteFile(settingsFile, []byte(initialSettings), 0644)

	err := WriteGeminiMCPSettings([]string{})
	if err != nil {
		t.Fatalf("WriteGeminiMCPSettings() error = %v", err)
	}

	data, _ := os.ReadFile(settingsFile)
	var config map[string]interface{}
	_ = json.Unmarshal(data, &config)

	// Should preserve all non-mcpServers fields
	if config["security"] == nil {
		t.Error("Should preserve security")
	}
	if config["theme"] != "dark" {
		t.Error("Should preserve theme")
	}
	if config["language"] != "en" {
		t.Error("Should preserve language")
	}

	// mcpServers should be replaced (empty since we passed empty slice)
	mcpServers, ok := config["mcpServers"].(map[string]interface{})
	if !ok {
		t.Fatal("mcpServers should be a map")
	}
	if len(mcpServers) != 0 {
		t.Errorf("mcpServers should be empty (we passed empty slice), got %d MCPs", len(mcpServers))
	}
}

func TestWriteGeminiMCPSettings_CreatesFile(t *testing.T) {
	tmpDir := t.TempDir()
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	// Clear user config cache
	userConfigCacheMu.Lock()
	userConfigCache = nil
	userConfigCacheMu.Unlock()

	// Don't create settings.json - test that WriteGeminiMCPSettings creates it
	settingsFile := filepath.Join(tmpDir, "settings.json")

	err := WriteGeminiMCPSettings([]string{})
	if err != nil {
		t.Fatalf("WriteGeminiMCPSettings() error = %v", err)
	}

	// File should be created
	if _, err := os.Stat(settingsFile); os.IsNotExist(err) {
		t.Error("settings.json should be created")
	}

	data, _ := os.ReadFile(settingsFile)
	var config map[string]interface{}
	_ = json.Unmarshal(data, &config)

	// Should have mcpServers
	if config["mcpServers"] == nil {
		t.Error("Should have mcpServers section")
	}
}

func TestGetGeminiMCPNames(t *testing.T) {
	tmpDir := t.TempDir()
	geminiConfigDirOverride = tmpDir
	defer func() { geminiConfigDirOverride = "" }()

	settingsFile := filepath.Join(tmpDir, "settings.json")
	settingsJSON := `{
  "mcpServers": {
    "zeta": {"command": "npx"},
    "alpha": {"command": "npx"},
    "beta": {"command": "npx"}
  }
}`
	_ = os.WriteFile(settingsFile, []byte(settingsJSON), 0644)

	names := GetGeminiMCPNames()

	if len(names) != 3 {
		t.Errorf("Expected 3 names, got %d", len(names))
	}

	// Should be sorted
	if names[0] != "alpha" || names[1] != "beta" || names[2] != "zeta" {
		t.Errorf("Names should be sorted, got %v", names)
	}
}
