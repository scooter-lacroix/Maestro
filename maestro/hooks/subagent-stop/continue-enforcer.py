#!/usr/bin/env python3
"""
Subagent-stop hook: Continue Until Complete Enforcer

When the agent tries to stop during an implement/orchestrate session,
this hook checks whether the track is actually complete or a manual
review point was reached. If neither, it reprimands and forces continuation.
"""

import json
import sys


def check_should_continue(input_data: dict) -> dict:
    """Check if the agent should be forced to continue."""
    try:
        session_type = input_data.get("session_type", "")
        if session_type not in ("implement", "orchestrate"):
            return input_data

        track_complete = input_data.get("track_complete", False)
        review_point_reached = input_data.get("review_point_reached", False)
        user_requested_stop = input_data.get("user_requested_stop", False)

        if track_complete:
            input_data["checkpoint_reached"] = True
        if review_point_reached:
            input_data["review_point_reached"] = True

        if track_complete or review_point_reached or user_requested_stop:
            return input_data

        input_data["hook_block"] = True
        input_data["hook_message"] = (
            "⚠️ WORKFLOW VIOLATION: You attempted to stop before the track is complete "
            "and no manual review point has been reached.\n\n"
            "You MUST continue working until either:\n"
            "1. The current track is COMPLETE (all tasks done, all tests passing)\n"
            "2. A user-configured manual review point is reached\n"
            "3. The user explicitly requests a stop\n\n"
            "CONTINUE WORKING on the current task."
        )
    except Exception as e:
        input_data["hook_error"] = str(e)

    return input_data


def main() -> None:
    input_data = json.loads(sys.stdin.read())
    result = check_should_continue(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
