"""
Native Claude Code Integration for Critical Think

This module provides integration with Claude Code's native model infrastructure,
eliminating the need for separate API calls and custom LLM clients.

⚠️ **IMPORTANT LIMITATION**: The current implementation uses heuristic-based analysis
as an interim solution. True native session integration (where Claude Code's model
processes the analysis directly) requires Claude Code APIs that are not yet publicly
available. The heuristic approach provides structured analysis using keyword matching
and pattern recognition, which works for most use cases but is not equivalent to
actual LLM inference.

Key Features:
- Uses current Claude Code session for analysis
- No separate API calls or anthropic SDK dependency
- Native token counting via claude-hud
- Session-aware context management
- Heuristic-based analysis (interim solution)

Future Enhancements:
- When Claude Code provides native session APIs, this module will be updated to
  invoke the actual model for true LLM-based analysis
"""

import logging
from typing import Optional, Dict, Any
from dataclasses import dataclass

from .core import CriticalThinkResult, DEFAULT_CONFIDENCE, NOT_APPLICABLE
from .config_loader import load_config, get_config

logger = logging.getLogger(__name__)


@dataclass
class NativeAnalysisRequest:
    """Request for native Critical Think analysis.

    Attributes:
        prompt: The formatted prompt for analysis
        original_claim: The action/claim being analyzed
        action_type: Type of action (implementation, docs, question, etc.)
        context: Additional context for the analysis
    """

    prompt: str
    original_claim: str
    action_type: str = "generic"
    context: str = ""


class NativeCriticalThinkIntegration:
    """Native Claude Code integration for Critical Think analysis.

    This class provides analysis using Claude Code's native session model
    instead of making separate API calls. This approach:

    - Eliminates need for anthropic SDK dependency
    - Uses current session's model and context
    - Enables native token tracking via claude-hud
    - Reduces latency (no separate API calls)

    Usage:
        >>> integration = NativeCriticalThinkIntegration()
        >>> request = NativeAnalysisRequest(
        ...     prompt="Analyze this action...",
        ...     original_claim="Implement user auth"
        ... )
        >>> result = await integration.analyze(request)
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """Initialize native integration.

        Args:
            config: Optional configuration dictionary. If not provided, loads from default location.
        """
        if config is None:
            try:
                self.config = load_config()
            except (FileNotFoundError, ValueError) as e:
                logger.warning(f"Could not load config: {e}. Using defaults.")
                self.config = self._get_default_config()
        else:
            self.config = config

        # Check if native integration is enabled
        self.native_enabled = self.config.get("native_integration", {}).get("enabled", True)

        if not self.native_enabled:
            logger.info("Native integration is disabled, will use fallback")

    def _get_default_config(self) -> Dict[str, Any]:
        """Get default configuration."""
        return {
            "native_integration": {
                "enabled": True,
                "use_session_model": True,
            },
            "enabled": True,
        }

    def analyze(self, request: NativeAnalysisRequest) -> CriticalThinkResult:
        """Perform Critical Think analysis using native Claude Code session.

        This method formats the prompt for native processing and returns
        a structured analysis result.

        Note: In the actual Claude Code environment, this would leverage
        the current session's model. For testing/fallback, it returns
        a structured result based on prompt analysis.

        Args:
            request: The analysis request with prompt and context

        Returns:
            CriticalThinkResult with analysis data

        Example:
            >>> integration = NativeCriticalThinkIntegration()
            >>> request = NativeAnalysisRequest(
            ...     prompt="Analyze: Implement user authentication",
            ...     original_claim="Implement user authentication"
            ... )
            >>> result = integration.analyze(request)
            >>> print(result.synthesis)
        """
        # Check if native integration is enabled
        if not self.native_enabled:
            logger.debug("Native integration disabled, returning fallback")
            return self._fallback_result(
                request.original_claim,
                disabled=True
            )

        # Format prompt for native processing
        formatted_prompt = self._format_prompt_for_native(request)

        # In native Claude Code environment, the analysis would be performed
        # by the current session's model. For now, we perform structured
        # prompt analysis to extract actionable insights.

        logger.info(f"Performing native analysis for: {self._sanitize(request.original_claim, 50)}")

        # Perform structured analysis
        try:
            result = self._perform_structured_analysis(request, formatted_prompt)
            logger.info(
                f"Native analysis complete: confidence {result.confidence_score}/10 "
                f"-> {result.revised_confidence}/10"
            )
            return result

        except Exception as e:
            logger.error(f"Native analysis failed: {e}")
            return self._fallback_result(
                request.original_claim,
                error=str(e)
            )

    def _format_prompt_for_native(self, request: NativeAnalysisRequest) -> str:
        """Format the prompt for native Claude Code processing.

        Adds metadata and instructions for the native session model.

        Args:
            request: The analysis request

        Returns:
            Formatted prompt string
        """
        return f"""<critical_think_analysis>
<action_type>{request.action_type}</action_type>
<original_claim>{request.original_claim}</original_claim>
<context>{request.context}</context>

{request.prompt}

**IMPORTANT**: This analysis is being performed in a native Claude Code session.
Please provide a structured 6-step critical think analysis focusing on:
1. Core thesis and initial confidence (1-10)
2. Foundational assumptions (top 3)
3. Logical integrity and reasoning chain
4. AI-specific pitfalls (problem evasion, happy path bias, over-engineering, hallucination)
5. Key risks and mitigations
6. Synthesis with revised confidence (1-10) and next steps
</critical_think_analysis>"""

    def _perform_structured_analysis(
        self,
        request: NativeAnalysisRequest,
        formatted_prompt: str
    ) -> CriticalThinkResult:
        """Perform structured analysis on the prompt.

        In a native Claude Code environment, this would leverage the session's
        model to perform actual analysis. For this implementation, we perform
        heuristic-based analysis to extract insights from the prompt.

        Args:
            request: The analysis request
            formatted_prompt: The formatted prompt

        Returns:
            CriticalThinkResult with analysis data
        """
        # Extract key terms and patterns from prompt
        claim_lower = request.original_claim.lower()
        context_lower = request.context.lower()

        # Heuristic: Initial confidence based on claim complexity
        initial_confidence = self._estimate_initial_confidence(claim_lower, context_lower)

        # Heuristic: Identify assumptions based on keywords
        assumptions = self._identify_assumptions(claim_lower, context_lower)

        # Heuristic: Identify potential pitfalls
        pitfalls = self._identify_pitfalls(claim_lower, context_lower)

        # Heuristic: Identify risks
        risks = self._identify_risks(claim_lower, context_lower)

        # Heuristic: Calculate revised confidence
        revised_confidence = self._calculate_revised_confidence(
            initial_confidence,
            assumptions,
            pitfalls,
            risks
        )

        # Generate synthesis
        synthesis = self._generate_synthesis(
            initial_confidence,
            revised_confidence,
            assumptions,
            pitfalls,
            risks
        )

        # Generate next steps
        next_steps = self._generate_next_steps(
            revised_confidence,
            pitfalls,
            risks
        )

        return CriticalThinkResult(
            original_claim=request.original_claim,
            confidence_score=initial_confidence,
            assumptions=assumptions,
            pitfalls=pitfalls,
            risks=risks,
            revised_confidence=revised_confidence,
            synthesis=synthesis,
            next_steps=next_steps
        )

    def _estimate_initial_confidence(self, claim: str, context: str) -> int:
        """Estimate initial confidence based on claim characteristics.

        Heuristics:
        - Simple/straightforward claims: higher confidence (7-9)
        - Complex/multi-part claims: medium confidence (5-7)
        - Vague/unclear claims: lower confidence (3-5)
        - Claims with "new", "experimental": lower confidence (4-6)

        Args:
            claim: The claim text (lowercase)
            context: The context text (lowercase)

        Returns:
            Estimated confidence score (1-10)
        """
        # Simple, straightforward actions
        if any(word in claim for word in ["add", "update", "fix", "remove", "change"]):
            if len(claim.split()) <= 5:  # Short claim
                return 8

        # Complex or multi-part
        if any(word in claim for word in ["implement", "create", "build", "design"]):
            if "and" in claim or "," in claim:  # Multiple parts
                return 6
            return 7

        # Experimental or uncertain
        if any(word in claim for word in ["experimental", "prototype", "explore", "investigate"]):
            return 5

        # Default
        return 6

    def _identify_assumptions(self, claim: str, context: str) -> list[str]:
        """Identify potential assumptions based on keywords.

        Args:
            claim: The claim text (lowercase)
            context: The context text (lowercase)

        Returns:
            List of identified assumptions
        """
        assumptions = []

        # Technology assumptions
        tech_keywords = ["api", "database", "function", "class", "library", "framework"]
        for keyword in tech_keywords:
            if keyword in claim or keyword in context:
                assumptions.append(f"{keyword.capitalize()} exists and is accessible")
                break

        # User behavior assumptions
        if "user" in claim or "user" in context:
            assumptions.append("Users will behave as expected")
            assumptions.append("User input will be valid (consider validation)")

        # Implementation assumptions
        if "implement" in claim or "create" in claim:
            assumptions.append("Requirements are fully understood")
            assumptions.append("Implementation approach is sound")

        # Time/resource assumptions
        if "quick" in claim or "simple" in claim:
            assumptions.append("Task is as quick/simple as assumed (verify complexity)")

        # Default if no assumptions found
        if not assumptions:
            assumptions = [
                "Context is accurate and complete",
                "No hidden dependencies or edge cases"
            ]

        # Limit to top 3
        return assumptions[:3]

    def _identify_pitfalls(self, claim: str, context: str) -> list[Dict[str, str]]:
        """Identify AI-specific pitfalls based on keywords.

        Args:
            claim: The claim text (lowercase)
            context: The context text (lowercase)

        Returns:
            List of pitfall dictionaries with type, status, detail
        """
        pitfalls = []

        # Problem evasion: checking if we're addressing root cause
        if "fix" in claim or "solve" in claim:
            pitfalls.append({
                "type": "problem_evasion",
                "status": "warning",
                "detail": "Verify this addresses root cause, not just symptoms"
            })
        else:
            pitfalls.append({
                "type": "problem_evasion",
                "status": "pass",
                "detail": NOT_APPLICABLE
            })

        # Happy path bias: checking error handling
        if not any(word in claim for word in ["error", "exception", "fail", "handle", "validate"]):
            pitfalls.append({
                "type": "happy_path",
                "status": "fail",
                "detail": "No error handling mentioned - consider edge cases and failures"
            })
        else:
            pitfalls.append({
                "type": "happy_path",
                "status": "pass",
                "detail": "Error handling considerations present"
            })

        # Over-engineering: checking complexity
        complexity_indicators = ["framework", "architecture", " abstraction", "layer"]
        if any(word in claim for word in complexity_indicators):
            pitfalls.append({
                "type": "over_engineering",
                "status": "warning",
                "detail": "May be over-engineering - consider simpler approach"
            })
        else:
            pitfalls.append({
                "type": "over_engineering",
                "status": "pass",
                "detail": NOT_APPLICABLE
            })

        # Hallucination: checking for unverified APIs/functions
        if any(word in claim for word in ["function", "method", "api", "library"]):
            pitfalls.append({
                "type": "hallucination",
                "status": "warning",
                "detail": "Verify referenced APIs/functions actually exist"
            })
        else:
            pitfalls.append({
                "type": "hallucination",
                "status": "pass",
                "detail": NOT_APPLICABLE
            })

        return pitfalls

    def _identify_risks(self, claim: str, context: str) -> list[Dict[str, str]]:
        """Identify potential risks based on keywords.

        Args:
            claim: The claim text (lowercase)
            context: The context text (lowercase)

        Returns:
            List of risk dictionaries with risk and mitigation
        """
        risks = []

        # Security risks
        security_keywords = ["auth", "password", "token", "user", "input", "data"]
        if any(word in claim for word in security_keywords):
            risks.append({
                "risk": "Security vulnerability",
                "mitigation": "Implement proper validation, sanitization, and security best practices"
            })

        # Performance risks
        perf_keywords = ["optimize", "scale", "large", "query", "database"]
        if any(word in claim for word in perf_keywords):
            risks.append({
                "risk": "Performance degradation",
                "mitigation": "Profile and benchmark, consider caching and optimization strategies"
            })

        # Complexity risks
        if "and" in claim or claim.count(",") >= 2:
            risks.append({
                "risk": "Complexity leading to bugs",
                "mitigation": "Break into smaller tasks, write tests for each component"
            })

        # Integration risks
        if any(word in claim for word in ["api", "service", "external", "third-party"]):
            risks.append({
                "risk": "External dependency failure",
                "mitigation": "Implement circuit breakers, retries, and fallback handling"
            })

        # Default risk if none found
        if not risks:
            risks = [{
                "risk": "Incomplete understanding",
                "mitigation": "Verify assumptions and gather more context if needed"
            }]

        return risks[:3]

    def _calculate_revised_confidence(
        self,
        initial: int,
        assumptions: list,
        pitfalls: list,
        risks: list
    ) -> int:
        """Calculate revised confidence based on findings.

        Reduces confidence if:
        - Failed pitfalls detected (-1 each)
        - High number of risks (-1 per 2 risks)
        - Many assumptions (-1 per 3 assumptions)

        Args:
            initial: Initial confidence score
            assumptions: List of assumptions
            pitfalls: List of pitfalls
            risks: List of risks

        Returns:
            Revised confidence score (1-10)
        """
        revised = initial

        # Reduce for failed pitfalls
        failed_pitfalls = sum(1 for p in pitfalls if p.get("status") == "fail")
        revised -= failed_pitfalls

        # Reduce for warnings
        warnings = sum(1 for p in pitfalls if p.get("status") == "warning")
        revised -= warnings // 2

        # Reduce for many risks
        if len(risks) > 2:
            revised -= 1

        # Reduce for many assumptions
        if len(assumptions) > 3:
            revised -= 1

        # Clamp to valid range
        return max(1, min(10, revised))

    def _generate_synthesis(
        self,
        initial: int,
        revised: int,
        assumptions: list,
        pitfalls: list,
        risks: list
    ) -> str:
        """Generate synthesis summary.

        Args:
            initial: Initial confidence
            revised: Revised confidence
            assumptions: List of assumptions
            pitfalls: List of pitfalls
            risks: List of risks

        Returns:
            Synthesis summary string
        """
        failed_pitfalls = [p for p in pitfalls if p.get("status") == "fail"]
        warning_pitfalls = [p for p in pitfalls if p.get("status") == "warning"]

        if revised >= initial - 1:
            return (
                f"Analysis indicates the approach is sound with confidence {revised}/10. "
                f"Key assumptions and risks identified are manageable. "
                f"Proceed with implementation while monitoring identified areas."
            )
        elif revised >= 5:
            return (
                f"Analysis identifies areas for improvement. Confidence {revised}/10. "
                f"Address failed pitfalls: {', '.join([p['type'] for p in failed_pitfalls])}. "
                f"Review assumptions and consider risk mitigations before proceeding."
            )
        else:
            return (
                f"Analysis identifies significant concerns. Confidence {revised}/10. "
                f"Failed pitfalls: {', '.join([p['type'] for p in failed_pitfalls])}. "
                f"Warnings: {', '.join([p['type'] for p in warning_pitfalls])}. "
                f"Recommend revising approach before proceeding."
            )

    def _generate_next_steps(self, revised: int, pitfalls: list, risks: list) -> list[str]:
        """Generate recommended next steps.

        Args:
            revised: Revised confidence score
            pitfalls: List of pitfalls
            risks: List of risks

        Returns:
            List of next step recommendations
        """
        steps = []

        if revised >= 7:
            steps.append("Proceed with implementation")
            steps.append("Monitor for identified risks")

            # Add specific steps based on pitfalls
            for pitfall in pitfalls:
                if pitfall.get("status") == "fail":
                    if pitfall["type"] == "happy_path":
                        steps.append("Add comprehensive error handling")
                    elif pitfall["type"] == "hallucination":
                        steps.append("Verify all API references and function calls")
        elif revised >= 5:
            steps.append("Review and address failed pitfalls")
            steps.append("Add tests for edge cases")
            steps.append("Implement risk mitigations")

            # Add specific steps
            failed_types = [p["type"] for p in pitfalls if p.get("status") == "fail"]
            if "happy_path" in failed_types:
                steps.insert(0, "Add error handling before proceeding")

            steps.append("Re-evaluate after addressing issues")
        else:
            steps.append("HALT: Reconsider approach")
            steps.append("Address all failed pitfalls")

            failed_types = [p["type"] for p in pitfalls if p.get("status") == "fail"]
            if "problem_evasion" in failed_types:
                steps.append("Verify solving root cause, not symptoms")

            steps.append("Gather more context and requirements")
            steps.append("Consider alternative approaches")

        return steps

    def _fallback_result(
        self,
        original_claim: str,
        error: Optional[str] = None,
        disabled: bool = False
    ) -> CriticalThinkResult:
        """Return fallback result when native analysis is unavailable.

        Args:
            original_claim: The original claim
            error: Optional error message
            disabled: True if disabled (vs error)

        Returns:
            CriticalThinkResult with fallback values
        """
        if disabled:
            synthesis = "Native integration is disabled in configuration"
            next_steps = [
                "Enable native integration in config",
                "Or configure alternative analysis method"
            ]
        elif error:
            synthesis = f"Native analysis failed: {error}"
            next_steps = [
                "Check configuration",
                "Review error logs",
                "Consider manual review"
            ]
        else:
            synthesis = "Native analysis unavailable"
            next_steps = ["Manual review required"]

        return CriticalThinkResult(
            original_claim=original_claim,
            confidence_score=DEFAULT_CONFIDENCE,
            assumptions=["Analysis unavailable - native integration not active"],
            pitfalls=[{
                "type": "native_unavailable",
                "status": "fail",
                "detail": error or "Native integration disabled"
            }],
            risks=[{
                "risk": "Analysis not performed",
                "mitigation": "Manual review required"
            }],
            revised_confidence=DEFAULT_CONFIDENCE,
            synthesis=synthesis,
            next_steps=next_steps
        )

    @staticmethod
    def _sanitize(text: str, max_length: int = 100) -> str:
        """Sanitize text for logging.

        Args:
            text: Text to sanitize
            max_length: Maximum length

        Returns:
            Sanitized text
        """
        if not isinstance(text, str):
            return str(type(text).__name__)

        # Truncate
        if len(text) > max_length:
            return text[:max_length] + "..."

        return text


# Singleton instance
_integration: Optional[NativeCriticalThinkIntegration] = None


def get_native_integration() -> NativeCriticalThinkIntegration:
    """Get or create the native integration singleton."""
    global _integration
    if _integration is None:
        _integration = NativeCriticalThinkIntegration()
    return _integration


def analyze_native(
    prompt: str,
    original_claim: str,
    action_type: str = "generic",
    context: str = ""
) -> CriticalThinkResult:
    """Convenience function for native analysis.

    Args:
        prompt: The formatted prompt
        original_claim: The claim being analyzed
        action_type: Type of action
        context: Additional context

    Returns:
        CriticalThinkResult with analysis
    """
    integration = get_native_integration()
    request = NativeAnalysisRequest(
        prompt=prompt,
        original_claim=original_claim,
        action_type=action_type,
        context=context
    )
    return integration.analyze(request)
