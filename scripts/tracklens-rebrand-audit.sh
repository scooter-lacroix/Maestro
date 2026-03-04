#!/usr/bin/env bash
# TrackLens Rebranding Audit Script
#
# Greps for forbidden strings from Plannotator legacy that should not exist
# in TrackLens codebase (excluding acceptable contexts)

set -euo pipefail

# Forbidden strings that indicate incomplete rebranding
FORBIDDEN=(
  "plannotator"
  "PLANNOTATOR_"
  "tater"
  "backnotprop"
  "plannotator.ai"
)

# Directories to scan (source only, exclude dist/)
SCAN_DIRS=(
  "packages/tracklens-*/src"
  "apps/tracklens-*/src"
  "pi-maestro/src/tracklens"
  "src/leindex/src/tracklens"
  "crates/cockpit/src/tracklens"
  "crates/cli/src/commands/tracklens*"
)

# Acceptable comment patterns (documentation of migration)
ACCEPTABLE_COMMENT_PATTERNS=(
  "REBRANDED:"
  "Changed from"
  "Renamed from"
  "Tags include"
  "Default folder"
  "Storage key changed"
  "Removed.*references"
  "Should not import"
  "import.*from.*@plannotator"
)

found=0

for term in "${FORBIDDEN[@]}"; do
  for dir in ${SCAN_DIRS[@]}; do
    if [ -d "$dir" ]; then
      # Grep for the term in source files
      while IFS= read -r line; do
        file_path=$(echo "$line" | cut -d: -f1)
        line_content=$(echo "$line" | cut -d: -f2-)

        # Check if line contains any acceptable pattern
        is_acceptable=0
        for pattern in "${ACCEPTABLE_COMMENT_PATTERNS[@]}"; do
          if echo "$line_content" | grep -q "$pattern"; then
            is_acceptable=1
            break
          fi
        done

        if [ $is_acceptable -eq 0 ]; then
          echo "FAIL: Found '$term' in $file_path"
          echo "  Line: $line_content"
          found=1
        fi
      done < <(grep -rn --include='*.ts' --include='*.tsx' --include='*.rs' "$term" "$dir" 2>/dev/null || true)
    fi
  done
done

if [ $found -eq 0 ]; then
  echo "✓ Rebranding audit passed - no forbidden strings found"
  exit 0
else
  echo ""
  echo "✗ Rebranding audit failed - violations detected"
  exit 1
fi
