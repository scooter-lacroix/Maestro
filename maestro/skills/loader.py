"""
Maestro Skill Loader

Handles loading and parsing of skill definitions from the filesystem.
"""

import yaml
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


# Directories inside skills/ that are NOT skill directories themselves.
# We detect skill dirs by the presence of SKILL.md, so this list is used only
# to skip non-skill infrastructure folders at the root of skills/.
_INFRA_DIRS = frozenset(
    {"__pycache__", "tests", "_sandbox", "agents", "hooks", "developer"}
)


class SkillLoader:
    """
    Loads skill definitions from the filesystem.

    Skills are stored as directories containing SKILL.md files
    with YAML frontmatter.  The loader supports arbitrary nesting depth
    (e.g. math/math/linear-algebra/matrices/SKILL.md).
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

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

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

            # Determine category from relative directory structure
            rel = skill_path.relative_to(self.skills_dir)
            parts = list(rel.parts)
            if not parts:
                return None
            # The skill name is always the leaf directory name
            name = parts[-1]
            # Category is the first ancestor under skills_dir, if any
            category = parts[0] if len(parts) > 1 else ""

            return self._create_definition(name, frontmatter, skill_path, category, body)

        except Exception as e:
            raise SkillLoadError(f"Failed to load skill from {skill_path}: {e}")

    def load_all_skills(self) -> Dict[str, SkillDefinition]:
        """
        Load all skills from the skills directory.

        Uses recursive rglob so skills at any nesting depth are discovered:
          skills_dir/skill-name/SKILL.md
          skills_dir/category/skill-name/SKILL.md
          skills_dir/category/subcategory/skill-name/SKILL.md   ← previously missed
          skills_dir/math/math/linear-algebra/matrices/SKILL.md ← previously missed

        Also discovers skills with fallback names (skill.md, README.md).

        Returns:
            Dictionary mapping skill names to SkillDefinitions.
        """
        skills: Dict[str, SkillDefinition] = {}

        # Collect unique skill directories (primary + fallback names)
        skill_paths = {p.parent for p in self.skills_dir.rglob("SKILL.md")}
        for fallback_name in ["skill.md", "README.md"]:
            skill_paths.update(p.parent for p in self.skills_dir.rglob(fallback_name))

        for skill_path in sorted(skill_paths):

            # Skip infrastructure directories at root of skills dir
            rel_parts = skill_path.relative_to(self.skills_dir).parts
            if rel_parts and rel_parts[0] in _INFRA_DIRS:
                continue

            try:
                skill = self.load_skill(skill_path)
            except SkillLoadError as e:
                import warnings
                warnings.warn(str(e), stacklevel=2)
                continue

            if skill is None:
                continue

            # Warn on name collision instead of silently overwriting
            if skill.name in skills:
                existing = skills[skill.name]
                import warnings
                warnings.warn(
                    f"Skill name collision: '{skill.name}' loaded from both "
                    f"'{existing.path}' and '{skill.path}'. "
                    "The later one (alphabetically) will take precedence.",
                    stacklevel=2,
                )

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

    def list_skills(self) -> List[str]:
        """List all available skill names."""
        return sorted(self.load_all_skills().keys())

    def get_categories(self) -> List[str]:
        """Get all skill categories."""
        categories: set = set()
        for skill in self.load_all_skills().values():
            if skill.category:
                categories.add(skill.category)
        return sorted(categories)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _parse_frontmatter(self, content: str) -> Tuple[Dict[str, Any], str]:
        """
        Parse YAML frontmatter from markdown content using yaml.safe_load.

        Args:
            content: The markdown content with frontmatter.

        Returns:
            Tuple of (frontmatter_dict, body_content).
        """
        if not content.startswith("---"):
            return {}, content

        # Find the closing ---
        rest = content[3:]
        close_idx = rest.find("\n---")
        if close_idx == -1:
            return {}, content

        frontmatter_str = rest[:close_idx]
        body = rest[close_idx + 4:].lstrip("\n")

        try:
            frontmatter = yaml.safe_load(frontmatter_str) or {}
        except yaml.YAMLError:
            return {}, content

        if not isinstance(frontmatter, dict):
            return {}, content

        return frontmatter, body

    def _find_skill_path(self, skill_name: str) -> Optional[Path]:
        """Find the path to a skill directory by name (recursive)."""
        # Check primary SKILL.md first
        for skill_file in self.skills_dir.rglob("SKILL.md"):
            if skill_file.parent.name == skill_name:
                return skill_file.parent
        # Check fallback names
        for fallback_name in ["skill.md", "README.md"]:
            for skill_file in self.skills_dir.rglob(fallback_name):
                if skill_file.parent.name == skill_name:
                    return skill_file.parent
        return None

    def _create_definition(
        self,
        name: str,
        frontmatter: Dict[str, Any],
        path: Path,
        category: str,
        body: str
    ) -> SkillDefinition:
        """Create a SkillDefinition from parsed data."""

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
        skill_type = type_map.get(str(type_str).lower(), SkillType.DOMAIN)

        priority_map = {
            "critical": Priority.CRITICAL,
            "high": Priority.HIGH,
            "medium": Priority.MEDIUM,
            "low": Priority.LOW,
        }
        priority_str = str(frontmatter.get("priority", "medium")).lower()
        skill_priority = priority_map.get(priority_str, Priority.MEDIUM)

        # Create empty triggers (will be merged from skill-rules.json by registry)
        triggers = SkillTrigger()

        enforcement_map = {
            "suggest": Enforcement.SUGGEST,
            "require": Enforcement.REQUIRE,
            "block": Enforcement.BLOCK,
        }
        enforcement_str = str(frontmatter.get("enforcement", "suggest")).lower()
        skill_enforcement = enforcement_map.get(enforcement_str, Enforcement.SUGGEST)

        # Parse user-invocable field (was previously ignored)
        raw_invocable = frontmatter.get("user-invocable", None)
        if raw_invocable is None:
            user_invocable = True  # default: user can invoke
        elif isinstance(raw_invocable, bool):
            user_invocable = raw_invocable
        else:
            user_invocable = str(raw_invocable).lower() not in ("false", "no", "0")

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
            user_invocable=user_invocable,
        )


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
