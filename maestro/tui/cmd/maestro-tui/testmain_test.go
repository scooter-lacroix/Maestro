package main

import (
	"os"
	"testing"
)

// TestMain ensures all cmd tests use the _test profile to prevent
// accidental modification of production data.
// CRITICAL: This was missing and caused test data to overwrite production sessions!
func TestMain(m *testing.M) {
	// Force _test profile for all tests in this package
	os.Setenv("AGENTDECK_PROFILE", "_test")
	os.Exit(m.Run())
}
