package session

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// BenchmarkGetMCPInfo_NoMCPJson benchmarks the case with no .mcp.json (worst case - walks to root)
func BenchmarkGetMCPInfo_NoMCPJson(b *testing.B) {
	// Create deep directory structure without any .mcp.json
	tmpDir, err := os.MkdirTemp("", "mcp-bench-*")
	if err != nil {
		b.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	// Create a deep path (10 levels)
	deepPath := tmpDir
	for i := 0; i < 10; i++ {
		deepPath = filepath.Join(deepPath, "subdir")
	}
	if err := os.MkdirAll(deepPath, 0755); err != nil {
		b.Fatalf("Failed to create deep path: %v", err)
	}

	// Setup minimal Claude config
	oldConfigDir := os.Getenv("CLAUDE_CONFIG_DIR")
	tmpClaudeConfig, _ := os.MkdirTemp("", "claude-config-*")
	defer os.RemoveAll(tmpClaudeConfig)
	os.Setenv("CLAUDE_CONFIG_DIR", tmpClaudeConfig)
	defer func() {
		if oldConfigDir != "" {
			os.Setenv("CLAUDE_CONFIG_DIR", oldConfigDir)
		} else {
			os.Unsetenv("CLAUDE_CONFIG_DIR")
		}
	}()
	claudeConfig := map[string]interface{}{"mcpServers": map[string]interface{}{}}
	claudeData, _ := json.MarshalIndent(claudeConfig, "", "  ")
	_ = os.WriteFile(filepath.Join(tmpClaudeConfig, ".claude.json"), claudeData, 0600)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		GetMCPInfo(deepPath)
	}
}

// BenchmarkGetMCPInfo_WithMCPJson benchmarks the case with .mcp.json (best case - immediate find)
func BenchmarkGetMCPInfo_WithMCPJson(b *testing.B) {
	tmpDir, err := os.MkdirTemp("", "mcp-bench-*")
	if err != nil {
		b.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	// Write .mcp.json with a few MCPs
	mcpConfig := map[string]interface{}{
		"mcpServers": map[string]interface{}{
			"exa": map[string]interface{}{
				"command": "npx",
				"args":    []string{"-y", "exa-mcp-server"},
			},
			"firecrawl": map[string]interface{}{
				"command": "npx",
				"args":    []string{"-y", "firecrawl-mcp"},
			},
		},
	}
	mcpData, _ := json.MarshalIndent(mcpConfig, "", "  ")
	_ = os.WriteFile(filepath.Join(tmpDir, ".mcp.json"), mcpData, 0644)

	// Setup minimal Claude config
	oldConfigDir := os.Getenv("CLAUDE_CONFIG_DIR")
	tmpClaudeConfig, _ := os.MkdirTemp("", "claude-config-*")
	defer os.RemoveAll(tmpClaudeConfig)
	os.Setenv("CLAUDE_CONFIG_DIR", tmpClaudeConfig)
	defer func() {
		if oldConfigDir != "" {
			os.Setenv("CLAUDE_CONFIG_DIR", oldConfigDir)
		} else {
			os.Unsetenv("CLAUDE_CONFIG_DIR")
		}
	}()
	claudeConfig := map[string]interface{}{"mcpServers": map[string]interface{}{}}
	claudeData, _ := json.MarshalIndent(claudeConfig, "", "  ")
	_ = os.WriteFile(filepath.Join(tmpClaudeConfig, ".claude.json"), claudeData, 0600)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		GetMCPInfo(tmpDir)
	}
}

// BenchmarkGetMCPInfo_ParentDirectory benchmarks finding .mcp.json in parent (common case)
func BenchmarkGetMCPInfo_ParentDirectory(b *testing.B) {
	tmpRoot, err := os.MkdirTemp("", "mcp-bench-*")
	if err != nil {
		b.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpRoot)

	// Create .mcp.json in root
	mcpConfig := map[string]interface{}{
		"mcpServers": map[string]interface{}{
			"airbnb": map[string]interface{}{
				"command": "npx",
				"args":    []string{"-y", "@openbnb/mcp-server-airbnb"},
			},
		},
	}
	mcpData, _ := json.MarshalIndent(mcpConfig, "", "  ")
	_ = os.WriteFile(filepath.Join(tmpRoot, ".mcp.json"), mcpData, 0644)

	// Create subdir (project will be here)
	projectDir := filepath.Join(tmpRoot, "project", "subdir")
	if err := os.MkdirAll(projectDir, 0755); err != nil {
		b.Fatalf("Failed to create project dir: %v", err)
	}

	// Setup minimal Claude config
	oldConfigDir := os.Getenv("CLAUDE_CONFIG_DIR")
	tmpClaudeConfig, _ := os.MkdirTemp("", "claude-config-*")
	defer os.RemoveAll(tmpClaudeConfig)
	os.Setenv("CLAUDE_CONFIG_DIR", tmpClaudeConfig)
	defer func() {
		if oldConfigDir != "" {
			os.Setenv("CLAUDE_CONFIG_DIR", oldConfigDir)
		} else {
			os.Unsetenv("CLAUDE_CONFIG_DIR")
		}
	}()
	claudeConfig := map[string]interface{}{"mcpServers": map[string]interface{}{}}
	claudeData, _ := json.MarshalIndent(claudeConfig, "", "  ")
	_ = os.WriteFile(filepath.Join(tmpClaudeConfig, ".claude.json"), claudeData, 0600)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		GetMCPInfo(projectDir)
	}
}
