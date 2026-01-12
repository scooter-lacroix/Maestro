#!/usr/bin/env python3
"""
Maestro Pre-Tool-Use Hook: Smart Search Router

Routes search operations to appropriate search methods:
- AST-grep for structural searches
- Grep for literal searches
- Semantic search for concept searches

Stores search context for TLDR cross-file lookup.
"""

import json
import sys
import os
import re
from pathlib import Path

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))


def is_structural_search(pattern: str) -> bool:
    """
    Determine if a search pattern is structural.

    Structural patterns include:
    - Class definitions (class Foo, class Foo extends Bar)
    - Function definitions (def foo, function foo, foo() {})
    - AST-like patterns ($$$, ...)
    """
    structural_indicators = [
        r'\bclass\s+\w+',  # class definition
        r'\b(def|function|func)\s+\w+\s*\(',  # function definition
        r'\$\$\$',  # ast-grep placeholder
        r'\.\.\.',  # spread/rest operators
        r'==>\s*\w+',  # type annotations
        r'::\s*\w+',  # scope resolution
    ]

    for indicator in structural_indicators:
        if re.search(indicator, pattern):
            return True

    return False


def smart_search_hook(input_data: dict) -> dict:
    """
    Pre-tool-use hook that routes searches intelligently.

    Args:
        input_data: Hook input data containing tool invocation info

    Returns:
        Modified input data with search routing info
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Only process Grep operations
        if tool_name != "Grep":
            return input_data

        tool_input = input_data.get("tool_input", {})
        pattern = tool_input.get("pattern", "")

        if not pattern:
            return input_data

        # Determine search type
        search_type = "literal"
        if is_structural_search(pattern):
            search_type = "structural"
        elif any(c in pattern for c in ['.*', '^', '$', '[', ']', '(', ')']):
            search_type = "regex"
        elif len(pattern.split()) > 3:
            # Multi-word queries might be semantic
            search_type = "semantic"

        # Store search context
        input_data["search_context"] = {
            "pattern": pattern,
            "search_type": search_type,
            "path": tool_input.get("path", ""),
        }

        # For structural searches, suggest using TLDR
        if search_type == "structural":
            input_data["search_suggestion"] = {
                "tool": "maestro:ast-grep",
                "pattern": pattern,
                "reason": "Structural search detected - AST-grep will be more accurate",
            }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = smart_search_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
