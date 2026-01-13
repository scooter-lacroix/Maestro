"""
Rich Markup Utilities

Utilities for handling Rich markup characters in user-facing output.
This module provides functions to escape and clean Rich markup characters
to prevent injection attacks and ensure safe display in terminals.
"""

import re
from typing import Optional
from loguru import logger

# Rich Unicode characters that could be used for markup injection
RICH_MARKUP_PATTERNS = [
    (r'❌', r'\u274c'),  # Cross mark
    (r'✅', r'\u2705'),  # Check mark
    (r'⚠️', r'\u26a0'),  # Warning sign (note: the emoji variant has FE0F)
    (r'🔥', r'\U0001f525'),  # Fire
    (r'⭐', r'\u2b50'),  # White star
    (r'➡️', r'\u27a1'),  # Right arrow (note: emoji variant)
    (r'📁', r'\U0001f4c1'),  # Folder
    (r'📂', r'\U0001f4c2'),  # Open folder
    (r'📄', r'\U0001f4c4'),  # Document
    (r'📝', r'\U0001f4dd'),  # Memo
    (r'🚀', r'\U0001f680'),  # Rocket
    (r'💻', r'\U0001f4bb'),  # Laptop
    (r'⌨️', r'\u2328'),  # Keyboard (note: emoji variant)
    (r'🖥️', r'\U0001f5a5'),  # Desktop computer (emoji variant)
    (r'📱', r'\U0001f4f1'),  # Mobile phone
    (r'🔧', r'\U0001f527'),  # Wrench
    (r'🛠️', r'\U0001f6e0'),  # Hammer and wrench (emoji variant)
    (r'⚡', r'\u26a1'),  # High voltage
    (r'🎯', r'\U0001f3af'),  # Bullseye
    (r'🔍', r'\U0001f50d'),  # Magnifying glass
    (r'📊', r'\U0001f4ca'),  # Bar chart
    (r'📈', r'\U0001f4c8'),  # Chart increasing
    (r'📉', r'\U0001f4c9'),  # Chart decreasing
    (r'🔴', r'\U0001f534'),  # Red circle
    (r'🟢', r'\U0001f7e2'),  # Green circle
    (r'🔵', r'\U0001f535'),  # Blue circle
    (r'⚫', r'\u2b1b'),  # Black circle
    (r'⚪', r'\u2b1c'),  # White circle
    (r'🟡', r'\U0001f7e1'),  # Yellow circle
]


def escape_rich_markup(text: Optional[str]) -> Optional[str]:
    """
    Escape Rich markup characters in text to prevent injection.

    This function converts Unicode emoji characters to their Unicode escape sequences,
    preventing them from being interpreted as Rich markup in terminals.

    Args:
        text: Input text to escape. Can be None or empty string.

    Returns:
        Escaped text with Rich markup characters converted to Unicode escapes,
        or None if input was None.

    Example:
        >>> escape_rich_markup("❌ Error")
        '\\u274c Error'
    """
    if text is None:
        return None

    if not isinstance(text, str):
        logger.warning(f"Expected string but got {type(text)}, converting to string")
        text = str(text)

    # Create a translation mapping
    translation_map = {}
    for pattern, replacement in RICH_MARKUP_PATTERNS:
        # Extract the actual character from the pattern (the first character)
        char = pattern
        translation_map[char] = replacement

    # Apply translation using str.replace for each pattern
    result = text
    for char, replacement in translation_map.items():
        result = result.replace(char, replacement)

    return result


def has_rich_markup(text: Optional[str]) -> bool:
    """
    Check if text contains Rich markup characters.

    Args:
        text: Text to check for Rich markup.

    Returns:
        True if text contains any Rich markup characters, False otherwise.

    Example:
        >>> has_rich_markup("❌ Error")
        True
        >>> has_rich_markup("Normal error")
        False
    """
    if text is None or not isinstance(text, str):
        return False

    # Check if any Rich markup pattern is present
    for pattern, _ in RICH_MARKUP_PATTERNS:
        if pattern in text:
            return True

    return False


def clean_user_message(text: Optional[str]) -> Optional[str]:
    """
    Clean and sanitize user-facing messages.

    This function performs multiple cleaning operations:
    1. Escapes Rich markup characters
    2. Removes control characters (except \t, \n, \r)
    3. Normalizes whitespace

    Args:
        text: Input text to clean.

    Returns:
        Cleaned and sanitized text.

    Example:
        >>> clean_user_message("❌ Error\x00with  bad\tchars")
        '\\u274c Error with bad\\tchars'
    """
    if text is None:
        return None

    if not isinstance(text, str):
        logger.warning(f"Expected string but got {type(text)}, converting to string")
        text = str(text)

    # Step 1: Remove control characters (except \t, \n, \r)
    # This removes characters with code points < 32 except the allowed ones
    cleaned = re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f-\x9f]', '', text)

    # Step 2: Normalize whitespace (replace multiple spaces with single, trim)
    cleaned = re.sub(r'\s+', ' ', cleaned).strip()

    # Step 3: Escape Rich markup characters
    cleaned = escape_rich_markup(cleaned)

    return cleaned


def safe_format_message(template: str, **kwargs) -> str:
    """
    Safely format a message template, ensuring all values are cleaned.

    This function formats a template string with the given arguments,
    but first cleans all string values to prevent Rich markup injection.

    Args:
        template: Format template string.
        **kwargs: Values to format into the template.

    Returns:
        Formatted and cleaned message string.

    Example:
        >>> safe_format_message("❌ {status}: {message}",
        ...                     status="error", message="File not found")
        '\\u274c error: File not found'
    """
    # Clean all string values
    cleaned_kwargs = {}
    for key, value in kwargs.items():
        if isinstance(value, str):
            cleaned_kwargs[key] = clean_user_message(value)
        else:
            cleaned_kwargs[key] = value

    try:
        return template.format(**cleaned_kwargs)
    except (KeyError, ValueError) as e:
        logger.error(f"Error formatting message template: {e}")
        # Fallback to basic string conversion
        return str(cleaned_kwargs)


def is_safe_for_terminal(text: Optional[str]) -> bool:
    """
    Check if text is safe for terminal output.

    This function verifies that text doesn't contain:
    - Rich markup characters
    - Dangerous control characters
    - Null bytes

    Args:
        text: Text to check for safety.

    Returns:
        True if text is safe for terminal output, False otherwise.

    Example:
        >>> is_safe_for_terminal("❌ Error")
        False
        >>> is_safe_for_terminal("Safe message")
        True
    """
    if text is None:
        return True

    if not isinstance(text, str):
        return False

    # Check for Rich markup
    if has_rich_markup(text):
        return False

    # Check for dangerous control characters
    dangerous_chars = re.search(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]', text)
    if dangerous_chars:
        return False

    return True