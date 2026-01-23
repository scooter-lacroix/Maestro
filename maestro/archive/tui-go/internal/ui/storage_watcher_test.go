package ui

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

func TestNewStorageWatcher(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "sessions.json")

	// Create test file
	err := os.WriteFile(testFile, []byte("{}"), 0644)
	require.NoError(t, err)

	watcher, err := NewStorageWatcher(testFile)
	require.NoError(t, err)
	require.NotNil(t, watcher)

	defer watcher.Close()
}

func TestStorageWatcher_DetectsChanges(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "sessions.json")

	// Create initial file
	err := os.WriteFile(testFile, []byte("{}"), 0644)
	require.NoError(t, err)

	watcher, err := NewStorageWatcher(testFile)
	require.NoError(t, err)
	defer watcher.Close()

	// Start watching
	watcher.Start()

	// Modify file
	time.Sleep(50 * time.Millisecond)
	err = os.WriteFile(testFile, []byte(`{"test": "data"}`), 0644)
	require.NoError(t, err)

	// Should receive reload signal
	select {
	case <-watcher.ReloadChannel():
		// Success
	case <-time.After(500 * time.Millisecond):
		t.Fatal("Expected reload signal but got timeout")
	}
}

func TestStorageWatcher_Debouncing(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "sessions.json")

	err := os.WriteFile(testFile, []byte("{}"), 0644)
	require.NoError(t, err)

	watcher, err := NewStorageWatcher(testFile)
	require.NoError(t, err)
	defer watcher.Close()

	watcher.Start()

	// Rapid writes (simulate CLI making multiple changes)
	for i := 0; i < 5; i++ {
		time.Sleep(10 * time.Millisecond)
		_ = os.WriteFile(testFile, []byte(`{"count": `+string(rune('0'+i))+`}`), 0644)
	}

	// Should only get ONE reload signal (debounced)
	reloadCount := 0
	timeout := time.After(300 * time.Millisecond)

	for {
		select {
		case <-watcher.ReloadChannel():
			reloadCount++
		case <-timeout:
			require.Equal(t, 1, reloadCount, "Should debounce rapid writes to single reload")
			return
		}
	}
}

func TestStorageWatcher_NotifySaveIgnoresOwnChanges(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "sessions.json")

	err := os.WriteFile(testFile, []byte("{}"), 0644)
	require.NoError(t, err)

	watcher, err := NewStorageWatcher(testFile)
	require.NoError(t, err)
	defer watcher.Close()

	watcher.Start()

	// Notify that we're about to save (simulating TUI save)
	watcher.NotifySave()

	// Write to file (this simulates TUI's own save)
	time.Sleep(10 * time.Millisecond)
	err = os.WriteFile(testFile, []byte(`{"from_tui": true}`), 0644)
	require.NoError(t, err)

	// Should NOT receive reload signal (within ignore window)
	select {
	case <-watcher.ReloadChannel():
		t.Fatal("Should not receive reload signal for TUI's own save")
	case <-time.After(600 * time.Millisecond):
		// Success - no reload signal received
	}
}

func TestStorageWatcher_ExternalChangesStillDetected(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "sessions.json")

	err := os.WriteFile(testFile, []byte("{}"), 0644)
	require.NoError(t, err)

	watcher, err := NewStorageWatcher(testFile)
	require.NoError(t, err)
	defer watcher.Close()

	watcher.Start()

	// Notify that we saved
	watcher.NotifySave()

	// Wait for ignore window to expire
	time.Sleep(600 * time.Millisecond)

	// Now an external change should be detected
	err = os.WriteFile(testFile, []byte(`{"from_cli": true}`), 0644)
	require.NoError(t, err)

	// Should receive reload signal (outside ignore window)
	select {
	case <-watcher.ReloadChannel():
		// Success
	case <-time.After(500 * time.Millisecond):
		t.Fatal("Expected reload signal for external change but got timeout")
	}
}

// TestStorageWatcher_CrossProfileIsolation verifies that watchers for different profiles
// do NOT trigger on each other's file changes. This prevents the catastrophic data loss
// bug where creating a session in work profile would wipe out default profile sessions.
func TestStorageWatcher_CrossProfileIsolation(t *testing.T) {
	// Create two profile directories simulating ~/.maestro-tui/profiles/default and /work
	profile1Dir := filepath.Join(t.TempDir(), "profile1")
	profile2Dir := filepath.Join(t.TempDir(), "profile2")
	require.NoError(t, os.MkdirAll(profile1Dir, 0700))
	require.NoError(t, os.MkdirAll(profile2Dir, 0700))

	// Create sessions.json in each profile
	profile1File := filepath.Join(profile1Dir, "sessions.json")
	profile2File := filepath.Join(profile2Dir, "sessions.json")
	require.NoError(t, os.WriteFile(profile1File, []byte(`{"instances":[]}`), 0644))
	require.NoError(t, os.WriteFile(profile2File, []byte(`{"instances":[]}`), 0644))

	// Create watcher for profile1 only
	watcher1, err := NewStorageWatcher(profile1File)
	require.NoError(t, err)
	defer watcher1.Close()
	watcher1.Start()

	// Wait for watcher to initialize
	time.Sleep(100 * time.Millisecond)

	// Modify profile2's sessions.json (simulating work profile TUI saving)
	err = os.WriteFile(profile2File, []byte(`{"instances":[{"id":"test"}]}`), 0644)
	require.NoError(t, err)

	// Profile1's watcher should NOT fire (this is the critical test!)
	select {
	case <-watcher1.ReloadChannel():
		t.Fatal("CRITICAL BUG: Profile1 watcher fired when profile2 file changed! Cross-profile contamination detected!")
	case <-time.After(500 * time.Millisecond):
		// Success - watcher correctly ignored the other profile's change
	}

	// Verify profile1 watcher DOES fire when its own file changes
	err = os.WriteFile(profile1File, []byte(`{"instances":[{"id":"profile1-session"}]}`), 0644)
	require.NoError(t, err)

	select {
	case <-watcher1.ReloadChannel():
		// Success - detected change to its own file
	case <-time.After(500 * time.Millisecond):
		t.Fatal("Profile1 watcher should have detected change to its own file")
	}
}
