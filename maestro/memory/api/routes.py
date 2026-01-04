"""
Maestro Memory API Routes

FastAPI endpoints for memory operations.

CRITICAL-3 FIX: Removed stub routes that were never implemented.
The dashboard.py file provides the complete API implementation.
This file is kept for backwards compatibility but routes are not registered.

Use the dashboard routes instead:
- POST /api/v1/store - Store memory via dashboard
- GET /api/v1/context/project - Retrieve project context
- GET /api/v1/context/track - Retrieve track context
- GET /api/v1/search - Search similar commands
"""

from typing import Optional, List, Dict, Any
from fastapi import APIRouter, HTTPException

# Empty router - all routes are implemented in dashboard.py
# This file is kept for backwards compatibility only
router = APIRouter(prefix="/api/v1/maestro/memory", tags=["memory"])

# No routes registered - use dashboard.py instead
# The dashboard provides a complete REST API at /api/v1/* endpoints
