"""
Maestro Memory Exceptions

Issue 16: Custom exception hierarchy for Maestro-specific errors.

This module provides a structured exception hierarchy for the Maestro
memory system, enabling better error handling and debugging.
"""


class MaestroMemoryError(Exception):
    """Base exception for all Maestro memory errors"""

    def __init__(self, message: str, details: dict = None):
        """
        Initialize Maestro memory error.

        Args:
            message: Error message
            details: Additional error details (optional)
        """
        super().__init__(message)
        self.message = message
        self.details = details or {}

    def __str__(self):
        if self.details:
            return f"{self.message} - Details: {self.details}"
        return self.message

    def to_dict(self) -> dict:
        """Convert exception to dictionary for API responses"""
        return {
            "error_type": self.__class__.__name__,
            "message": self.message,
            "details": self.details
        }


class MaestroValidationError(MaestroMemoryError):
    """Raised when input validation fails"""

    pass


class MaestroPathTraversalError(MaestroValidationError):
    """Raised when path traversal is detected"""

    def __init__(self, path: str, reason: str = ""):
        super().__init__(
            f"Path traversal detected: {path}",
            {"path": path, "reason": reason}
        )


class MaestroDatabaseError(MaestroMemoryError):
    """Raised when database operations fail"""

    pass


class MaestroInitializationError(MaestroDatabaseError):
    """Raised when service initialization fails"""

    pass


class MaestroQueryError(MaestroDatabaseError):
    """Raised when database query fails"""

    pass


class MaestroTransactionError(MaestroDatabaseError):
    """Raised when database transaction fails"""

    pass


class MaestroStorageError(MaestroMemoryError):
    """Raised when memory storage fails"""

    pass


class MaestroRetrievalError(MaestroMemoryError):
    """Raised when memory retrieval fails"""

    pass


class MaestroSearchError(MaestroMemoryError):
    """Raised when memory search fails"""

    pass


class MaestroConfigurationError(MaestroMemoryError):
    """Raised when configuration is invalid"""

    pass


class MaestroAuthenticationError(MaestroMemoryError):
    """Raised when authentication fails"""

    pass


class MaestroAuthorizationError(MaestroMemoryError):
    """Raised when authorization fails"""

    pass


class MaestroRateLimitError(MaestroMemoryError):
    """Raised when rate limit is exceeded"""

    def __init__(self, limit: int, window: int):
        super().__init__(
            f"Rate limit exceeded: {limit} requests per {window} seconds",
            {"limit": limit, "window": window}
        )


class MaestroServiceUnavailableError(MaestroMemoryError):
    """Raised when service is unavailable"""

    pass


__all__ = [
    "MaestroMemoryError",
    "MaestroValidationError",
    "MaestroPathTraversalError",
    "MaestroDatabaseError",
    "MaestroInitializationError",
    "MaestroQueryError",
    "MaestroTransactionError",
    "MaestroStorageError",
    "MaestroRetrievalError",
    "MaestroSearchError",
    "MaestroConfigurationError",
    "MaestroAuthenticationError",
    "MaestroAuthorizationError",
    "MaestroRateLimitError",
    "MaestroServiceUnavailableError",
]
