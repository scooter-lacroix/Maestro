"""
Maestro Skill Loader

Handles loading and parsing of skill definitions from the filesystem.
"""

import re
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .registry import (
    SkillDefinition,
    SkillType,
    Enforcement,
    Priority,
    SkillTrigger,
)


class SkillLoadError(Exception):
    """Error loading a skill."""


class SkillLoader:
    """
    Loads skill definitions from the filesystem.

    Skills are stored as directories containing SKILL.md files
    with YAML frontmatter.
    """

    def __init__(self, skills_dir: Optional[Path] = None):
        """
        Initialize the skill loader.

        Args:
            skills_dir: Path to the skills directory.
        """
        if skills_dir is None:
            skills_dir = Path(__file__).parent

        self.skills_dir = skills_dir

    def load_skill(self, skill_path: Path) -> Optional[SkillDefinition]:
        """
        Load a single skill from its directory.

        Args:
            skill_path: Path to the skill directory.

        Returns:
            SkillDefinition or None if loading failed.
        """
        skill_file = skill_path / "SKILL.md"

        if not skill_file.exists():
            # Try alternative names
            for alt_name in ["skill.md", "README.md"]:
                alt_file = skill_path / alt_name
                if alt_file.exists():
                    skill_file = alt_file
                    break

        if not skill_file.exists():
            return None

        try:
            with open(skill_file, "r", encoding="utf-8") as f:
                content = f.read()

            frontmatter, body = self._parse_frontmatter(content)

            if not frontmatter:
                return None

            # Extract skill name from directory
            name = skill_path.name

            # Determine category
            category = ""
            if skill_path.parent != self.skills_dir:
                category = skill_path.parent.name

            # Create skill definition
            return self._create_definition(name, frontmatter, skill_path, category, body)

        except Exception as e:
            raise SkillLoadError(f"Failed to load skill from {skill_path}: {e}")

    def _parse_frontmatter(self, content: str) -> Tuple[Dict[str, Any], str]:
        """
        Parse YAML frontmatter from markdown content.

        Args:
            content: The markdown content with frontmatter.

        Returns:
            Tuple of (frontmatter_dict, body_content).
        """
        # Check for YAML frontmatter
        frontmatter_match = re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", content, re.DOTALL)

        if not frontmatter_match:
            return {}, content

        frontmatter_str = frontmatter_match.group(1)
        body = frontmatter_match.group(2)

        # Simple YAML parsing for common keys
        frontmatter = {}
        for line in frontmatter_str.split("\n"):
            if ":" in line:
                key, value = line.split(":", 1)
                frontmatter[key.strip()] = value.strip()

        return frontmatter, body

    def _create_definition(
        self,
        name: str,
        frontmatter: Dict[str, Any],
        path: Path,
        category: str,
        body: str
    ) -> SkillDefinition:
        """Create a SkillDefinition from parsed data."""

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

        # Determine type from frontmatter or category
        type_str = frontmatter.get("type", category)
        skill_type = type_map.get(type_str, SkillType.DOMAIN)

        # Map priority
        priority_map = {
            "critical": Priority.CRITICAL,
            "high": Priority.HIGH,
            "medium": Priority.MEDIUM,
            "low": Priority.LOW,
        }
        priority_str = frontmatter.get("priority", "medium").lower()
        skill_priority = priority_map.get(priority_str, Priority.MEDIUM)

        # Create empty triggers (will be loaded from skill-rules.json)
        triggers = SkillTrigger()

        # Map enforcement
        enforcement_map = {
            "suggest": Enforcement.SUGGEST,
            "require": Enforcement.REQUIRE,
            "block": Enforcement.BLOCK,
        }
        enforcement_str = frontmatter.get("enforcement", "suggest").lower()
        skill_enforcement = enforcement_map.get(enforcement_str, Enforcement.SUGGEST)

        return SkillDefinition(
            name=name,
            description=frontmatter.get("description", ""),
            type=skill_type,
            enforcement=skill_enforcement,
            priority=skill_priority,
            triggers=triggers,
            category=category,
            path=path,
            frontmatter=frontmatter,
        )

    def load_all_skills(self) -> Dict[str, SkillDefinition]:
        """
        Load all skills from the skills directory.

        Returns:
            Dictionary mapping skill names to SkillDefinitions.
        """
        skills = {}

        # Load from category directories
        for category_dir in self.skills_dir.iterdir():
            if not category_dir.is_dir():
                continue

            # Skip non-category directories
            if category_dir.name.startswith("_"):
                continue

            for skill_dir in category_dir.iterdir():
                if not skill_dir.is_dir():
                    continue

                skill = self.load_skill(skill_dir)
                if skill:
                    skills[skill.name] = skill

        # Load from root level
        for skill_dir in self.skills_dir.iterdir():
            if not skill_dir.is_dir():
                continue

            # Skip directories that are categories or special
            if skill_dir.name in [
                "meta", "context", "analysis", "research", "quality", "planning", "math",
                "agents", "workflow", "hooks", "developer", "__pycache__"
            ]:
                continue

            skill = self.load_skill(skill_dir)
            if skill:
                skills[skill.name] = skill

        return skills

    def get_skill_content(self, skill_name: str) -> Optional[str]:
        """
        Get the full content (SKILL.md) of a skill.

        Args:
            skill_name: Name of the skill.

        Returns:
            The skill content or None if not found.
        """
        # Try to find the skill
        skill_path = self._find_skill_path(skill_name)

        if not skill_path:
            return None

        skill_file = skill_path / "SKILL.md"
        if not skill_file.exists():
            for alt_name in ["skill.md", "README.md"]:
                alt_file = skill_path / alt_name
                if alt_file.exists():
                    skill_file = alt_file
                    break

        if not skill_file.exists():
            return None

        with open(skill_file, "r", encoding="utf-8") as f:
            return f.read()

    def _find_skill_path(self, skill_name: str) -> Optional[Path]:
        """Find the path to a skill directory."""
        # Check in categories first
        for category_dir in self.skills_dir.iterdir():
            if not category_dir.is_dir():
                continue

            skill_path = category_dir / skill_name
            if skill_path.exists() and skill_path.is_dir():
                return skill_path

        # Check root level
        skill_path = self.skills_dir / skill_name
        if skill_path.exists() and skill_path.is_dir():
            return skill_path

        return None

    def list_skills(self) -> List[str]:
        """List all available skill names."""
        skills = set()

        for category_dir in self.skills_dir.iterdir():
            if not category_dir.is_dir():
                continue

            if category_dir.name.startswith("_"):
                continue

            for skill_dir in category_dir.iterdir():
                if skill_dir.is_dir():
                    skills.add(skill_dir.name)

        # Root level skills
        for skill_dir in self.skills_dir.iterdir():
            if not skill_dir.is_dir():
                continue

            if skill_dir.name in [
                "meta", "context", "analysis", "research", "quality", "planning", "math",
                "agents", "workflow", "hooks", "developer", "__pycache__"
            ]:
                continue

            skills.add(skill_dir.name)

        return sorted(skills)

    def get_categories(self) -> List[str]:
        """Get all skill categories."""
        categories = []

        for item in self.skills_dir.iterdir():
            if item.is_dir() and not item.name.startswith("_"):
                # Check if it contains skills
                has_skills = any(
                    (item / s).is_dir() and (item / s / "SKILL.md").exists()
                    for s in item.iterdir() if s.is_dir()
                )
                if has_skills:
                    categories.append(item.name)

        return sorted(categories)


def load_all_skills(skills_dir: Optional[Path] = None) -> Dict[str, SkillDefinition]:
    """
    Convenience function to load all skills.

    Args:
        skills_dir: Optional path to skills directory.

    Returns:
        Dictionary mapping skill names to SkillDefinitions.
    """
    loader = SkillLoader(skills_dir)
    return loader.load_all_skills()
