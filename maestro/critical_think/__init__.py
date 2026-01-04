"""
Maestro Critical Think Module

Integrates slash-criticalthink metacognitive framework into Maestro workflow.
Provides hooks for systematic self-critique before and after key workflow actions.

This module implements a 6-step critical thinking framework:
1. Core Thesis & Confidence Assessment
2. Foundational Analysis (assumptions)
3. Logical Integrity (premise mapping)
4. AI-Specific Pitfall Analysis
5. Risk & Mitigation
6. Synthesis & Revised Recommendation

Example:
    >>> from maestro.critical_think import invoke_before, invoke_after
    >>>
    >>> # Pre-action analysis
    >>> result = invoke_before(
    ...     action_description="Add user authentication",
    ...     context="Web application using JWT tokens"
    ... )
    >>>
    >>> # Post-action analysis
    >>> result = invoke_after(
    ...     action_description="Add user authentication",
    ...     original_plan="Implement JWT auth with refresh tokens",
    ...     actual_result="Auth implemented but missing rate limiting"
    ... )
    >>>
    >>> # Format results
    >>> from maestro.critical_think import format_synthesis
    >>> print(format_synthesis(result))

Security:
    This module includes input validation and sanitization to prevent
    injection attacks through template variables. All string inputs are
    validated for length and dangerous patterns.
"""

from typing import Dict, List
from .core import (
    CriticalThinkEngine,
    CriticalThinkResult,
    ConfidenceLevel,
    invoke_before,
    invoke_after,
    calculate_confidence,
    format_synthesis,
    get_engine,
    # Security functions
    validate_string_input,
    sanitize_for_logging,
)
from .config_loader import (
    validate_config,
    load_config,
    get_config,
    is_enabled,
    is_integration_point_enabled,
    get_confidence_threshold,
    get_template_path,
    reload_config,
)
from .native_integration import (
    NativeCriticalThinkIntegration,
    NativeAnalysisRequest,
    analyze_native,
    get_native_integration,
)

# Version information
__version__: str = "2.0.0"  # Updated for native integration
__author__: str = "Maestro Framework"
__all__: List[str] = [
    "CriticalThinkEngine",
    "CriticalThinkResult",
    "ConfidenceLevel",
    "invoke_before",
    "invoke_after",
    "calculate_confidence",
    "format_synthesis",
    "get_engine",
    # Security utilities
    "validate_string_input",
    "sanitize_for_logging",
    "validate_config",
    "load_config",
    "get_config",
    "is_enabled",
    "is_integration_point_enabled",
    "get_confidence_threshold",
    "get_template_path",
    "reload_config",
    # Native integration
    "NativeCriticalThinkIntegration",
    "NativeAnalysisRequest",
    "analyze_native",
    "get_native_integration",
    # Constants
    "DEFAULT_CONFIDENCE",
    "MIN_CONFIDENCE",
    "MAX_CONFIDENCE",
    "MAX_INPUT_LENGTH",
    "NOT_APPLICABLE",
    "CONFIDENCE_BOUNDARIES",
    "TEMPLATE_BEFORE_ACTION",
    "TEMPLATE_AFTER_ACTION",
]

# Module-level constants for easy access
from .core import (
    DEFAULT_CONFIDENCE,
    MIN_CONFIDENCE,
    MAX_CONFIDENCE,
    MAX_INPUT_LENGTH,
    NOT_APPLICABLE,
    CONFIDENCE_BOUNDARIES,
    TEMPLATE_BEFORE_ACTION,
    TEMPLATE_AFTER_ACTION,
)
