package ui

import (
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// Preview shows session terminal content
type Preview struct {
	content string
	title   string
	width   int
	height  int
}

// NewPreview creates a new preview pane
func NewPreview() *Preview {
	return &Preview{
		content: "",
		title:   "",
	}
}

// SetContent sets the preview content
func (p *Preview) SetContent(content, title string) {
	p.content = content
	p.title = title
}

// SetSize sets preview dimensions
func (p *Preview) SetSize(width, height int) {
	p.width = width
	p.height = height
}

// View renders the preview
func (p *Preview) View() string {
	var b strings.Builder

	// Header
	header := PreviewHeaderStyle.Render("Preview: " + p.title)
	b.WriteString(header)
	b.WriteString("\n")
	lineLen := min(p.width-4, 40)
	if lineLen < 0 {
		lineLen = 40
	}
	b.WriteString(strings.Repeat("─", lineLen))
	b.WriteString("\n\n")

	// Content
	if p.content == "" {
		dimStyle := lipgloss.NewStyle().Foreground(ColorTextDim).Italic(true)
		b.WriteString(dimStyle.Render("No content"))
	} else {
		// Limit content to available height
		lines := strings.Split(p.content, "\n")
		maxLines := p.height - 4
		if maxLines < 1 {
			maxLines = 10
		}
		if len(lines) > maxLines {
			lines = lines[len(lines)-maxLines:]
		}

		contentStyle := PreviewContentStyle
		for _, line := range lines {
			// Truncate long lines
			maxWidth := p.width - 4
			if maxWidth > 0 && len(line) > maxWidth {
				truncateAt := maxWidth - 3
				if truncateAt > 0 {
					line = line[:truncateAt] + "..."
				} else {
					line = "..."
				}
			}
			b.WriteString(contentStyle.Render(line))
			b.WriteString("\n")
		}
	}

	return b.String()
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
