"""
Coordination handlers for multi-agent collaboration

This package provides handlers for:
- File claims: Prevent concurrent file modifications
- Handoffs: Session continuity across agents
- Continuity ledgers: Track session progress
"""

from maestro.memory.coordination.file_claims import (
    FileClaimsHandler,
    FileClaimsManager,
    ClaimConflictError,
    ClaimExpiredError,
)
from maestro.memory.coordination.handoffs import (
    HandoffHandler,
    HandoffTemplate,
    HandoffNotFoundError,
    HandoffNotPickableError,
)
from maestro.memory.coordination.ledgers import (
    ContinuityLedgerHandler,
    LedgerBuilder,
    EntryType,
)
from maestro.memory.coordination.file_locks import AdvisoryLock
from maestro.memory.coordination.session_registry import SessionRegistry

__all__ = [
    # File claims
    "FileClaimsHandler",
    "FileClaimsManager",
    "ClaimConflictError",
    "ClaimExpiredError",
    # Handoffs
    "HandoffHandler",
    "HandoffTemplate",
    "HandoffNotFoundError",
    "HandoffNotPickableError",
    # Ledgers
    "ContinuityLedgerHandler",
    "LedgerBuilder",
    "EntryType",
    # Coordination locks and registries
    "AdvisoryLock",
    "SessionRegistry",
]
