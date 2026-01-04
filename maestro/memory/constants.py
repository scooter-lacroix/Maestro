"""
Maestro Memory System Constants

IMPORTANT-4 FIX: Extracted all magic numbers into named constants.
This improves code maintainability and makes the codebase easier to understand.

Constants are organized by category:
- Database: Database-related timeouts and limits
- Memory: Memory operation limits and thresholds
- Rate Limiting: Rate limiting configuration
- Performance: Performance-related thresholds
"""

# =============================================================================
# Database Constants
# =============================================================================

# Database operation timeouts (in seconds)
DATABASE_QUERY_TIMEOUT = 30.0  # Maximum time for a single database query
DATABASE_OPERATION_TIMEOUT = 60.0  # Maximum time for complex database operations
DATABASE_LOCK_RELEASE_DELAY = 0.05  # 50ms delay to ensure WAL files are released (Issue 17)

# Database connection settings
DATABASE_BUSY_TIMEOUT = 10000  # 10 seconds - SQLite busy timeout in milliseconds
DATABASE_POOL_SIZE = 1  # SQLite only supports single writer
DATABASE_MAX_OVERFLOW = 0  # No additional connections for SQLite
DATABASE_POOL_RECYCLE = 3600  # Recycle connections after 1 hour

# Database retry settings
DATABASE_MAX_RETRIES = 5  # Maximum number of retries for database operations
DATABASE_BASE_DELAY = 0.05  # 50ms base delay for exponential backoff

# =============================================================================
# Memory Constants
# =============================================================================

# Memory enhancement limits
DEFAULT_MEMORY_LIMIT = 3  # Default number of memories to retrieve for enhancement
MAX_CONTEXT_LENGTH = 200  # Maximum length of a single memory context in characters
MAX_TOKENS = 4000  # Approximate token limit for enhanced context (Issue 6)

# Memory content validation
MAX_CONTENT_SIZE = 10 * 1024 * 1024  # 10MB maximum content size (IMPORTANT-5)
MAX_LABELS_COUNT = 50  # Maximum number of labels per memory

# Memory batch processing
DEFAULT_BATCH_SIZE = 100  # Process memories in batches to avoid memory exhaustion (Issue 5)

# =============================================================================
# Rate Limiting Constants
# =============================================================================

# Rate limiting configuration
RATE_LIMIT_WINDOW = 1.0  # seconds - Time window for rate limiting (Issue 17)
RATE_LIMIT_MAX_REQUESTS = 10  # Maximum requests per time window (Issue 17)

# =============================================================================
# Performance Constants
# =============================================================================

# Performance thresholds
MEMORY_ENHANCEMENT_TIMEOUT = 5.0  # Maximum time for context enhancement in seconds
SEARCH_TIMEOUT = 10.0  # Maximum time for memory search in seconds

# =============================================================================
# API Constants
# =============================================================================

# API pagination defaults
API_DEFAULT_LIMIT = 50  # Default number of results per page
API_MAX_LIMIT = 200  # Maximum number of results per page

# =============================================================================
# Validation Constants
# =============================================================================

# Path validation
MAX_PROJECT_PATH_LENGTH = 4096  # Maximum length for project paths

# String validation
MAX_STRING_LENGTH = 10000  # Maximum length for generic string fields
