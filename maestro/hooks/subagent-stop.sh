#!/bin/bash
set -euo pipefail
source "$(dirname "$0")/common.sh"
run_hook "subagent-stop" "SubagentStop"
