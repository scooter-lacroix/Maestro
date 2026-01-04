package tmux

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestLogWatcher(t *testing.T) {
	// Create temp log directory
	logDir := t.TempDir()
	logFile := filepath.Join(logDir, "test_session.log")

	// Track events
	events := make(chan string, 10)

	// Create watcher
	watcher, err := NewLogWatcher(logDir, func(sessionName string) {
		events <- sessionName
	})
	assert.NoError(t, err)
	defer watcher.Close()

	// Start watching
	go watcher.Start()
	time.Sleep(100 * time.Millisecond)

	// Create and write to log file
	f, err := os.Create(logFile)
	assert.NoError(t, err)
	_, _ = f.WriteString("test output\n")
	f.Close()

	// Wait for event
	select {
	case name := <-events:
		assert.Equal(t, "test_session", name)
	case <-time.After(2 * time.Second):
		t.Fatal("Timeout waiting for file event")
	}
}
