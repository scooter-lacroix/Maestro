package ui

import (
	"os"
	"strings"
	"testing"
)

func TestNewNewDialog(t *testing.T) {
	d := NewNewDialog()

	if d == nil {
		t.Fatal("NewNewDialog returned nil")
	}
	if d.IsVisible() {
		t.Error("Dialog should not be visible by default")
	}
	if len(d.presetCommands) == 0 {
		t.Error("presetCommands should not be empty")
	}
}

func TestDialogVisibility(t *testing.T) {
	d := NewNewDialog()

	d.Show()
	if !d.IsVisible() {
		t.Error("Dialog should be visible after Show()")
	}

	d.Hide()
	if d.IsVisible() {
		t.Error("Dialog should not be visible after Hide()")
	}
}

func TestDialogSetSize(t *testing.T) {
	d := NewNewDialog()
	d.SetSize(100, 50)

	if d.width != 100 {
		t.Errorf("Width = %d, want 100", d.width)
	}
	if d.height != 50 {
		t.Errorf("Height = %d, want 50", d.height)
	}
}

func TestDialogPresetCommands(t *testing.T) {
	d := NewNewDialog()

	// Should have shell (empty), claude, gemini, opencode, codex
	expectedCommands := []string{"", "claude", "gemini", "opencode", "codex"}

	if len(d.presetCommands) != len(expectedCommands) {
		t.Errorf("Expected %d preset commands, got %d", len(expectedCommands), len(d.presetCommands))
	}

	for i, cmd := range expectedCommands {
		if d.presetCommands[i] != cmd {
			t.Errorf("presetCommands[%d] = %s, want %s", i, d.presetCommands[i], cmd)
		}
	}
}

func TestDialogGetValues(t *testing.T) {
	d := NewNewDialog()
	d.nameInput.SetValue("my-session")
	d.pathInput.SetValue("/tmp/project")
	d.commandCursor = 1 // claude

	name, path, command := d.GetValues()

	if name != "my-session" {
		t.Errorf("name = %s, want my-session", name)
	}
	if path != "/tmp/project" {
		t.Errorf("path = %s, want /tmp/project", path)
	}
	if command != "claude" {
		t.Errorf("command = %s, want claude", command)
	}
}

func TestDialogExpandTilde(t *testing.T) {
	d := NewNewDialog()
	d.nameInput.SetValue("test")
	d.pathInput.SetValue("~/projects")

	_, path, _ := d.GetValues()

	home, _ := os.UserHomeDir()
	if !strings.HasPrefix(path, home) {
		t.Errorf("path should expand ~ to home directory, got %s", path)
	}
}

func TestDialogView(t *testing.T) {
	d := NewNewDialog()

	// Not visible - should return empty
	view := d.View()
	if view != "" {
		t.Error("View should be empty when not visible")
	}

	// Visible - should return content
	d.SetSize(80, 24)
	d.Show()
	view = d.View()
	if view == "" {
		t.Error("View should not be empty when visible")
	}
	if !strings.Contains(view, "New Session") {
		t.Error("View should contain 'New Session' title")
	}
}

func TestNewDialog_SetPathSuggestions(t *testing.T) {
	d := NewNewDialog()

	paths := []string{
		"/Users/test/project1",
		"/Users/test/project2",
		"/Users/test/other",
	}

	d.SetPathSuggestions(paths)

	if len(d.pathSuggestions) != 3 {
		t.Errorf("expected 3 suggestions, got %d", len(d.pathSuggestions))
	}

	// Verify suggestions are set on textinput
	available := d.pathInput.AvailableSuggestions()
	if len(available) != 3 {
		t.Errorf("expected 3 available suggestions on pathInput, got %d", len(available))
	}
}

func TestNewDialog_ShowSuggestionsEnabled(t *testing.T) {
	d := NewNewDialog()

	// ShowSuggestions should be enabled by default
	if !d.pathInput.ShowSuggestions {
		t.Error("expected ShowSuggestions to be true on pathInput")
	}
}

func TestNewDialog_SuggestionFiltering(t *testing.T) {
	d := NewNewDialog()

	paths := []string{
		"/Users/test/project-alpha",
		"/Users/test/project-beta",
		"/Users/test/other-thing",
	}

	d.SetPathSuggestions(paths)

	// Verify suggestions are available
	available := d.pathInput.AvailableSuggestions()
	if len(available) != 3 {
		t.Errorf("expected 3 available suggestions, got %d", len(available))
	}

	// Verify specific suggestions are in the list
	hasProjectAlpha := false
	hasProjectBeta := false
	hasOtherThing := false
	for _, s := range available {
		if s == "/Users/test/project-alpha" {
			hasProjectAlpha = true
		}
		if s == "/Users/test/project-beta" {
			hasProjectBeta = true
		}
		if s == "/Users/test/other-thing" {
			hasOtherThing = true
		}
	}

	if !hasProjectAlpha || !hasProjectBeta || !hasOtherThing {
		t.Error("not all expected suggestions are available")
	}
}

func TestNewDialog_MalformedPathFix(t *testing.T) {
	home, _ := os.UserHomeDir()

	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{
			name:     "normal tilde path",
			input:    "~/projects/myapp",
			expected: home + "/projects/myapp",
		},
		{
			name:     "malformed path with cwd prefix",
			input:    "/Users/someone/claude-deck~/projects/myapp",
			expected: home + "/projects/myapp",
		},
		{
			name:     "already expanded path",
			input:    "/Users/ashesh/projects/myapp",
			expected: "/Users/ashesh/projects/myapp",
		},
		{
			name:     "just tilde",
			input:    "~",
			expected: home,
		},
		{
			name:     "malformed path with different prefix",
			input:    "/some/random/path~/other/path",
			expected: home + "/other/path",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			d := NewNewDialog()
			d.pathInput.SetValue(tt.input)

			_, path, _ := d.GetValues()

			if path != tt.expected {
				t.Errorf("GetValues() path = %q, want %q", path, tt.expected)
			}
		})
	}
}
