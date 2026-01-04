"""
Critical Think Core Engine

Provides the 6-step metacognitive framework for systematic self-critique.
"""

from enum import Enum
from typing import Optional, Dict, List, Union
from dataclasses import dataclass
import json
import logging
import re
import threading
from string import Template
from pathlib import Path

# Configure logging
logger = logging.getLogger(__name__)

# Import config loader
from .config_loader import (
    load_config,
    get_config,
    is_enabled,
    is_integration_point_enabled,
    get_confidence_threshold,
    get_template_path
)


# ============================================================================
# Constants
# ============================================================================

# Confidence score thresholds
DEFAULT_CONFIDENCE: int = 5
MIN_CONFIDENCE: int = 1
MAX_CONFIDENCE: int = 10

# Confidence level boundaries (upper bounds, inclusive)
CONFIDENCE_BOUNDARIES: Dict[str, int] = {
    "very_low": 2,    # Scores 1-2
    "low": 4,         # Scores 3-4
    "medium": 6,      # Scores 5-6
    "high": 8,        # Scores 7-8
    "very_high": 10   # Scores 9-10
}

# Validation constraints
MAX_INPUT_LENGTH: int = 10000
NOT_APPLICABLE: str = "N/A"

# Template keys
TEMPLATE_BEFORE_ACTION: str = "before_action"
TEMPLATE_AFTER_ACTION: str = "after_action"


# ============================================================================
# Security: Input Validation
# ============================================================================

def validate_string_input(value: str, name: str, max_length: int = MAX_INPUT_LENGTH) -> str:
    """
    Validate string input for security and integrity.

    Args:
        value: The string value to validate
        name: Parameter name for error messages
        max_length: Maximum allowed length (default: MAX_INPUT_LENGTH)

    Returns:
        The validated string

    Raises:
        TypeError: If value is not a string
        ValueError: If value is empty, exceeds max length, or contains invalid characters
    """
    if not isinstance(value, str):
        raise TypeError(f"{name} must be str, got {type(value).__name__}")

    if len(value) == 0:
        raise ValueError(f"{name} cannot be empty")

    if len(value) > max_length:
        raise ValueError(f"{name} exceeds {max_length} characters (got {len(value)})")

    # Check for potential injection patterns
    dangerous_patterns = [
        r'\${',       # Template injection attempt
        r'__import__', # Python import injection
        r'eval\(',    # eval injection
        r'exec\(',    # exec injection
    ]

    for pattern in dangerous_patterns:
        if re.search(pattern, value, re.IGNORECASE):
            raise ValueError(f"{name} contains potentially dangerous pattern: {pattern}")

    return value


def sanitize_for_logging(text: str, max_length: int = 100) -> str:
    """
    Sanitize text for safe logging (truncate and remove sensitive data).

    Args:
        text: Text to sanitize
        max_length: Maximum length to return

    Returns:
        Sanitized text safe for logging
    """
    if not isinstance(text, str):
        return str(type(text).__name__)

    # Remove potential sensitive patterns (basic implementation)
    # In production, you might want more sophisticated redaction
    sanitized = re.sub(r'(password|token|key|secret)=\S+', r'\1=***', text, flags=re.IGNORECASE)

    # Truncate
    if len(sanitized) > max_length:
        return sanitized[:max_length] + "..."

    return sanitized


# Note: validate_config and load_config are now imported from config_loader module


# ============================================================================
# Core Classes
# ============================================================================

class ConfidenceLevel(Enum):
    """Confidence level classification based on score.

    Attributes:
        VERY_LOW: Confidence scores 1-2 (high uncertainty)
        LOW: Confidence scores 3-4 (significant uncertainty)
        MEDIUM: Confidence scores 5-6 (moderate confidence)
        HIGH: Confidence scores 7-8 (good confidence)
        VERY_HIGH: Confidence scores 9-10 (very certain)
    """

    VERY_LOW = 1  # Score 1-2
    LOW = 2  # Score 3-4
    MEDIUM = 3  # Score 5-6
    HIGH = 4  # Score 7-8
    VERY_HIGH = 5  # Score 9-10


@dataclass
class CriticalThinkResult:
    """Result of a critical think analysis.

    Attributes:
        original_claim: The action or claim being analyzed
        confidence_score: Initial confidence score (1-10)
        assumptions: List of identified assumptions
        pitfalls: List of pitfall dictionaries with type, status, and detail
        risks: List of risk dictionaries with risk and mitigation
        revised_confidence: Revised confidence score after analysis (1-10)
        synthesis: Summary of the analysis
        next_steps: Recommended next actions

    Example:
        >>> result = CriticalThinkResult(
        ...     original_claim="Add user authentication",
        ...     confidence_score=7,
        ...     assumptions=["Users have valid emails"],
        ...     pitfalls=[{"type": "security", "status": "fail", "detail": "Missing rate limiting"}],
        ...     risks=[{"risk": "Brute force attacks", "mitigation": "Rate limiting"}],
        ...     revised_confidence=DEFAULT_CONFIDENCE,
        ...     synthesis="Need additional security measures",
        ...     next_steps=["Add rate limiting", "Implement 2FA"]
        ... )
    """

    original_claim: str
    confidence_score: int
    assumptions: List[str]
    pitfalls: List[Dict[str, str]]
    risks: List[Dict[str, str]]
    revised_confidence: int
    synthesis: str
    next_steps: List[str]


def calculate_confidence(score: int) -> ConfidenceLevel:
    """Calculate confidence level from numerical score.

    Args:
        score: Numerical confidence score (1-10)

    Returns:
        ConfidenceLevel enum value corresponding to the score range

    Raises:
        TypeError: If score is not an integer
        ValueError: If score is not between 1 and 10

    Example:
        >>> calculate_confidence(7)
        <ConfidenceLevel.HIGH: 4>
        >>> calculate_confidence(2)
        <ConfidenceLevel.VERY_LOW: 1>

    Note:
        Score boundaries are inclusive at the upper bound:
        - 1-2: VERY_LOW
        - 3-4: LOW
        - 5-6: MEDIUM
        - 7-8: HIGH
        - 9-10: VERY_HIGH
    """
    # Type checking
    if not isinstance(score, int):
        raise TypeError(f"Score must be int, got {type(score).__name__}")
    
    # Validate input range
    if not (MIN_CONFIDENCE <= score <= MAX_CONFIDENCE):
        raise ValueError(
            f"Confidence score must be between {MIN_CONFIDENCE} and {MAX_CONFIDENCE}, got {score}"
        )

    # Use constant boundaries for consistency and maintainability
    if score <= CONFIDENCE_BOUNDARIES["very_low"]:
        return ConfidenceLevel.VERY_LOW
    elif score <= CONFIDENCE_BOUNDARIES["low"]:
        return ConfidenceLevel.LOW
    elif score <= CONFIDENCE_BOUNDARIES["medium"]:
        return ConfidenceLevel.MEDIUM
    elif score <= CONFIDENCE_BOUNDARIES["high"]:
        return ConfidenceLevel.HIGH
    else:
        return ConfidenceLevel.VERY_HIGH


class CriticalThinkEngine:
    """Engine for executing critical think analyses with security hardening."""

    def __init__(self, config: Optional[dict] = None, config_path: Optional[str] = None):
        """Initialize engine with optional configuration.

        Args:
            config: Optional configuration dictionary. If not provided, loads from config file.
            config_path: Optional path to config file. If not provided, uses default location.
        """
        if config is None:
            try:
                self.config = load_config(config_path)
            except (FileNotFoundError, ValueError) as e:
                logger.warning(f"Could not load config: {e}. Using defaults.")
                self.config = self._get_default_config()
        else:
            self.config = config

        # Check if globally enabled
        if not self.config.get("enabled", True):
            logger.info("Critical Think is disabled in configuration")

        self.templates = self._load_templates()

    def _get_default_config(self) -> dict:
        """Get default configuration if config file is unavailable."""
        return {
            "enabled": True,
            "integration_points": {},
            "confidence_thresholds": {
                "critical": 4,
                "warning": 6,
                "acceptable": 7,
                "high": 9,
            },
        }

    def _load_templates(self) -> Dict[str, Template]:
        """Load critical think prompt templates from files with security-safe substitution.

        Returns:
            Dictionary mapping template keys to Template objects for safe string formatting

        Note:
            Uses string.Template instead of str.format() for security.
            Safe substitution prevents injection attacks through template variables.
            Loads templates from files specified in config, with fallback to defaults.
        """
        templates = {}

        # Try to load before_action template from file
        try:
            before_path = get_template_path("before_action", self.config)
            logger.debug(f"Loading before_action template from {before_path}")
            with open(before_path, 'r') as f:
                templates["before_action"] = Template(f.read())
        except (FileNotFoundError, ValueError) as e:
            logger.warning(f"Could not load before_action template from file: {e}. Using default.")
            templates["before_action"] = self._get_default_before_template()

        # Try to load after_action template from file
        try:
            after_path = get_template_path("after_action", self.config)
            logger.debug(f"Loading after_action template from {after_path}")
            with open(after_path, 'r') as f:
                templates["after_action"] = Template(f.read())
        except (FileNotFoundError, ValueError) as e:
            logger.warning(f"Could not load after_action template from file: {e}. Using default.")
            templates["after_action"] = self._get_default_after_template()

        # Load specialized templates based on action_type
        # This enables template routing for different contexts
        for action_type in ["question", "docs", "implementation", "agent_delegation"]:
            try:
                template_path = get_template_path(action_type, self.config)
                logger.debug(f"Loading {action_type} template from {template_path}")
                with open(template_path, 'r') as f:
                    templates[action_type] = Template(f.read())
            except (FileNotFoundError, ValueError):
                # Use before_action as fallback for specialized templates
                logger.debug(f"Could not load {action_type} template, using before_action as fallback")
                templates[action_type] = templates.get("before_action", self._get_default_before_template())

        return templates

    def _get_default_before_template(self) -> Template:
        """Get default before_action template."""
        return Template("""Analyze the following action BEFORE execution:

## Action Description
$action_description

## Context
$context

## 6-Step Critical Analysis

### 1. Core Thesis & Confidence (Initial)
What is the core claim/goal? Rate initial confidence (1-10):
**Confidence Score**: ___

### 2. Foundational Analysis
Identify the top 3 high-impact assumptions:
1. _
2. _
3. _

### 3. Logical Integrity
Map premises and chain of inference. Flag any logical leaps:
_

### 4. AI-Specific Pitfall Analysis
- **Problem Evasion**: Is this solving the stated problem or avoiding the underlying difficulty?
  _
- **"Happy Path" Bias**: Are error handling and failure scenarios considered?
  _
- **Over-Engineering**: Is this proposing unnecessary complexity?
  _
- **Hallucination**: Could this fabricate non-existent functions/APIs?
  _

### 5. Risk & Mitigation
Identify overlooked risks and alternative scenarios:
1. _
2. _

### 6. Synthesis & Revised Recommendation
**Revised Confidence Score**: ___
**Key Flaws Identified**: _
**Actionable Next Step**: _
""")

    def _get_default_after_template(self) -> Template:
        """Get default after_action template."""
        return Template("""Analyze the following action AFTER execution:

## Action Description
$action_description

## Original Plan
$original_plan

## Actual Result
$actual_result

## 6-Step Critical Analysis

### 1. Core Thesis & Confidence (Initial)
What was the expected outcome? Rate post-confidence (1-10):
**Confidence Score**: ___

### 2. Foundational Analysis
Did assumptions hold true?
1. _
2. _
3. _

### 3. Logical Integrity
Did execution match intended logic?
_

### 4. AI-Specific Pitfall Analysis
- **Problem Evasion**: Did the solution address the core problem?
  _
- **"Happy Path" Bias**: Were edge cases handled?
  _
- **Over-Engineering**: Was the implementation appropriately complex?
  _
- **Hallucination**: Were all functions/API calls valid?
  _

### 5. Risk & Mitigation
What risks materialized? What new risks emerged?
1. _
2. _

### 6. Synthesis & Revised Recommendation
**Revised Confidence Score**: ___
**What Worked**: _
**What Needs Fixing**: _
**Recommended Corrections**: _
""")

    def _skip_result(self, action_description: str) -> CriticalThinkResult:
        """
        Return a skip result when Critical Think is disabled.

        Args:
            action_description: The action that was skipped

        Returns:
            CriticalThinkResult with disabled indicator
        """
        return CriticalThinkResult(
            original_claim=action_description,
            confidence_score=DEFAULT_CONFIDENCE,
            assumptions=["Analysis skipped - disabled"],
            pitfalls=[],
            risks=[],
            revised_confidence=DEFAULT_CONFIDENCE,
            synthesis="Analysis skipped - Critical Think is disabled",
            next_steps=["Proceed without critical think analysis"]
        )

    def invoke_before(
        self, action_description: str, context: str, action_type: str = "generic"
    ) -> CriticalThinkResult:
        """
        Invoke critical think before an action.

        Performs pre-action analysis to identify potential issues, assumptions,
        and risks before executing an action.

        Args:
            action_description: Clear description of planned action (1-{MAX_INPUT_LENGTH} chars)
            context: Relevant background, requirements, constraints
            action_type: Type of action for template selection (default: "generic")

        Returns:
            CriticalThinkResult with analysis results including assumptions,
            pitfalls, risks, and recommendations

        Raises:
            TypeError: If parameters are not strings
            ValueError: If parameters exceed maximum length or contain dangerous patterns

        Example:
            >>> engine = CriticalThinkEngine()
            >>> result = engine.invoke_before(
            ...     action_description="Add user authentication",
            ...     context="Using JWT tokens"
            ... )
            >>> print(result.synthesis)
        """.format(MAX_INPUT_LENGTH=MAX_INPUT_LENGTH)
        # Check if globally enabled
        if not is_enabled(self.config):
            logger.debug("Critical Think is globally disabled, skipping analysis")
            return self._skip_result(action_description)

        # Check if integration point is enabled
        integration_point = f"before_{action_type}" if action_type != "generic" else "before_implementation"
        if not is_integration_point_enabled(integration_point, self.config):
            logger.debug(f"Integration point '{integration_point}' is disabled, skipping analysis")
            return self._skip_result(action_description)

        # SECURITY: Validate all inputs
        action_description = validate_string_input(action_description, "action_description")
        context = validate_string_input(context, "context")

        # Log sanitized version
        logger.info(f"Before-action analysis: {sanitize_for_logging(action_description)}")

        # Store context for native integration
        self._current_action_type = action_type
        self._current_context = context

        # SECURITY: Use safe template substitution
        template = self.templates["before_action"]
        prompt = template.safe_substitute(
            action_description=action_description,
            context=context,
        )

        return self._execute_analysis(prompt, action_description)

    def invoke_after(
        self,
        action_description: str,
        original_plan: str,
        actual_result: str,
        action_type: str = "generic",
    ) -> CriticalThinkResult:
        """
        Invoke critical think after an action.

        Performs post-action analysis to evaluate what worked, what didn't,
        and what needs improvement.

        Args:
            action_description: Description of the action that was executed
            original_plan: The original plan or expected outcome
            actual_result: What actually happened during execution
            action_type: Type of action for template selection (default: "generic")

        Returns:
            CriticalThinkResult with post-mortem analysis including
            validation of assumptions and lessons learned

        Raises:
            TypeError: If parameters are not strings
            ValueError: If parameters exceed maximum length or contain dangerous patterns

        Example:
            >>> engine = CriticalThinkEngine()
            >>> result = engine.invoke_after(
            ...     action_description="Add user authentication",
            ...     original_plan="Implement JWT auth",
            ...     actual_result="Auth implemented but missing rate limiting"
            ... )
        """
        # Check if globally enabled
        if not is_enabled(self.config):
            logger.debug("Critical Think is globally disabled, skipping analysis")
            return self._skip_result(action_description)

        # Check if integration point is enabled
        integration_point = f"after_{action_type}" if action_type != "generic" else "after_implementation"
        if not is_integration_point_enabled(integration_point, self.config):
            logger.debug(f"Integration point '{integration_point}' is disabled, skipping analysis")
            return self._skip_result(action_description)

        # SECURITY: Validate all inputs
        action_description = validate_string_input(action_description, "action_description")
        original_plan = validate_string_input(original_plan, "original_plan")
        actual_result = validate_string_input(actual_result, "actual_result")

        # Log sanitized version
        logger.info(f"After-action analysis: {sanitize_for_logging(action_description)}")

        # Store context for native integration
        self._current_action_type = action_type
        self._current_context = f"Plan: {original_plan[:100]}... Result: {actual_result[:100]}..."

        # SECURITY: Use safe template substitution
        template = self.templates["after_action"]
        prompt = template.safe_substitute(
            action_description=action_description,
            original_plan=original_plan,
            actual_result=actual_result,
        )

        return self._execute_analysis(prompt, action_description)

    def _execute_analysis(
        self, prompt: str, original_claim: str
    ) -> CriticalThinkResult:
        """
        Execute the critical think analysis using native Claude Code integration.

        This method performs analysis using Claude Code's native session model
        instead of making separate API calls. This approach:

        - Uses the current session's model and context
        - Eliminates need for anthropic SDK dependency
        - Enables native token tracking via claude-hud
        - Reduces latency (no separate API calls)

        If native integration is disabled or fails, returns a fallback result.

        Args:
            prompt: The formatted prompt for analysis
            original_claim: The original claim being analyzed

        Returns:
            CriticalThinkResult with analysis data (or fallback on error)
        """
        # Import native integration
        try:
            from .native_integration import analyze_native
            NATIVE_AVAILABLE = True
        except ImportError as e:
            logger.warning(f"Native integration module not available: {e}")
            NATIVE_AVAILABLE = False

        # Check if native integration is enabled in config
        native_config = self.config.get("native_integration", {})
        if not native_config.get("enabled", True):
            logger.info("Native integration disabled in config, returning fallback result")
            return self._fallback_result(original_claim, disabled=True)

        # Check if native integration module is available
        if not NATIVE_AVAILABLE:
            logger.warning("Native integration not available, returning fallback result")
            return self._fallback_result(
                original_claim,
                error="Native integration module not found"
            )

        try:
            # Use native integration
            logger.info(
                f"Executing native analysis for: {sanitize_for_logging(original_claim, 50)}"
            )

            # Extract action_type and context from the current invocation context
            # This is passed from invoke_before/invoke_after methods
            action_type = getattr(self, '_current_action_type', 'generic')
            context = getattr(self, '_current_context', '')

            result = analyze_native(
                prompt=prompt,
                original_claim=original_claim,
                action_type=action_type,
                context=context
            )

            logger.info(
                f"Native analysis complete: confidence {result.confidence_score}/10 "
                f"-> {result.revised_confidence}/10"
            )

            return result

        except Exception as e:
            logger.error(f"Native integration error: {e}")
            return self._fallback_result(original_claim, error=str(e))

    def _fallback_result(
        self, original_claim: str, error: Optional[str] = None, disabled: bool = False
    ) -> CriticalThinkResult:
        """Return fallback result when LLM analysis fails or is disabled.

        Args:
            original_claim: The original claim being analyzed
            error: Optional error message
            disabled: True if LLM is disabled (vs. error)

        Returns:
            CriticalThinkResult with fallback values
        """
        if disabled:
            synthesis = "LLM analysis is disabled in configuration"
            next_steps = ["Check LLM configuration to enable analysis", "Manual review recommended"]
        elif error:
            synthesis = f"LLM analysis failed: {error}"
            next_steps = ["Manual review required", "Check LLM configuration and API key"]
        else:
            synthesis = "LLM analysis unavailable"
            next_steps = ["Manual review required"]

        return CriticalThinkResult(
            original_claim=original_claim,
            confidence_score=DEFAULT_CONFIDENCE,
            assumptions=["Analysis unavailable - LLM not configured"],
            pitfalls=[
                {"type": "llm_unavailable", "status": "fail", "detail": error or "LLM disabled"}
            ],
            risks=[{"risk": "Analysis not performed", "mitigation": "Manual review required"}],
            revised_confidence=DEFAULT_CONFIDENCE,
            synthesis=synthesis,
            next_steps=next_steps
        )


_engine: Optional[CriticalThinkEngine] = None
_lock = threading.Lock()


def get_engine() -> CriticalThinkEngine:
    """Get or create the critical think engine singleton (thread-safe)."""
    global _engine
    if _engine is None:
        with _lock:
            if _engine is None:
                _engine = CriticalThinkEngine()
    return _engine


def invoke_before(
    action_description: str, context: str, action_type: str = "generic"
) -> CriticalThinkResult:
    """
    Invoke critical think before an action.

    Args:
        action_description: Description of the action to analyze
        context: Context for the analysis
        action_type: Type of action for template selection

    Returns:
        CriticalThinkResult with analysis results
    """
    return get_engine().invoke_before(action_description, context, action_type)


def invoke_after(
    action_description: str,
    original_plan: str,
    actual_result: str,
    action_type: str = "generic",
) -> CriticalThinkResult:
    """
    Invoke critical think after an action.

    Args:
        action_description: Description of the action that was taken
        original_plan: The original plan
        actual_result: What actually happened
        action_type: Type of action for template selection

    Returns:
        CriticalThinkResult with analysis results
    """
    return get_engine().invoke_after(
        action_description, original_plan, actual_result, action_type
    )


def format_synthesis(result: CriticalThinkResult) -> str:
    """
    Format critical think result as markdown.

    Args:
        result: CriticalThinkResult to format

    Returns:
        Markdown formatted string
    """
    # SECURITY: Sanitize result.original_claim for output
    safe_claim = sanitize_for_logging(result.original_claim, max_length=500)

    return f"""## Critical Think Analysis

### Original Claim
{safe_claim}

### Confidence Scores
- Initial: {result.confidence_score}/10
- Revised: {result.revised_confidence}/10

### Assumptions
{chr(10).join(f"- {a}" for a in result.assumptions)}

### AI-Specific Pitfalls
{chr(10).join(f"- **{p['type']}**: {p['status']} - {p['detail']}" for p in result.pitfalls)}

### Risks & Mitigations
{chr(10).join(f"- **{r['risk']}**: {r['mitigation']}" for r in result.risks)}

### Synthesis
{result.synthesis}

### Next Steps
{chr(10).join(f"- {s}" for s in result.next_steps)}
"""


if __name__ == "__main__":
    # Example usage
    print("Critical Think Engine - Security Hardened Version")
    print("=" * 50)

    # Test input validation
    try:
        engine = get_engine()
        result = engine.invoke_before(
            action_description="Implement user authentication",
            context="Web application needs secure login"
        )
        print("Analysis completed successfully!")
        print(format_synthesis(result))
    except (TypeError, ValueError) as e:
        print(f"Validation error: {e}")
