#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATCH_DIR="$ROOT_DIR/vendor/patches"

if [ ! -d "$PATCH_DIR" ]; then
  echo "No vendor patches directory found at $PATCH_DIR"
  exit 0
fi

for patch in "$PATCH_DIR"/*.patch; do
  [ -e "$patch" ] || continue
  if git -C "$ROOT_DIR" apply --reverse --check "$patch" >/dev/null 2>&1; then
    echo "Vendor patch already applied: $(basename "$patch")"
    continue
  fi

  echo "Applying vendor patch: $(basename "$patch")"
  git -C "$ROOT_DIR" apply "$patch"
done
