package ui

import (
	"testing"
)

func TestNewForkDialog(t *testing.T) {
	d := NewForkDialog()
	if d == nil {
		t.Fatal("NewForkDialog() returned nil")
	}
	if d.IsVisible() {
		t.Error("Dialog should not be visible initially")
	}
}

func TestForkDialog_Show(t *testing.T) {
	d := NewForkDialog()
	d.Show("Original Session", "/path/to/project", "group/path")

	if !d.IsVisible() {
		t.Error("Dialog should be visible after Show()")
	}

	name, group := d.GetValues()
	if name != "Original Session (fork)" {
		t.Errorf("Name = %s, want 'Original Session (fork)'", name)
	}
	if group != "group/path" {
		t.Errorf("Group = %s, want 'group/path'", group)
	}
}

func TestForkDialog_Hide(t *testing.T) {
	d := NewForkDialog()
	d.Show("Test", "/path", "group")

	if !d.IsVisible() {
		t.Error("Dialog should be visible after Show()")
	}

	d.Hide()

	if d.IsVisible() {
		t.Error("Dialog should not be visible after Hide()")
	}
}

func TestForkDialog_GetValues(t *testing.T) {
	d := NewForkDialog()
	d.Show("My Session", "/project", "work/team")

	name, group := d.GetValues()
	if name != "My Session (fork)" {
		t.Errorf("Name = %s, want 'My Session (fork)'", name)
	}
	if group != "work/team" {
		t.Errorf("Group = %s, want 'work/team'", group)
	}
}

func TestForkDialog_SetSize(t *testing.T) {
	d := NewForkDialog()
	d.SetSize(100, 50)

	if d.width != 100 {
		t.Errorf("Width = %d, want 100", d.width)
	}
	if d.height != 50 {
		t.Errorf("Height = %d, want 50", d.height)
	}
}

func TestForkDialog_EmptyProjectPath(t *testing.T) {
	d := NewForkDialog()
	d.Show("Test", "", "")

	if !d.IsVisible() {
		t.Error("Dialog should be visible even with empty paths")
	}

	name, group := d.GetValues()
	if name != "Test (fork)" {
		t.Errorf("Name = %s, want 'Test (fork)'", name)
	}
	if group != "" {
		t.Errorf("Group = %s, want ''", group)
	}
}
