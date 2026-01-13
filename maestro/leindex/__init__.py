"""
LeIndex: Maestro's unified search and code analysis system.

This module integrates TLDR's 5-layer analysis capabilities with LeIndexer's
search infrastructure to provide code understanding and retrieval.
"""

from typing import List, Optional, Dict, Any, Union
from abc import ABC, abstractmethod

# Version of the LeIndex module
__version__ = "0.1.0"

class LeIndexError(Exception):
    """Base exception for LeIndex errors."""
    pass

class AnalysisError(LeIndexError):
    """Error during code analysis."""
    pass

class SearchError(LeIndexError):
    """Error during search operations."""
    pass

class IndexingError(LeIndexError):
    """Error during content indexing."""
    pass
