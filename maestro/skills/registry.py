"""
Maestro Skill Registry

Manages skill registration, lookup, and matching for Maestro v2.
"""

import json
import re
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional


class SkillType(Enum):
    """Types of skills in the Maestro system."""
    WORKFLOW = "workflow"
    DOMAIN = "domain"
    META = "meta"
    CONTEXT = "context"
    ANALYSIS = "analysis"
    RESEARCH = "research"
    QUALITY = "quality"
    PLANNING = "planning"
    MATH = "math"


class Enforcement(Enum):
    """Skill enforcement levels."""
    SUGGEST = "suggest"
    REQUIRE = "require"
    BLOCK = "block"


class Priority(Enum):
    """Skill priority levels."""
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"


@dataclass
class SkillTrigger:
    """Trigger configuration for a skill."""
    keywords: List[str] = field(default_factory=list)
    intent_patterns: List[str] = field(default_factory=list)

    def matches(self, prompt: str) -> float:
        """
        Check if this trigger matches the given prompt.

        Returns a confidence score between 0.0 and 1.0.
        """
        score = 0.0
        prompt_lower = prompt.lower()

        # Check keyword matches
        for keyword in self.keywords:
            if keyword.lower() in prompt_lower:
                score += 0.3

        # Check intent pattern matches
        for pattern in self.intent_patterns:
            try:
                if re.search(pattern, prompt, re.IGNORECASE):
                    score += 0.4
            except re.error:
                pass

        return min(score, 1.0)


@dataclass
class SkillDefinition:
    """Definition of a Maestro skill."""
    name: str
    description: str
    type: SkillType
    enforcement: Enforcement
    priority: Priority
    triggers: SkillTrigger
    category: str
    path: Path
    frontmatter: Dict[str, Any] = field(default_factory=dict)
    user_invocable: bool = True  # False = internal-only; not shown in /help

    @classmethod
    def from_dict(cls, name: str, data: Dict[str, Any], skills_dir: Path) -> "SkillDefinition":
        """Create a SkillDefinition from skill-rules.json data."""
        # Determine category and path
        skill_path = skills_dir / name
        if not skill_path.exists():
            # Try to find in subdirectories
            for category_dir in skills_dir.iterdir():
                if category_dir.is_dir() and (category_dir / name).exists():
                    skill_path = category_dir / name
                    break

        # Map type string to enum
        type_map = {
            "workflow": SkillType.WORKFLOW,
            "domain": SkillType.DOMAIN,
            "meta": SkillType.META,
            "context": SkillType.CONTEXT,
            "analysis": SkillType.ANALYSIS,
            "research": SkillType.RESEARCH,
            "quality": SkillType.QUALITY,
            "planning": SkillType.PLANNING,
            "math": SkillType.MATH,
        }

        # Map enforcement string to enum
        enforcement_map = {
            "suggest": Enforcement.SUGGEST,
            "require": Enforcement.REQUIRE,
            "block": Enforcement.BLOCK,
        }

        # Map priority string to enum
        priority_map = {
            "critical": Priority.CRITICAL,
            "high": Priority.HIGH,
            "medium": Priority.MEDIUM,
            "low": Priority.LOW,
        }

        # Parse triggers
        trigger_data = data.get("promptTriggers", {})
        triggers = SkillTrigger(
            keywords=trigger_data.get("keywords", []),
            intent_patterns=trigger_data.get("intentPatterns", [])
        )

        # Determine category from path
        category = skill_path.parent.name if skill_path.parent != skills_dir else ""

        return cls(
            name=name,
            description=data.get("description", ""),
            type=type_map.get(data.get("type", "domain"), SkillType.DOMAIN),
            enforcement=enforcement_map.get(data.get("enforcement", "suggest"), Enforcement.SUGGEST),
            priority=priority_map.get(data.get("priority", "medium"), Priority.MEDIUM),
            triggers=triggers,
            category=category,
            path=skill_path,
        )

    def matches(self, prompt: str) -> float:
        """Check if this skill matches the given prompt."""
        return self.triggers.matches(prompt)


@dataclass
class SkillMatch:
    """Result of skill matching."""
    skill: SkillDefinition
    confidence: float
    reason: str = ""


class SkillRegistry:
    """
    Registry for all Maestro skills.

    Manages skill registration, lookup, and matching based on
    user prompts and skill triggers.
    """

    def __init__(self, skills_dir: Optional[Path] = None):
        """
        Initialize the skill registry.

        Args:
            skills_dir: Path to the skills directory. Defaults to maestro/skills/
        """
        if skills_dir is None:
            skills_dir = Path(__file__).parent

        self.skills_dir = skills_dir
        self._skills: Dict[str, SkillDefinition] = {}
        self._by_category: Dict[str, List[str]] = {}
        self._by_type: Dict[SkillType, List[str]] = {}

    @property
    def skills(self) -> Dict[str, SkillDefinition]:
        """Get all registered skills."""
        return self._skills.copy()

    def register(self, skill: SkillDefinition) -> None:
        """
        Register a skill in the registry.

        Args:
            skill: The skill definition to register.
        """
        self._skills[skill.name] = skill

        # Index by category
        if skill.category not in self._by_category:
            self._by_category[skill.category] = []
        if skill.name not in self._by_category[skill.category]:
            self._by_category[skill.category].append(skill.name)

        # Index by type
        if skill.type not in self._by_type:
            self._by_type[skill.type] = []
        if skill.name not in self._by_type[skill.type]:
            self._by_type[skill.type].append(skill.name)

    def get(self, name: str) -> Optional[SkillDefinition]:
        """Get a skill by name."""
        return self._skills.get(name)

    def get_by_category(self, category: str) -> List[SkillDefinition]:
        """Get all skills in a category."""
        names = self._by_category.get(category, [])
        return [self._skills[name] for name in names if name in self._skills]

    def get_by_type(self, skill_type: SkillType) -> List[SkillDefinition]:
        """Get all skills of a type."""
        names = self._by_type.get(skill_type, [])
        return [self._skills[name] for name in names if name in self._skills]

    def match(
        self,
        prompt: str,
        min_confidence: float = 0.3,
        limit: int = 5
    ) -> List[SkillMatch]:
        """
        Find skills that match the given prompt.

        Args:
            prompt: The user prompt to match against.
            min_confidence: Minimum confidence threshold.
            limit: Maximum number of results to return.

        Returns:
            List of skill matches sorted by confidence.
        """
        matches = []

        for skill in self._skills.values():
            confidence = skill.matches(prompt)
            if confidence >= min_confidence:
                matches.append(SkillMatch(
                    skill=skill,
                    confidence=confidence,
                    reason=f"Matched on {int(confidence * 100)}% confidence"
                ))

        # Sort by confidence and limit results
        matches.sort(key=lambda m: m.confidence, reverse=True)
        return matches[:limit]

    def load_rules(self, rules_file: Optional[Path] = None) -> int:
        """
        Load skill definitions from skill-rules.json.

        Args:
            rules_file: Path to skill-rules.json. Defaults to skills_dir/skill-rules.json

        Returns:
            Number of skills loaded.
        """
        if rules_file is None:
            rules_file = self.skills_dir / "skill-rules.json"

        if not rules_file.exists():
            return 0

        with open(rules_file, "r", encoding="utf-8") as f:
            data = json.load(f)

        skills_data = data.get("skills", {})
        count = 0

        for name, skill_data in skills_data.items():
            skill = SkillDefinition.from_dict(name, skill_data, self.skills_dir)
            self.register(skill)
            count += 1

        return count

    def list_categories(self) -> List[str]:
        """Get list of all categories with skills."""
        return sorted(self._by_category.keys())

    def list_types(self) -> List[SkillType]:
        """Get list of all skill types."""
        return list(self._by_type.keys())

    def stats(self) -> Dict[str, Any]:
        """Get registry statistics."""
        return {
            "total_skills": len(self._skills),
            "categories": self.list_categories(),
            "types": [t.value for t in self.list_types()],
            "by_category": {
                cat: len(skills)
                for cat, skills in self._by_category.items()
            },
            "by_type": {
                t.value: len(skills)
                for t, skills in self._by_type.items()
            }
        }


# Global registry instance
_registry: Optional[SkillRegistry] = None
_registry_skills_dir: Optional[Path] = None


def get_registry(skills_dir: Optional[Path] = None) -> SkillRegistry:
    """
    Get the global skill registry instance.

    Args:
        skills_dir: Path to skills directory. Only used on first call.
                    Subsequent calls with a *different* path will log a warning
                    and return the already-initialised singleton.

    Returns:
        The global SkillRegistry instance.
    """
    global _registry, _registry_skills_dir

    if _registry is None:
        _registry = SkillRegistry(skills_dir)
        _registry_skills_dir = skills_dir
        _registry.load_rules()
    elif skills_dir is not None and skills_dir != _registry_skills_dir:
        import warnings
        warnings.warn(
            f"get_registry() called with skills_dir={skills_dir!r} but the "
            f"singleton was already created with skills_dir={_registry_skills_dir!r}. "
            "The existing singleton is returned unchanged. Call reset_registry() first "
            "if you need a different skills directory.",
            stacklevel=2,
        )

    return _registry


def reset_registry() -> None:
    """
    Reset the global skill registry singleton.

    Useful in tests and when skills_dir needs to change at runtime.
    The next call to get_registry() will create a fresh instance.
    """
    global _registry, _registry_skills_dir
    _registry = None
    _registry_skills_dir = None


def load_skill(name: str) -> Optional[SkillDefinition]:
    """Load a skill by name from the registry."""
    registry = get_registry()
    return registry.get(name)


def match_skills(
    prompt: str,
    min_confidence: float = 0.3,
    limit: int = 5
) -> List[SkillMatch]:
    """Match skills against a prompt."""
    registry = get_registry()
    return registry.match(prompt, min_confidence, limit)
