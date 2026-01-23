#!/usr/bin/env python3
"""
Maestro User-Prompt-Submit Hook: Skill Activation

Analyzes user prompts to activate relevant Maestro skills.
Provides intelligent skill matching based on intent and keywords.
"""

import json
import sys
import os
import re
from pathlib import Path
from typing import Any

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))

def get_registry() -> Any:
    try:
        import importlib

        module = importlib.import_module("maestro.skills.registry")
        registry_getter = getattr(module, "get_registry", None)
        if callable(registry_getter):
            return registry_getter()
    except Exception:
        return None
    return None


# Skill keyword patterns
SKILL_PATTERNS = {
    "workflow": r"\b/(maestro:)?(workflow|build|fix|tdd|refactor|review|test)\b",
    "create-handoff": r"\b/(maestro:)?(create.?handoff|handoff)\b",
    "resume-handoff": r"\b/(maestro:)?(resume.?handoff)\b",
    "tldr-code": r"\b/(maestro:)?(tree|structure|tldr)\b",
    "ast-grep-find": r"\b/(maestro:)?(ast.?grep|ast.?search)\b",
    "premortem": r"\b/(maestro:)?premortem\b",
    "qlty-check": r"\b/(maestro:)?(quality|qlty.?check)\b",
    "braintrust-analyze": r"\b/(maestro:)?(braintrust|analyze)\b",
    "perplexity-search": r"\b/(maestro:)?(perplexity|search.?web)\b",
    "discovery-interview": r"\b/(maestro:)?(discovery|interview)\b",
    "math-unified": r"\b/(maestro:)?(math|calculate)\b",
}


def analyze_prompt_for_skills(prompt: str) -> list[str]:
    """
    Analyze a user prompt and return relevant skill suggestions.

    Args:
        prompt: User's input prompt

    Returns:
        List of skill names that match the prompt
    """
    suggested_skills: list[str] = []

    # Check for explicit skill commands
    for skill_name, pattern in SKILL_PATTERNS.items():
        if re.search(pattern, prompt, re.IGNORECASE):
            suggested_skills.append(skill_name)

    # Check for intent-based matching
    prompt_lower = prompt.lower()

    # Intent: Create something
    if any(word in prompt_lower for word in ["create", "add new", "implement", "build"]):
        if "workflow" not in suggested_skills:
            suggested_skills.append("workflow")

    # Intent: Analyze code
    if any(word in prompt_lower for word in ["analyze", "understand", "explain code"]):
        if "tldr-code" not in suggested_skills:
            suggested_skills.append("tldr-code")

    # Intent: Fix bugs
    if any(word in prompt_lower for word in ["bug", "error", "not working", "fix"]):
        if "workflow" not in suggested_skills:
            suggested_skills.append("workflow")  # /maestro:fix
        if "ast-grep-find" not in suggested_skills:
            suggested_skills.append("ast-grep-find")

    # Intent: Refactor
    if any(word in prompt_lower for word in ["refactor", "clean up", "restructure"]):
        if "workflow" not in suggested_skills:
            suggested_skills.append("workflow")  # /maestro:refactor

    # Intent: Search
    if any(word in prompt_lower for word in ["search", "find", "look for"]):
        if "ast-grep-find" not in suggested_skills:
            suggested_skills.append("ast-grep-find")

    # Intent: Quality check
    if any(word in prompt_lower for word in ["quality", "review", "check", "audit"]):
        if "qlty-check" not in suggested_skills:
            suggested_skills.append("qlty-check")

    return suggested_skills


def skill_activation_hook(input_data: dict) -> dict:
    """
    User-prompt-submit hook that activates relevant skills.

    Args:
        input_data: Hook input data containing user prompt

    Returns:
        Modified input data with skill suggestions
    """
    try:
        prompt = input_data.get("prompt", "")

        if not prompt:
            return input_data

        # Analyze prompt for skill matches
        suggested_skills = analyze_prompt_for_skills(prompt)

        # Get skill registry for more info
        registry = get_registry()

        skill_info: list[dict[str, Any]] = []
        if registry:
            for skill_name in suggested_skills:
                skill = registry.get_skill(skill_name)
                if skill:
                    skill_info.append({
                        "name": skill_name,
                        "description": skill.get("description", ""),
                        "command": f"/maestro:{skill_name}",
                    })
                else:
                    skill_info.append({
                        "name": skill_name,
                        "command": f"/maestro:{skill_name}",
                    })
        else:
            # Fallback: just list the skill names
            skill_info = [{"name": s, "command": f"/maestro:{s}"} for s in suggested_skills]

        if skill_info:
            input_data["suggested_skills"] = skill_info
            input_data["skill_activation_enabled"] = True

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = skill_activation_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
