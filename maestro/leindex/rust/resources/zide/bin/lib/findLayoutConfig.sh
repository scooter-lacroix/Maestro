#!/usr/bin/env bash

# Usage:
#   findLayoutConfig "<config_list>" "<current_layout>"
#
#   Where <config_list> is a space-separated list of items.
#   Each item can be either "path" or "path:layout".
#   <current_layout> is the layout to match against.

findLayoutConfig() {
  local config_list="$1"
  local current_layout="$2"

  # If config_list is empty, exit with failure
  [ -z "$config_list" ] && return 1

  for item in $config_list; do
    if [[ "$item" == *:* ]]; then
      # Split on the colon
      local path="${item%%:*}"
      local layout="${item#*:}"
      # Check if layout matches current_layout
      if [[ "$current_layout" == "$layout"* ]]; then
        echo "$path"
        return 0
      fi
    else
      # No colon -> treat the entire item as path for any layout
      echo "$item"
      return 0
    fi
  done

  return 1  # No match found
}
