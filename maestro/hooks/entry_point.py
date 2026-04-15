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

    original_stdout = sys.stdout
    original_stderr = sys.stderr

    try:
        # Hijack streams inside try to ensure finally restores them
        sys.stdout = io.StringIO()
        sys.stderr = io.StringIO()

        # Capture input from stdin
        try:
            raw_input = sys.stdin.read()
            data = json.loads(raw_input) if raw_input.strip() else {}
        except json.JSONDecodeError:
            data = {}
        except Exception as e:
            data = {}
            sys.stderr.write(f"Failed to read hook stdin: {e}\n")

        if not isinstance(data, dict):
            data = {}

        # Ensure project_path is set (handle missing, None, and empty string)
        if not data.get("project_path"):
            data["project_path"] = data.get("cwd") or os.getcwd()
        
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

        # Consume critical_think_result from checkpoint/review/loop hooks
        # and surface as additionalContext so Claude sees the metacognitive analysis.
        ct_result = result.get("critical_think_result")
        if ct_result and isinstance(ct_result, dict):
            ct_parts = []
            if ct_result.get("synthesis"):
                ct_parts.append(f"Analysis: {ct_result['synthesis']}")
            if ct_result.get("pitfalls"):
                pitfalls = ct_result["pitfalls"]
                if isinstance(pitfalls, list) and pitfalls:
                    ct_parts.append(f"Pitfalls: {'; '.join(str(p) for p in pitfalls[:5])}")
            if ct_result.get("risks"):
                risks = ct_result["risks"]
                if isinstance(risks, list) and risks:
                    ct_parts.append(f"Risks: {'; '.join(str(r) for r in risks[:5])}")
            if ct_result.get("next_steps"):
                next_steps = ct_result["next_steps"]
                if isinstance(next_steps, list) and next_steps:
                    ct_parts.append(f"Suggested next steps: {'; '.join(str(s) for s in next_steps[:5])}")
            if ct_result.get("revised_confidence") is not None:
                revised_confidence = ct_result['revised_confidence']
                if isinstance(revised_confidence, (int, float)):
                    ct_parts.append(f"Confidence: {revised_confidence:.0%}")

            if ct_parts:
                ct_context = f"[Maestro Critical Think - {phase}]\n" + "\n".join(ct_parts)
                if response and "hookSpecificOutput" in response:
                    existing = response["hookSpecificOutput"].get("additionalContext", "")
                    response["hookSpecificOutput"]["additionalContext"] = (
                        f"{existing}\n\n{ct_context}" if existing else ct_context
                    )
                elif response and "systemMessage" in response:
                    response["systemMessage"] += f"\n\n{ct_context}"
                elif response and "reason" in response:
                    # subagent-stop block — preserve decision, append CT context
                    response["reason"] = f"{response['reason']}\n\n{ct_context}"
                else:
                    response = {
                        "hookSpecificOutput": {
                            "hookEventName": event_name,
                            "additionalContext": ct_context,
                        }
                    }

        # Print final JSON to REAL stdout
        original_stdout.write(json.dumps(response if response else {}))
        
    except Exception as e:
        # On fatal error, still try to return something Claude likes
        # and log the error to REAL stderr for debugging
        original_stderr.write(f"FATAL HOOK ERROR ({phase}): {str(e)}\n")
        original_stdout.write("{}")
    finally:
        # Capture hijacked stderr content before restoring streams
        captured_stderr = ""
        if hasattr(sys.stderr, 'getvalue'):
            captured_stderr = sys.stderr.getvalue()

        # Restore original streams first to ensure consistent state
        sys.stdout = original_stdout
        sys.stderr = original_stderr

        # Flush captured diagnostics to real stderr; guard against write failures
        if captured_stderr:
            try:
                sys.stderr.write(captured_stderr)
            except Exception:
                pass

if __name__ == "__main__":
    if len(sys.argv) != 3:
        # sys.stderr is not hijacked yet at this point
        sys.stderr.write("Usage: entry_point.py <phase> <event_name>\n")
        sys.stdout.write("{}")
        sys.exit(1)

    try:
        run_hook(sys.argv[1], sys.argv[2])
    except Exception as e:
        sys.stderr.write(f"FATAL HOOK ERROR: {e}\n")
        sys.stdout.write("{}")
        sys.exit(1)
