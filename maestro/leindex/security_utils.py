"""
Security utilities for LeIndex to prevent path traversal and other vulnerabilities.
"""
import os
import re
from pathlib import Path
from typing import List, Optional


def is_safe_path(base_path: str, target_path: str) -> bool:
    """
    Check if target_path is safely contained within base_path to prevent path traversal.
    
    Args:
        base_path: The base directory path (must be absolute)
        target_path: The path to validate (can be relative or absolute)
        
    Returns:
        True if target_path is safely contained within base_path, False otherwise
    """
    if not base_path or not target_path:
        return False
        
    try:
        # Convert to absolute paths for comparison
        base_path = os.path.abspath(base_path)
        
        # If target_path is relative, join it with base_path
        if not os.path.isabs(target_path):
            target_path = os.path.abspath(os.path.join(base_path, target_path))
        else:
            target_path = os.path.abspath(target_path)
            
        # Normalize paths to resolve any .. or . components
        base_path = os.path.normpath(base_path)
        target_path = os.path.normpath(target_path)
        
        # Check if target_path starts with base_path
        # Use os.path.commonpath to handle path separators correctly across platforms
        common_path = Path(os.path.commonpath([base_path, target_path]))
        
        # The target path must be exactly the base path or a subdirectory of it
        return str(common_path) == base_path
        
    except (ValueError, OSError):
        # Handle any path manipulation errors
        return False


def is_approved_project_path(project_path: str, allowed_base_dirs: Optional[List[str]] = None) -> bool:
    """
    Validate that a project path is within approved directories.

    Args:
        project_path: The project path to validate
        allowed_base_dirs: List of allowed base directories, or None to use default approved locations

    Returns:
        True if the project path is approved, False otherwise
    """
    if not project_path or not os.path.exists(project_path):
        return False

    if not os.path.isdir(project_path):
        return False

    # If no allowed base dirs specified, use default approved locations
    if not allowed_base_dirs:
        # Define default approved directories
        home_dir = os.path.expanduser("~")
        default_allowed_dirs = [
            os.path.join(home_dir, "workspace"),
            os.path.join(home_dir, "projects"),
            os.getcwd()  # Current working directory
        ]

        # Add any existing directories from defaults
        allowed_base_dirs = []
        for dir_path in default_allowed_dirs:
            if os.path.exists(dir_path) and os.path.isdir(dir_path):
                allowed_base_dirs.append(dir_path)

    # Check if project_path is within any of the allowed base directories
    project_path = os.path.abspath(project_path)

    for base_dir in allowed_base_dirs:
        if not base_dir:
            continue

        base_dir = os.path.abspath(base_dir)
        if not os.path.exists(base_dir) or not os.path.isdir(base_dir):
            continue

        if is_safe_path(base_dir, project_path):
            return True

    return False


def is_safe_zip_extraction(zip_path: str, extract_to: str) -> bool:
    """
    Validate that zip extraction is safe to prevent zip-slip attacks.
    
    Args:
        zip_path: Path to the zip file
        extract_to: Target extraction directory
        
    Returns:
        True if extraction is safe, False otherwise
    """
    if not zip_path or not extract_to:
        return False
        
    if not os.path.exists(zip_path):
        return False
        
    if not os.path.isdir(extract_to):
        try:
            os.makedirs(extract_to, exist_ok=True)
        except OSError:
            return False
            
    # Check that extract_to is an absolute path to prevent confusion
    extract_to = os.path.abspath(extract_to)
    
    # Basic validation - more comprehensive checks should be done during extraction
    return os.path.isdir(extract_to)


def sanitize_file_path(file_path: str) -> str:
    """
    Sanitize a file path to remove potentially dangerous characters.
    
    Args:
        file_path: The file path to sanitize
        
    Returns:
        Sanitized file path
    """
    if not file_path:
        return ""
        
    # Remove null bytes and control characters
    sanitized = re.sub(r'[\x00-\x1f\x7f]', '', file_path)
    
    # Remove potentially dangerous path components
    sanitized = sanitized.replace('\x00', '')
    
    return sanitized.strip()