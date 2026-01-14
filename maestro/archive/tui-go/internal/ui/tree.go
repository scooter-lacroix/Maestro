package ui

import (
	"fmt"
	"sort"
	"strings"
)

// TreeFolder represents a folder in the tree
type TreeFolder struct {
	Name     string
	Expanded bool
	Count    int
}

// Tree manages folder structure
type Tree struct {
	folders map[string]*TreeFolder
	order   []string
}

// NewTree creates a new tree
func NewTree() *Tree {
	return &Tree{
		folders: make(map[string]*TreeFolder),
		order:   []string{},
	}
}

// AddFolder adds a folder to the tree
func (t *Tree) AddFolder(name string) {
	if _, exists := t.folders[name]; !exists {
		t.folders[name] = &TreeFolder{
			Name:     name,
			Expanded: true,
			Count:    0,
		}
		t.order = append(t.order, name)
		sort.Strings(t.order)
	}
}

// SetFolderCount sets the session count for a folder
func (t *Tree) SetFolderCount(name string, count int) {
	if folder, exists := t.folders[name]; exists {
		folder.Count = count
	}
}

// GetFolders returns all folder names in order
func (t *Tree) GetFolders() []string {
	return t.order
}

// IsFolderExpanded returns whether a folder is expanded
func (t *Tree) IsFolderExpanded(name string) bool {
	if folder, exists := t.folders[name]; exists {
		return folder.Expanded
	}
	return false
}

// ToggleFolder toggles folder expansion
func (t *Tree) ToggleFolder(name string) {
	if folder, exists := t.folders[name]; exists {
		folder.Expanded = !folder.Expanded
	}
}

// GetFolder returns a folder by name
func (t *Tree) GetFolder(name string) *TreeFolder {
	return t.folders[name]
}

// Clear removes all folders
func (t *Tree) Clear() {
	t.folders = make(map[string]*TreeFolder)
	t.order = []string{}
}

// View renders the tree (just folders, not sessions)
func (t *Tree) View(selectedFolder string) string {
	var b strings.Builder

	for _, name := range t.order {
		folder := t.folders[name]

		style := FolderStyle
		arrow := "▼"
		if !folder.Expanded {
			arrow = "▶"
			style = FolderCollapsedStyle
		}

		if name == selectedFolder {
			style = style.Background(ColorSurface)
		}

		line := style.Render(fmt.Sprintf("%s %s (%d)", arrow, name, folder.Count))
		b.WriteString(line)
		b.WriteString("\n")
	}

	return b.String()
}
