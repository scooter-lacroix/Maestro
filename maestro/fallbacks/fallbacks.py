"""
Fallback System - Alternative execution paths and recovery mechanisms.

Implements fallback strategies for when primary execution paths fail,
providing graceful degradation and alternative solutions.
"""

from abc import ABC, abstractmethod
from typing import Any, Callable, Optional, Dict, List, Union
from enum import Enum
import traceback
import logging
from dataclasses import dataclass
from datetime import datetime


class FallbackLevel(Enum):
    """Enumeration for fallback severity levels."""
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"


@dataclass
class FallbackResult:
    """Result of a fallback operation."""
    success: bool
    value: Any = None
    error: Optional[Exception] = None
    level: FallbackLevel = FallbackLevel.INFO
    message: str = ""
    timestamp: datetime = None

    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = datetime.now()


class FallbackStrategy(ABC):
    """Abstract base class for fallback strategies."""
    
    def __init__(self, name: str, description: str = ""):
        self.name = name
        self.description = description
        self.logger = logging.getLogger(f"fallbacks.{name}")
    
    @abstractmethod
    def execute(self, *args, **kwargs) -> FallbackResult:
        """Execute the fallback strategy."""
        pass
    
    def can_handle(self, error: Exception, context: Dict[str, Any]) -> bool:
        """Determine if this fallback can handle the given error."""
        return True  # Default implementation accepts all errors


class RetryFallback(FallbackStrategy):
    """Fallback strategy that retries the operation with exponential backoff."""
    
    def __init__(self, max_attempts: int = 3, base_delay: float = 1.0, multiplier: float = 2.0):
        super().__init__("retry", "Retry operation with exponential backoff")
        self.max_attempts = max_attempts
        self.base_delay = base_delay
        self.multiplier = multiplier
    
    def execute(self, func: Callable, *args, **kwargs) -> FallbackResult:
        """Execute the function with retry logic."""
        import time
        
        last_error = None
        
        for attempt in range(self.max_attempts):
            try:
                result = func(*args, **kwargs)
                return FallbackResult(success=True, value=result, message=f"Succeeded on attempt {attempt + 1}")
            except Exception as e:
                last_error = e
                if attempt < self.max_attempts - 1:  # Don't sleep on the last attempt
                    delay = self.base_delay * (self.multiplier ** attempt)
                    time.sleep(delay)
                else:
                    break
        
        return FallbackResult(
            success=False, 
            error=last_error, 
            level=FallbackLevel.ERROR,
            message=f"Failed after {self.max_attempts} attempts"
        )


class DefaultResponseFallback(FallbackStrategy):
    """Fallback strategy that returns a default response."""
    
    def __init__(self, default_value: Any, condition_func: Optional[Callable] = None):
        super().__init__("default_response", "Return a default response")
        self.default_value = default_value
        self.condition_func = condition_func
    
    def execute(self, *args, **kwargs) -> FallbackResult:
        """Return the default value."""
        if self.condition_func:
            try:
                if not self.condition_func(*args, **kwargs):
                    return FallbackResult(success=False, message="Condition not met")
            except Exception:
                return FallbackResult(success=False, message="Condition check failed")
        
        return FallbackResult(success=True, value=self.default_value, message="Using default response")


class AlternativeMethodFallback(FallbackStrategy):
    """Fallback strategy that tries an alternative method."""
    
    def __init__(self, alternative_func: Callable, *alt_args, **alt_kwargs):
        super().__init__("alternative_method", "Try an alternative method")
        self.alternative_func = alternative_func
        self.alt_args = alt_args
        self.alt_kwargs = alt_kwargs
    
    def execute(self, *args, **kwargs) -> FallbackResult:
        """Execute the alternative function."""
        try:
            result = self.alternative_func(*self.alt_args, **self.alt_kwargs)
            return FallbackResult(success=True, value=result, message="Alternative method succeeded")
        except Exception as e:
            return FallbackResult(success=False, error=e, level=FallbackLevel.ERROR, 
                                message="Alternative method failed")


class CircuitBreakerFallback(FallbackStrategy):
    """Fallback strategy that implements circuit breaker pattern."""
    
    def __init__(self, failure_threshold: int = 5, timeout: int = 60):
        super().__init__("circuit_breaker", "Circuit breaker pattern")
        self.failure_threshold = failure_threshold
        self.timeout = timeout
        self.failure_count = 0
        self.last_failure_time = None
        self.state = "CLOSED"  # CLOSED, OPEN, HALF_OPEN
    
    def execute(self, func: Callable, *args, **kwargs) -> FallbackResult:
        """Execute with circuit breaker logic."""
        import time
        
        current_time = time.time()
        
        if self.state == "OPEN":
            if current_time - self.last_failure_time >= self.timeout:
                self.state = "HALF_OPEN"
            else:
                return FallbackResult(success=False, message="Circuit breaker is OPEN", 
                                    level=FallbackLevel.WARNING)
        
        if self.state == "HALF_OPEN":
            # Try once in half-open state
            try:
                result = func(*args, **kwargs)
                self.failure_count = 0
                self.state = "CLOSED"
                return FallbackResult(success=True, value=result, message="Recovered from circuit breaker")
            except Exception as e:
                # Failed again, go back to OPEN
                self.last_failure_time = current_time
                self.state = "OPEN"
                return FallbackResult(success=False, error=e, level=FallbackLevel.ERROR,
                                    message="Still failing, keeping circuit breaker OPEN")
        
        # State is CLOSED, execute normally
        try:
            result = func(*args, **kwargs)
            self.failure_count = 0  # Reset on success
            return FallbackResult(success=True, value=result)
        except Exception as e:
            self.failure_count += 1
            self.last_failure_time = current_time
            
            if self.failure_count >= self.failure_threshold:
                self.state = "OPEN"
                return FallbackResult(success=False, error=e, level=FallbackLevel.CRITICAL,
                                    message="Circuit breaker OPENED due to failures")
            
            return FallbackResult(success=False, error=e, level=FallbackLevel.ERROR,
                                message="Operation failed but circuit breaker still CLOSED")


class FallbackChain:
    """Chain of fallback strategies to execute in sequence."""
    
    def __init__(self, name: str = "fallback_chain"):
        self.name = name
        self.strategies: List[FallbackStrategy] = []
        self.logger = logging.getLogger(f"fallbacks.chain.{name}")
    
    def add_strategy(self, strategy: FallbackStrategy) -> 'FallbackChain':
        """Add a fallback strategy to the chain."""
        self.strategies.append(strategy)
        return self
    
    def execute(self, primary_func: Callable, *args,
                context: Optional[Dict[str, Any]] = None, **kwargs) -> FallbackResult:
        """Execute the primary function with fallback strategies."""
        if context is None:
            context = {}

        # Initialize primary_error to ensure it's always defined in case of UnboundLocalError
        primary_error = None
        primary_failed = False

        # Try primary function first
        try:
            result = primary_func(*args, **kwargs)
            return FallbackResult(success=True, value=result, message="Primary function succeeded")
        except Exception as e:
            primary_error = e
            primary_failed = True
            self.logger.warning(f"Primary function failed: {primary_error}")

            # Execute fallback strategies in sequence
            for i, strategy in enumerate(self.strategies):
                try:
                    if strategy.can_handle(primary_error, context):
                        self.logger.info(f"Trying fallback strategy {i+1}: {strategy.name}")

                        # Initialize fallback_result to avoid UnboundLocalError
                        fallback_result = None

                        # For strategies that need the original function, wrap appropriately
                        if isinstance(strategy, RetryFallback):
                            fallback_result = strategy.execute(primary_func, *args, **kwargs)
                        elif isinstance(strategy, AlternativeMethodFallback):
                            fallback_result = strategy.execute()
                        else:
                            # Pass the error and context to the fallback
                            fallback_result = strategy.execute(primary_error, context)

                        if fallback_result and fallback_result.success:
                            self.logger.info(f"Fallback {strategy.name} succeeded")
                            return fallback_result
                        else:
                            error_msg = fallback_result.message if fallback_result else "Fallback returned None"
                            self.logger.warning(f"Fallback {strategy.name} failed: {error_msg}")
                    else:
                        self.logger.info(f"Fallback {strategy.name} cannot handle this error")

                except Exception as fallback_error:
                    self.logger.error(f"Error in fallback {strategy.name}: {fallback_error}")
                    continue

        # All strategies failed
        if primary_failed:
            return FallbackResult(success=False, error=primary_error,
                                level=FallbackLevel.CRITICAL, message="All fallback strategies failed")
        else:
            # This shouldn't happen given the logic, but added for safety
            return FallbackResult(success=False, error=None,
                                level=FallbackLevel.CRITICAL, message="Unexpected execution path")


class FallbackRegistry:
    """Registry for managing fallback strategies."""
    
    def __init__(self):
        self.fallbacks: Dict[str, FallbackStrategy] = {}
        self.chains: Dict[str, FallbackChain] = {}
        self.logger = logging.getLogger("fallbacks.registry")
    
    def register_fallback(self, name: str, strategy: FallbackStrategy) -> None:
        """Register a fallback strategy."""
        self.fallbacks[name] = strategy
        self.logger.info(f"Registered fallback: {name}")
    
    def get_fallback(self, name: str) -> Optional[FallbackStrategy]:
        """Get a registered fallback strategy."""
        return self.fallbacks.get(name)
    
    def register_chain(self, name: str, chain: FallbackChain) -> None:
        """Register a fallback chain."""
        self.chains[name] = chain
        self.logger.info(f"Registered fallback chain: {name}")
    
    def get_chain(self, name: str) -> Optional[FallbackChain]:
        """Get a registered fallback chain."""
        return self.chains.get(name)
    
    def execute_with_fallback(self, name: str, primary_func: Callable, *args, 
                            context: Optional[Dict[str, Any]] = None, **kwargs) -> FallbackResult:
        """Execute a function with a registered fallback chain."""
        chain = self.get_chain(name)
        if chain:
            return chain.execute(primary_func, *args, context=context, **kwargs)
        
        fallback = self.get_fallback(name)
        if fallback:
            try:
                return fallback.execute(primary_func, *args, **kwargs)
            except Exception as e:
                return FallbackResult(success=False, error=e, level=FallbackLevel.ERROR,
                                    message=f"Fallback {name} failed")
        
        return FallbackResult(success=False, message=f"No fallback found with name: {name}")


# Global registry instance
registry = FallbackRegistry()


def register_fallback(name: str, strategy: FallbackStrategy) -> None:
    """Register a fallback strategy globally."""
    registry.register_fallback(name, strategy)


def register_chain(name: str, chain: FallbackChain) -> None:
    """Register a fallback chain globally."""
    registry.register_chain(name, chain)


def execute_with_fallback(name: str, primary_func: Callable, *args, 
                         context: Optional[Dict[str, Any]] = None, **kwargs) -> FallbackResult:
    """Execute a function with a registered fallback."""
    return registry.execute_with_fallback(name, primary_func, *args, context=context, **kwargs)


# Common fallback strategies
def get_retry_fallback(max_attempts: int = 3) -> RetryFallback:
    """Get a standard retry fallback strategy."""
    return RetryFallback(max_attempts=max_attempts)


def get_default_response_fallback(default_value: Any) -> DefaultResponseFallback:
    """Get a default response fallback strategy."""
    return DefaultResponseFallback(default_value)


def get_circuit_breaker_fallback() -> CircuitBreakerFallback:
    """Get a circuit breaker fallback strategy."""
    return CircuitBreakerFallback()