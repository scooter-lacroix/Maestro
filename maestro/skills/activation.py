"""
Maestro Skill Activation

Handles skill activation logic for Maestro v2.
Determines which skills should be suggested or required based on user prompts.
"""

import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional

from .registry import (
    SkillDefinition,
    SkillMatch,
    Enforcement,
    Priority,
    get_registry,
    match_skills,
)


class ActivationReason(Enum):
    """Reason for skill activation."""
    KEYWORD_MATCH = "keyword_match"
    INTENT_MATCH = "intent_match"
    COMMAND_PREFIX = "command_prefix"
    HIGH_PRIORITY = "high_priority"
    CONTEXTUAL = "contextual"


@dataclass
class ActivationSuggestion:
    """Suggestion for skill activation."""
    skill: SkillDefinition
    reason: ActivationReason
    message: str
    confidence: float
    blocking: bool = False

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "skill": self.skill.name,
            "category": self.skill.category,
            "type": self.skill.type.value,
            "reason": self.reason.value,
            "message": self.message,
            "confidence": self.confidence,
            "blocking": self.blocking,
            "description": self.skill.description,
        }


@dataclass
class ActivationResult:
    """Result of skill activation analysis."""
    suggestions: List[ActivationSuggestion] = field(default_factory=list)
    blocked: List[str] = field(default_factory=list)
    required: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "suggestions": [s.to_dict() for s in self.suggestions],
            "blocked": self.blocked,
            "required": self.required,
            "has_suggestions": len(self.suggestions) > 0,
            "has_blocking": any(s.blocking for s in self.suggestions),
        }

    def add_suggestion(self, suggestion: ActivationSuggestion) -> None:
        """Add a suggestion, sorting by priority."""
        self.suggestions.append(suggestion)
        # Sort by priority (critical first) then confidence
        priority_order = {
            Priority.CRITICAL: 0,
            Priority.HIGH: 1,
            Priority.MEDIUM: 2,
            Priority.LOW: 3,
        }
        self.suggestions.sort(
            key=lambda s: (priority_order.get(s.skill.priority, 4), -s.confidence)
        )

    def get_blocking(self) -> List[ActivationSuggestion]:
        """Get blocking suggestions."""
        return [s for s in self.suggestions if s.blocking]

    def get_non_blocking(self) -> List[ActivationSuggestion]:
        """Get non-blocking suggestions."""
        return [s for s in self.suggestions if not s.blocking]


class SkillActivator:
    """
    Analyzes prompts and activates appropriate skills.

    The activator:
    1. Parses user prompts for skill triggers
    2. Matches skills based on keywords and intent patterns
    3. Enforces skill requirements and blocks
    4. Returns ordered suggestions for the user
    """

    def __init__(self) -> None:
        """Initialize the skill activator."""
        self._registry = get_registry()
        self._command_pattern = re.compile(r"^/maestro:(\w+)(?:\s|$)")

    def analyze(self, prompt: str) -> ActivationResult:
        """
        Analyze a prompt and return activation suggestions.

        Args:
            prompt: The user prompt to analyze.

        Returns:
            ActivationResult with suggestions and enforcement info.
        """
        result = ActivationResult()

        # Check for explicit command
        command_match = self._command_pattern.search(prompt)
        if command_match:
            command = command_match.group(1)
            self._handle_command(result, command, prompt)
            return result

        # Check for skill matches
        matches = match_skills(prompt, min_confidence=0.3)

        for match in matches:
            skill = match.skill

            # Handle enforcement levels
            if skill.enforcement == Enforcement.BLOCK:
                result.blocked.append(skill.name)
                continue

            if skill.enforcement == Enforcement.REQUIRE:
                result.required.append(skill.name)

            # Create suggestion
            blocking = skill.enforcement == Enforcement.REQUIRE

            # Determine activation reason
            if skill.priority == Priority.CRITICAL:
                reason = ActivationReason.HIGH_PRIORITY
            elif any(kw.lower() in prompt.lower() for kw in skill.triggers.keywords):
                reason = ActivationReason.KEYWORD_MATCH
            else:
                reason = ActivationReason.INTENT_MATCH

            suggestion = ActivationSuggestion(
                skill=skill,
                reason=reason,
                message=self._generate_suggestion_message(skill, match.confidence),
                confidence=match.confidence,
                blocking=blocking,
            )

            result.add_suggestion(suggestion)

        return result

    def _handle_command(
        self,
        result: ActivationResult,
        command: str,
        prompt: str
    ) -> None:
        """Handle an explicit /maestro:command."""
        # Find the skill by command name
        skill = self._registry.get(command)

        if skill:
            suggestion = ActivationSuggestion(
                skill=skill,
                reason=ActivationReason.COMMAND_PREFIX,
                message=f"Explicit command: /maestro:{command}",
                confidence=1.0,
                blocking=False,
            )
            result.add_suggestion(suggestion)
        else:
            # Unknown command - suggest similar skills
            matches = self._registry.match(prompt, min_confidence=0.2, limit=3)
            for match in matches:
                suggestion = ActivationSuggestion(
                    skill=match.skill,
                    reason=ActivationReason.COMMAND_PREFIX,
                    message=f"Did you mean /maestro:{match.skill.name}?",
                    confidence=match.confidence * 0.5,
                    blocking=False,
                )
                result.add_suggestion(suggestion)

    def _generate_suggestion_message(
        self,
        skill: SkillDefinition,
        confidence: float
    ) -> str:
        """Generate a human-readable suggestion message."""
        priority_suffix = ""
        if skill.priority == Priority.CRITICAL:
            priority_suffix = " (highly recommended)"
        elif skill.priority == Priority.HIGH:
            priority_suffix = " (recommended)"

        return f"Consider using /maestro:{skill.name}{priority_suffix}"

    def activate_for_context(
        self,
        prompt: str,
        context: Dict[str, Any]
    ) -> ActivationResult:
        """
        Activate skills based on prompt and additional context.

        Args:
            prompt: The user prompt.
            context: Additional context (file types, project info, etc.)

        Returns:
            ActivationResult with context-aware suggestions.
        """
        result = self.analyze(prompt)

        # Add contextual skill suggestions
        file_type = context.get("file_type")
        if file_type:
            self._add_file_type_suggestions(result, file_type)

        language = context.get("language")
        if language:
            self._add_language_suggestions(result, language)

        return result

    def _add_file_type_suggestions(
        self,
        result: ActivationResult,
        file_type: str
    ) -> None:
        """Add suggestions based on file type."""
        # Each entry is either a list of skill-name strings, or a dict of
        # {skill_name: reason_description}.  Previously the dict variant was
        # silently mishandled — iterating a dict yields its keys, not its items.
        file_type_skills: Dict[str, Any] = {
            "python": ["tdd", "test", "qlty-check"],
            "javascript": ["tdd", "test", "qlty-check"],
            "typescript": ["tdd", "test", "qlty-check"],
            "json": {"validate": "validate JSON structure"},
            "yaml": {"validate": "validate YAML structure"},
        }

        spec = file_type_skills.get(file_type)
        if spec is None:
            return

        if isinstance(spec, dict):
            entries = spec  # {skill_name: reason_msg}
        else:
            entries = {name: f"Recommended for {file_type} files" for name in spec}

        for skill_name, reason_msg in entries.items():
            skill = self._registry.get(skill_name)
            if skill and not any(s.skill.name == skill.name for s in result.suggestions):
                suggestion = ActivationSuggestion(
                    skill=skill,
                    reason=ActivationReason.CONTEXTUAL,
                    message=reason_msg,
                    confidence=0.5,
                    blocking=False,
                )
                result.add_suggestion(suggestion)

    def _add_language_suggestions(
        self,
        result: ActivationResult,
        language: str
    ) -> None:
        """Add suggestions based on programming language."""
        language_skills = {
            "python": ["math-unified", "pint-compute"],
            "javascript": ["tdd"],
            "typescript": ["tdd", "refactor"],
        }

        if language in language_skills:
            for skill_name in language_skills[language]:
                skill = self._registry.get(skill_name)
                if skill and not any(s.skill.name == skill.name for s in result.suggestions):
                    suggestion = ActivationSuggestion(
                        skill=skill,
                        reason=ActivationReason.CONTEXTUAL,
                        message=f"Available for {language}",
                        confidence=0.4,
                        blocking=False,
                    )
                    result.add_suggestion(suggestion)


# Module-level activator singleton — avoids re-compiling the command regex
# on every call to activate_skills_for_prompt().
_activator: Optional["SkillActivator"] = None


def activate_skills_for_prompt(
    prompt: str,
    context: Optional[Dict[str, Any]] = None
) -> ActivationResult:
    """
    Convenience function to activate skills for a prompt.

    Args:
        prompt: The user prompt.
        context: Optional additional context.

    Returns:
        ActivationResult with suggestions.
    """
    global _activator
    if _activator is None:
        _activator = SkillActivator()

    if context:
        return _activator.activate_for_context(prompt, context)
    return _activator.analyze(prompt)
