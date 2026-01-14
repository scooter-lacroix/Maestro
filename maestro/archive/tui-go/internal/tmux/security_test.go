package tmux

import (
	"strings"
	"testing"
)

// TestSanitizeNameSecurity tests session name sanitization for security
func TestSanitizeNameSecurity(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		// Normal cases
		{
			name:     "simple name",
			input:    "my-project",
			expected: "my-project",
		},
		{
			name:     "spaces become hyphens",
			input:    "my project",
			expected: "my-project",
		},
		{
			name:     "special chars become hyphens",
			input:    "my@#$project",
			expected: "my-project",
		},

		// SECURITY: Leading/trailing hyphens removed
		{
			name:     "leading hyphens removed",
			input:    "---test",
			expected: "test",
		},
		{
			name:     "trailing hyphens removed",
			input:    "test---",
			expected: "test",
		},
		{
			name:     "both leading and trailing",
			input:    "---test---",
			expected: "test",
		},

		// SECURITY: Multiple consecutive hyphens collapsed
		{
			name:     "multiple hyphens collapsed",
			input:    "test---name",
			expected: "test-name",
		},
		{
			name:     "many hyphens collapsed",
			input:    "a----b----c",
			expected: "a-b-c",
		},

		// SECURITY: Length limit enforced
		{
			name:     "long name truncated",
			input:    strings.Repeat("a", 100),
			expected: strings.Repeat("a", 50),
		},

		// SECURITY: Empty after sanitization gets default
		{
			name:     "only special chars",
			input:    "@#$%^&*()",
			expected: "session",
		},
		{
			name:     "empty string",
			input:    "",
			expected: "session",
		},
		{
			name:     "only hyphens",
			input:    "---",
			expected: "session",
		},

		// Edge cases
		{
			name:     "mixed case preserved",
			input:    "MyProject",
			expected: "MyProject",
		},
		{
			name:     "numbers allowed",
			input:    "project123",
			expected: "project123",
		},
		{
			name:     "dots become hyphens",
			input:    "my.project",
			expected: "my-project",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := sanitizeName(tt.input)
			if result != tt.expected {
				t.Errorf("sanitizeName(%q) = %q, want %q", tt.input, result, tt.expected)
			}
		})
	}
}

// TestShquote tests shell quoting for security
func TestShquote(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		contains []string // What the output should contain
		notContains []string // What the output should NOT contain
	}{
		{
			name:     "simple path",
			input:    "/tmp/test.log",
			contains: []string{"/tmp/test.log"},
		},
		// SECURITY: Single quotes are escaped
		{
			name:     "single quote escaped",
			input:    "file'name.log",
			contains: []string{"'"}, // Should have quotes
			notContains: []string{"file'name"}, // Should not have bare single quote in middle
		},
		// SECURITY: Dangerous characters trigger quoting
		{
			name:     "ampersand quoted",
			input:    "test&file.log",
			contains: []string{"'"},
		},
		{
			name:     "semicolon quoted",
			input:    "test;rm -rf /",
			contains: []string{"'"},
		},
		{
			name:     "backtick quoted",
			input:    "test`whoami`",
			contains: []string{"'"},
		},
		{
			name:     "dollar paren quoted",
			input:    "test$(malicious)",
			contains: []string{"'"},
		},
		{
			name:     "newline quoted",
			input:    "test\nfile",
			contains: []string{"'"},
		},
		{
			name:     "pipe quoted",
			input:    "test|cat",
			contains: []string{"'"},
		},
		{
			name:     "redirection quoted",
			input:    "test>/etc/passwd",
			contains: []string{"'"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := shquote(tt.input)
			for _, expected := range tt.contains {
				if !strings.Contains(result, expected) {
					t.Errorf("shquote(%q) = %q should contain %q", tt.input, result, expected)
				}
			}
			for _, notExpected := range tt.notContains {
				if strings.Contains(result, notExpected) {
					t.Errorf("shquote(%q) = %q should NOT contain %q", tt.input, result, notExpected)
				}
			}
		})
	}
}

// TestShquotePreventsCommandInjection verifies that shquote prevents
// common command injection patterns
func TestShquotePreventsCommandInjection(t *testing.T) {
	dangerousInputs := []string{
		"file.log; rm -rf /",
		"file.log & whoami",
		"file.log|cat /etc/passwd",
		"file.log`whoami`",
		"file.log$(malicious)",
		"file.log\nmalicious",
		"file.log\rmalicious",
		"file.log>/etc/passwd",
		"file.log</etc/passwd",
		"file.log && malicious",
		"file.log || malicious",
	}

	for _, input := range dangerousInputs {
		t.Run(input, func(t *testing.T) {
			quoted := shquote(input)
			// The quoted version should be wrapped in single quotes
			// which prevents shell interpretation
			if !strings.HasPrefix(quoted, "'") || !strings.HasSuffix(quoted, "'") {
				t.Errorf("shquote(%q) = %q should be wrapped in single quotes", input, quoted)
			}
		})
	}
}
