package main

import (
	"os/exec"
	"testing"

	"github.com/scooter-lacroix/maestro/tui/internal/ui"
)

func TestTmuxAvailable(t *testing.T) {
	_, err := exec.LookPath("tmux")
	if err != nil {
		t.Skip("tmux not available - skipping test")
	}
}

func TestHomeInit(t *testing.T) {
	home := ui.NewHome()
	if home == nil {
		t.Fatal("NewHome() returned nil")
	}
}

func TestHomeView(t *testing.T) {
	home := ui.NewHome()
	view := home.View()
	if view == "" {
		t.Error("View() returned empty string")
	}
}
