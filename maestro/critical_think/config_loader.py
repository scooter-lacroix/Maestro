"""
Configuration loader for Critical Think framework.

This module handles loading, validating, and accessing configuration for the
Critical Think metacognitive framework integration.
"""

import yaml  # type: ignore[import-untyped]  # type: ignore
from pathlib import Path
from typing import Dict, Any, Optional
import logging
import copy

logger = logging.getLogger(__name__)

# Default configuration structure
DEFAULT_CONFIG: Dict[str, Any] = {
    "enabled": True,
    "integration_points": {
        "before_question": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Analyze before asking clarifying questions"
        },
        "after_question": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Validate after receiving answers"
        },
        "before_docs": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Analyze before generating documentation"
        },
        "after_docs": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Validate documentation quality"
        },
        "before_implementation": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Analyze before implementing code"
        },
        "after_implementation": {
            "enabled": True,
            "confidence_threshold": 8,
            "description": "Validate implementation quality"
        },
        "before_agent_delegation": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Analyze before delegating to agents"
        },
        "after_agent_delegation": {
            "enabled": True,
            "confidence_threshold": 7,
            "description": "Validate agent results"
        }
    },
    "confidence_thresholds": {
        "critical": 4,
        "warning": 6,
        "acceptable": 7,
        "high": 9
    },
    "output": {
        "format": "detailed",
        "show_confidence": True,
        "show_all_steps": True,
        "show_risks": True,
        "highlight_pitfalls": True,
        "show_revised_confidence": True
    },
    "behavior": {
        "auto_proceed": True,
        "require_confirmation_on_low_confidence": True,
        "max_assumptions": 3,
        "max_risks": 3,
        "verbose": True
    },
    "templates": {
        "before_action": "maestro/critical_think/templates/criticalthink_before_action.md",
        "after_action": "maestro/critical_think/templates/criticalthink_after_action.md",
        "question": "maestro/critical_think/templates/criticalthink_question.md",
        "docs": "maestro/critical_think/templates/criticalthink_docs.md",
        "implementation": "maestro/critical_think/templates/criticalthink_implementation.md",
        "agent_delegation": "maestro/critical_think/templates/criticalthink_agent_delegation.md"
    },
    "maestro_integration": {
        "tdd_integration": True,
        "agent_selection_integration": True,
        "oracle_review_integration": True,
        "track_confidence_in_plans": True,
        "include_in_commits": False
    },
    "advanced": {
        "learning_mode": False,
        "caching": False,
        "parallel_analysis": False,
        "custom_parameters": {}
    }
}

# Global configuration cache
_config_cache: Optional[Dict[str, Any]] = None


def load_config(config_path: Optional[str] = None) -> Dict[str, Any]:
    """
    Load configuration from YAML file.

    Args:
        config_path: Path to configuration file. If None, uses default path.

    Returns:
        Configuration dictionary

    Raises:
        FileNotFoundError: If config file doesn't exist
        yaml.YAMLError: If config file is invalid YAML
        ValueError: If config is invalid
    """
    global _config_cache

    if config_path is None:
        # Try default locations
        default_paths = [
            "maestro/critical_think/config.yaml",
            Path(__file__).parent / "config.yaml",
            Path.cwd() / "config.yaml",
        ]

        config_path = None
        for path in default_paths:
            p = Path(str(path))
            if p.exists():
                config_path = str(p)
                break

    if config_path is None or not Path(config_path).exists():
        logger.warning("No config file found, using default configuration")
        _config_cache = copy.deepcopy(DEFAULT_CONFIG)
        return _config_cache

    try:
        with open(config_path, 'r', encoding="utf-8") as f:
            user_config = yaml.safe_load(f)

        if user_config is None:
            user_config = {}

        # Merge with defaults
        config = _merge_configs(DEFAULT_CONFIG, user_config)

        # Validate the merged config
        validate_config(config)

        _config_cache = config
        logger.info(f"Loaded configuration from {config_path}")
        return config

    except FileNotFoundError:
        logger.error(f"Configuration file not found: {config_path}")
        raise
    except yaml.YAMLError as e:
        logger.error(f"Invalid YAML in configuration file: {e}")
        raise
    except ValueError as e:
        logger.error(f"Configuration validation failed: {e}")
        raise


def _merge_configs(default: Dict[str, Any], user: Dict[str, Any]) -> Dict[str, Any]:
    """
    Deep merge user config with default config.

    Args:
        default: Default configuration
        user: User configuration

    Returns:
        Merged configuration
    """
    result = copy.deepcopy(default)

    for key, value in user.items():
        if key in result and isinstance(result[key], dict) and isinstance(value, dict):
            result[key] = _merge_configs(result[key], value)
        else:
            result[key] = value

    return result


def validate_config(config: Dict[str, Any]) -> None:
    """
    Validate configuration structure and values.

    Args:
        config: Configuration dictionary

    Raises:
        ValueError: If configuration is invalid
    """
    errors = []

    # Check enabled flag
    if "enabled" not in config:
        errors.append("Missing 'enabled' field")
    elif not isinstance(config["enabled"], bool):
        errors.append("'enabled' must be a boolean")

    # Check integration_points
    if "integration_points" not in config:
        errors.append("Missing 'integration_points' section")
    elif not isinstance(config["integration_points"], dict):
        errors.append("'integration_points' must be a dictionary")
    else:
        for point_name, point_config in config["integration_points"].items():
            if not isinstance(point_config, dict):
                errors.append(f"Integration point '{point_name}' must be a dictionary")
                continue

            if "enabled" not in point_config:
                errors.append(f"Integration point '{point_name}' missing 'enabled' field")
            elif not isinstance(point_config.get("enabled"), bool):
                errors.append(f"Integration point '{point_name}' 'enabled' must be boolean")

            if "confidence_threshold" not in point_config:
                errors.append(f"Integration point '{point_name}' missing 'confidence_threshold'")
            else:
                threshold = point_config["confidence_threshold"]
                if not isinstance(threshold, int) or not 1 <= threshold <= 10:
                    errors.append(f"Integration point '{point_name}' confidence_threshold must be 1-10")

    # Check confidence_thresholds
    if "confidence_thresholds" in config:
        thresholds = config["confidence_thresholds"]
        for key in ["critical", "warning", "acceptable", "high"]:
            if key in thresholds:
                value = thresholds[key]
                if not isinstance(value, int) or not 1 <= value <= 10:
                    errors.append(f"confidence_thresholds.{key} must be 1-10")

    # Check templates
    if "templates" in config:
        templates = config["templates"]
        required_templates = ["before_action", "after_action"]
        for template_name in required_templates:
            if template_name not in templates:
                errors.append(f"Missing required template: {template_name}")

    # Check output format
    if "output" in config and "format" in config["output"]:
        valid_formats = ["detailed", "summary", "minimal"]
        if config["output"]["format"] not in valid_formats:
            errors.append(f"output.format must be one of {valid_formats}")

    if errors:
        raise ValueError("Configuration validation failed:\n" + "\n".join(f"  - {e}" for e in errors))


def get_template_path(template_name: str, config: Dict[str, Any]) -> str:
    """
    Get path to template file from config.

    Args:
        template_name: Name of template (e.g., 'before_action', 'after_action')
        config: Configuration dictionary

    Returns:
        Path to template file

    Raises:
        ValueError: If template not found in config
        FileNotFoundError: If template file doesn't exist
    """
    if "templates" not in config:
        raise ValueError("Configuration missing 'templates' section")

    templates = config["templates"]

    if template_name not in templates:
        raise ValueError(f"Template '{template_name}' not found in configuration")

    template_path = templates[template_name]

    # Check if path is relative or absolute
    path_obj = Path(template_path)

    # Try to resolve relative paths
    if not path_obj.is_absolute():
        # Try relative to current directory
        if path_obj.exists():
            return str(path_obj)

        # Try relative to module directory
        module_dir = Path(__file__).parent
        module_path = module_dir / template_path
        if module_path.exists():
            return str(module_path)

        # Try relative to project root
        project_root = Path.cwd()
        project_path = project_root / template_path
        if project_path.exists():
            return str(project_path)
    else:
        if path_obj.exists():
            return str(path_obj)

    # If not found, raise error
    raise FileNotFoundError(f"Template file not found: {template_path}")


def is_enabled(config: Optional[Dict[str, Any]] = None) -> bool:
    """
    Check if Critical Think is globally enabled.

    Args:
        config: Configuration dictionary. If None, loads from default location.

    Returns:
        True if enabled, False otherwise
    """
    if config is None:
        config = load_config()

    return bool(config.get("enabled", True))


def is_integration_point_enabled(
    integration_point: str,
    config: Optional[Dict[str, Any]] = None
) -> bool:
    """
    Check if a specific integration point is enabled.

    Args:
        integration_point: Name of integration point (e.g., 'before_implementation')
        config: Configuration dictionary. If None, loads from default location.

    Returns:
        True if enabled, False otherwise
    """
    if config is None:
        config = load_config()

    # Check global enabled flag first
    if not config.get("enabled", True):
        return False

    # Check integration point specific flag
    integration_points = config.get("integration_points", {})
    point_config = integration_points.get(integration_point, {})

    return bool(point_config.get("enabled", True))


def get_confidence_threshold(
    integration_point: str,
    config: Optional[Dict[str, Any]] = None
) -> int:
    """
    Get confidence threshold for an integration point.

    Args:
        integration_point: Name of integration point
        config: Configuration dictionary. If None, loads from default location.

    Returns:
        Confidence threshold (1-10)
    """
    if config is None:
        config = load_config()

    integration_points = config.get("integration_points", {})
    point_config = integration_points.get(integration_point, {})

    return int(point_config.get("confidence_threshold", 7))


def reload_config() -> Dict[str, Any]:
    """
    Force reload configuration from file.

    Returns:
        Reloaded configuration dictionary
    """
    global _config_cache
    _config_cache = None
    return load_config()


def get_config() -> Dict[str, Any]:
    """
    Get cached configuration or load if not cached.

    Returns:
        Configuration dictionary
    """
    global _config_cache
    if _config_cache is None:
        _config_cache = load_config()
    return _config_cache
