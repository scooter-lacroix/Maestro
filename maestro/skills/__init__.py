"""
Maestro Skills System

Provides skill registry, loader, and activation logic for 109+ Maestro skills.
Skills are organized into categories: meta, context, analysis, research, quality, planning, math.
"""

from .registry import SkillRegistry, get_registry, reset_registry, load_skill, match_skills
from .activation import SkillActivator, activate_skills_for_prompt
from .loader import SkillLoader, load_all_skills

__all__ = [
    "SkillRegistry",
    "get_registry",
    "reset_registry",
    "load_skill",
    "match_skills",
    "SkillActivator",
    "activate_skills_for_prompt",
    "SkillLoader",
    "load_all_skills",
]

__version__ = "2.0.0"
