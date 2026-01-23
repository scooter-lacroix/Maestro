package ui

import (
	"testing"
)

func TestColorsDefined(t *testing.T) {
	colors := []string{
		string(ColorBg),
		string(ColorSurface),
		string(ColorBorder),
		string(ColorText),
		string(ColorAccent),
	}
	for _, c := range colors {
		if c == "" {
			t.Error("Color should not be empty")
		}
	}
}

func TestStatusIndicator(t *testing.T) {
	tests := []struct {
		status   string
		expected string
	}{
		{"running", "●"},
		{"waiting", "○"},
		{"idle", "◌"},
		{"error", "✕"},
		{"unknown", "◌"},
	}
	for _, tt := range tests {
		result := StatusIndicator(tt.status)
		if result == "" {
			t.Errorf("StatusIndicator(%s) returned empty", tt.status)
		}
	}
}

func TestToolIcon(t *testing.T) {
	tests := []struct {
		tool     string
		expected string
	}{
		{"claude", IconClaude},
		{"gemini", IconGemini},
		{"opencode", IconOpenCode},
		{"codex", IconCodex},
		{"shell", IconShell},
		{"unknown", IconShell},
	}
	for _, tt := range tests {
		result := ToolIcon(tt.tool)
		if result != tt.expected {
			t.Errorf("ToolIcon(%s) = %s, want %s", tt.tool, result, tt.expected)
		}
	}
}

func TestMenuKey(t *testing.T) {
	result := MenuKey("q", "Quit")
	if result == "" {
		t.Error("MenuKey should not return empty string")
	}
}
