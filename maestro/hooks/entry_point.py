#!/usr/bin/env python3
"""
Maestro Hook Entry Point

This script serves as the unified entry point for all Maestro hooks called by Claude Code.
It handles input parsing, suppresses noisy output, and ensures the response matches
Claude Code's hook schema.
"""

import json
import sys
import os
import io
from pathlib import Path



from typing import Dict, Any, List

# Ensure we can import maestro
maestro_root = Path(__file__).parent.parent.parent.absolute()
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))

def run_hook(phase: str, event_name: str):
    from maestro.hooks.executor import get_hook_executor
    
    # Delay stream hijacking until after imports
    original_stdout = sys.stdout
    original_stderr = sys.stderr
    sys.stdout = io.StringIO()
    sys.stderr = io.StringIO()
    
    # Capture input from stdin
    try:
        raw_input = sys.stdin.read()
        data = json.loads(raw_input) if raw_input.strip() else {}
    except json.JSONDecodeError:
        data = {}

    try:
        # Ensure project_path is set
        data.setdefault("project_path", data.get("cwd") or str(maestro_root))
        
        # Execute hooks for the phase
        executor = get_hook_executor()
        result = executor.execute_phase(phase, data)
        
        # Collect context and instructions
        context_parts = []
        
        # Phase-specific handling
        if phase == "session-start":
            instruction = result.get("maestro_docs_instruction")
            if instruction:
                context_parts.append(instruction)
        
        # General error reporting
        hook_error = result.get("hook_error")
        if hook_error:
            context_parts.append(f"Maestro hook warning ({phase}): {hook_error}")
            
        # Construct response
        response = {}
        
        if phase == "session-start":
            if context_parts:
                response = {
                    "hookSpecificOutput": {
                        "hookEventName": event_name,
                        "additionalContext": "\n\n".join(context_parts),
                    }
                }
        elif phase == "pre-tool-use":
            reason = result.get("hook_message") or result.get("hook_error")
            if result.get("hook_block") and reason:
                response = {
                    "hookSpecificOutput": {
                        "hookEventName": event_name,
                        "permissionDecision": "deny",
                        "permissionDecisionReason": reason,
                    }
                }
        elif phase == "subagent-stop":
            if result.get("hook_block") and result.get("hook_message"):
                response = {"decision": "block", "reason": result["hook_message"]}
        elif phase == "pre-compact":
            messages = []
            continuity = result.get("continuity_preserved")
            if continuity:
                messages.append(f"Preserved {continuity.get('preserved_count', 0)} memories.")
            if result.get("hook_error"):
                messages.append(f"Warning: {result['hook_error']}")
            
            response = {
                "continue": True,
                "systemMessage": "\n".join(messages) if messages else "Maestro pre-compact complete."
            }

        # Print final JSON to REAL stdout
        original_stdout.write(json.dumps(response if response else {}))
        
    except Exception as e:
        # On fatal error, still try to return something Claude likes
        # and log the error to REAL stderr for debugging
        original_stderr.write(f"FATAL HOOK ERROR ({phase}): {str(e)}\n")
        original_stdout.write("{}")
    finally:
        # Capture and flush hijacked stderr to the real stream for debugging
        captured_stderr = sys.stderr.getvalue()

        # Restore original streams
        sys.stdout = original_stdout
        sys.stderr = original_stderr

        if captured_stderr:
            sys.stderr.write(captured_stderr)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        # Use original_stderr because sys.stderr is currently hijacked
        original_stderr.write("Usage: entry_point.py <phase> <event_name>\n")
        sys.exit(1)
        
    run_hook(sys.argv[1], sys.argv[2])
