#!/bin/bash
# Test script to verify environment setup in tmux sessions

set -e

echo "=== Testing Environment Setup Fixes ==="
echo ""

# Get the current environment values
HOME_DIR="${HOME:-$(cd ~ && pwd)}"
PATH_DIR="${PATH}"

# Determine CLAUDE_CONFIG_DIR (same logic as the TUI code)
CLAUDE_CONFIG="${CLAUDE_CONFIG_DIR}"
if [ -z "$CLAUDE_CONFIG" ]; then
    CLAUDE_CONFIG="$HOME_DIR/.claude"
fi

echo "Current environment:"
echo "  HOME: $HOME_DIR"
echo "  PATH: ${PATH_DIR:0:100}..."
echo "  CLAUDE_CONFIG_DIR: $CLAUDE_CONFIG"
echo ""

# Create a test tmux session using the same method as maestro-tui
TEST_SESSION="maestro_test_env_$$"

echo "Creating test tmux session: $TEST_SESSION"
tmux new-session -d -s "$TEST_SESSION" -c "$HOME_DIR"

# Set HOME in tmux environment (like maestro-tui does)
tmux set-environment -t "$TEST_SESSION" "HOME" "$HOME_DIR"

# Set PATH in tmux environment (like maestro-tui does)
tmux set-environment -t "$TEST_SESSION" "PATH" "$PATH_DIR"

# Set CLAUDE_CONFIG_DIR (like maestro-tui does - defaults to ~/.claude if not set)
tmux set-environment -t "$TEST_SESSION" "CLAUDE_CONFIG_DIR" "$CLAUDE_CONFIG"

# Verify environment is set correctly
echo "Verifying tmux session environment..."
TMUX_HOME=$(tmux display-message -t "$TEST_SESSION" -p '#{HOME}')
TMUX_PATH=$(tmux display-message -t "$TEST_SESSION" -p '#{PATH}' | head -c 100)
TMUX_CLAUDE=$(tmux display-message -t "$TEST_SESSION" -p '#{CLAUDE_CONFIG_DIR}')

echo "  TMUX HOME: $TMUX_HOME"
echo "  TMUX PATH: ${TMUX_PATH}..."
echo "  TMUX CLAUDE_CONFIG_DIR: ${TMUX_CLAUDE:-<not set>}"
echo ""

# Test that commands can be found
echo "Testing command availability in tmux session..."
echo -n "  claude: "
if tmux run-shell -t "$TEST_SESSION" "which claude" 2>/dev/null; then
    echo "✓ FOUND"
else
    echo "✗ NOT FOUND"
fi

echo -n "  gemini: "
if tmux run-shell -t "$TEST_SESSION" "which gemini" 2>/dev/null; then
    echo "✓ FOUND"
else
    echo "✗ NOT FOUND"
fi

echo -n "  codex: "
if tmux run-shell -t "$TEST_SESSION" "which codex" 2>/dev/null; then
    echo "✓ FOUND"
else
    echo "✗ NOT FOUND"
fi

# Test that Claude can access its config
echo ""
echo "Testing Claude config access..."
echo -n "  Can read ~/.claude/settings.json: "
if tmux run-shell -t "$TEST_SESSION" "test -f ~/.claude/settings.json && echo YES" 2>/dev/null; then
    echo "✓ YES"
else
    echo "✗ NO"
fi

echo -n "  CLAUDE_CONFIG_DIR is set: "
if tmux run-shell -t "$TEST_SESSION" "test -n \"\$CLAUDE_CONFIG_DIR\" && echo YES" 2>/dev/null; then
    echo "✓ YES"
else
    echo "✗ NO"
fi

echo -n "  CLAUDE_CONFIG_DIR points to valid dir: "
if tmux run-shell -t "$TEST_SESSION" "test -d \"\$CLAUDE_CONFIG_DIR\" && echo YES" 2>/dev/null; then
    echo "✓ YES"
else
    echo "✗ NO"
fi

# Test that Claude can actually read settings.json via CLAUDE_CONFIG_DIR
echo -n "  Can read \$CLAUDE_CONFIG_DIR/settings.json: "
if tmux run-shell -t "$TEST_SESSION" "test -f \"\$CLAUDE_CONFIG_DIR/settings.json\" && echo YES" 2>/dev/null; then
    echo "✓ YES"
else
    echo "✗ NO"
fi

# Cleanup
echo ""
echo "Cleaning up test session..."
tmux kill-session -t "$TEST_SESSION" 2>/dev/null || true

echo ""
echo "=== Test Complete ==="
