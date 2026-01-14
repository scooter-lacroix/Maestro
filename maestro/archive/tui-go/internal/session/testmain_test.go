package session

import (
	"os"
	"testing"
)

func TestMain(m *testing.M) {
	// Force test profile to prevent production data corruption
	// See CLAUDE.md: "2025-12-11 Incident: Tests with AGENTDECK_PROFILE=work overwrote ALL 36 production sessions"
	os.Setenv("AGENTDECK_PROFILE", "_test")
	os.Exit(m.Run())
}
