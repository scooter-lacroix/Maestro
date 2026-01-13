"""
Handoffs Module - YAML goal/now/next schema and coordination utilities.

Implements structured handoff mechanisms with YAML schemas for goal/now/next
patterns, supporting cross-terminal coordination and state persistence.
"""

import yaml
from pathlib import Path
from typing import Dict, Any, Optional, List
from dataclasses import dataclass
from enum import Enum
import json
from datetime import datetime


class HandoffStatus(Enum):
    """Enumeration for handoff statuses."""
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class GoalNowNextSchema:
    """
    Data class representing the goal/now/next schema structure.
    
    This follows the CCv3 feature adoption pattern for structured handoffs.
    """
    goal: str  # The ultimate objective
    now: str   # Current focus/action
    next: str  # Immediate next step
    context: Optional[Dict[str, Any]] = None  # Additional context
    metadata: Optional[Dict[str, Any]] = None  # Metadata about the handoff
    timestamp: Optional[str] = None  # Creation timestamp
    status: HandoffStatus = HandoffStatus.PENDING  # Current status


class YAMLGoalNowNextSchema:
    """
    YAML schema handler for goal/now/next structures.
    
    Provides validation, serialization, and deserialization for YAML-based
    goal/now/next schemas used in handoffs.
    """
    
    SCHEMA_VERSION = "1.0"
    
    @staticmethod
    def get_schema_definition() -> Dict[str, Any]:
        """
        Get the definition of the YAML schema for goal/now/next structures.
        
        Returns:
            Dictionary containing the schema definition
        """
        return {
            "type": "object",
            "properties": {
                "version": {"type": "string"},
                "goal": {"type": "string"},
                "now": {"type": "string"},
                "next": {"type": "string"},
                "context": {
                    "type": "object",
                    "additionalProperties": True
                },
                "metadata": {
                    "type": "object",
                    "additionalProperties": True
                },
                "timestamp": {"type": "string"},
                "status": {"type": "string", "enum": ["pending", "in_progress", "completed", "failed"]}
            },
            "required": ["version", "goal", "now", "next"]
        }
    
    @classmethod
    def validate_schema(cls, data: Dict[str, Any]) -> bool:
        """
        Validate data against the goal/now/next schema.
        
        Args:
            data: Dictionary to validate
            
        Returns:
            True if valid, False otherwise
        """
        required_keys = ['goal', 'now', 'next']
        for key in required_keys:
            if key not in data:
                return False
        
        # Validate types
        if not isinstance(data['goal'], str):
            return False
        if not isinstance(data['now'], str):
            return False
        if not isinstance(data['next'], str):
            return False
        
        # Validate optional fields if present
        if 'context' in data and not isinstance(data['context'], dict):
            return False
        if 'metadata' in data and not isinstance(data['metadata'], dict):
            return False
        if 'timestamp' in data and not isinstance(data['timestamp'], str):
            return False
        if 'status' in data and data['status'] not in [status.value for status in HandoffStatus]:
            return False
            
        return True
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Optional[GoalNowNextSchema]:
        """
        Create a GoalNowNextSchema instance from a dictionary.
        
        Args:
            data: Dictionary containing goal/now/next data
            
        Returns:
            GoalNowNextSchema instance or None if invalid
        """
        if not cls.validate_schema(data):
            return None
        
        # Set default timestamp if not provided
        timestamp = data.get('timestamp', datetime.now().isoformat())
        
        # Set default status if not provided
        status_str = data.get('status', 'pending')
        status = HandoffStatus(status_str)
        
        return GoalNowNextSchema(
            goal=data['goal'],
            now=data['now'],
            next=data['next'],
            context=data.get('context'),
            metadata=data.get('metadata'),
            timestamp=timestamp,
            status=status
        )
    
    @classmethod
    def to_dict(cls, schema: GoalNowNextSchema) -> Dict[str, Any]:
        """
        Convert a GoalNowNextSchema instance to a dictionary.
        
        Args:
            schema: GoalNowNextSchema instance to convert
            
        Returns:
            Dictionary representation of the schema
        """
        return {
            'version': cls.SCHEMA_VERSION,
            'goal': schema.goal,
            'now': schema.now,
            'next': schema.next,
            'context': schema.context,
            'metadata': schema.metadata,
            'timestamp': schema.timestamp,
            'status': schema.status.value
        }


class HandoffManager:
    """
    Manager for handling handoffs with YAML goal/now/next schemas.
    
    Coordinates cross-terminal handoffs and manages state persistence.
    """
    
    def __init__(self, storage_path: Optional[str] = None):
        """
        Initialize the handoff manager.
        
        Args:
            storage_path: Optional path for persistent storage of handoffs
        """
        self.storage_path = Path(storage_path) if storage_path else None
        self.active_handoffs: Dict[str, GoalNowNextSchema] = {}
        
        if self.storage_path:
            self.load_from_storage()
    
    def create_handoff(self, goal: str, now: str, next_step: str,
                      context: Optional[Dict[str, Any]] = None,
                      metadata: Optional[Dict[str, Any]] = None) -> str:
        """
        Create a new handoff with goal/now/next structure.

        Args:
            goal: The ultimate objective
            now: Current focus/action
            next_step: Immediate next step
            context: Additional context information
            metadata: Metadata about the handoff

        Returns:
            Unique ID for the handoff
        """
        import uuid
        handoff_id = str(uuid.uuid4())

        schema = GoalNowNextSchema(
            goal=goal,
            now=now,
            next=next_step,
            context=context,
            metadata=metadata,
            timestamp=datetime.now().isoformat()
        )

        self.active_handoffs[handoff_id] = schema

        if self.storage_path:
            self.save_to_storage()

        return handoff_id
    
    def get_handoff(self, handoff_id: str) -> Optional[GoalNowNextSchema]:
        """
        Retrieve a handoff by ID.
        
        Args:
            handoff_id: ID of the handoff to retrieve
            
        Returns:
            GoalNowNextSchema instance or None if not found
        """
        return self.active_handoffs.get(handoff_id)
    
    def update_handoff_status(self, handoff_id: str, status: HandoffStatus) -> bool:
        """
        Update the status of a handoff.
        
        Args:
            handoff_id: ID of the handoff to update
            status: New status for the handoff
            
        Returns:
            True if update successful, False otherwise
        """
        if handoff_id in self.active_handoffs:
            self.active_handoffs[handoff_id].status = status
            
            if self.storage_path:
                self.save_to_storage()
            
            return True
        return False
    
    def update_handoff_now(self, handoff_id: str, now: str) -> bool:
        """
        Update the 'now' field of a handoff.
        
        Args:
            handoff_id: ID of the handoff to update
            now: New 'now' value
            
        Returns:
            True if update successful, False otherwise
        """
        if handoff_id in self.active_handoffs:
            self.active_handoffs[handoff_id].now = now
            self.active_handoffs[handoff_id].timestamp = datetime.now().isoformat()
            
            if self.storage_path:
                self.save_to_storage()
            
            return True
        return False
    
    def update_handoff_next(self, handoff_id: str, next_step: str) -> bool:
        """
        Update the 'next' field of a handoff.
        
        Args:
            handoff_id: ID of the handoff to update
            next_step: New 'next' value
            
        Returns:
            True if update successful, False otherwise
        """
        if handoff_id in self.active_handoffs:
            self.active_handoffs[handoff_id].next = next_step
            self.active_handoffs[handoff_id].timestamp = datetime.now().isoformat()
            
            if self.storage_path:
                self.save_to_storage()
            
            return True
        return False
    
    def complete_handoff(self, handoff_id: str) -> bool:
        """
        Mark a handoff as completed.
        
        Args:
            handoff_id: ID of the handoff to complete
            
        Returns:
            True if completion successful, False otherwise
        """
        return self.update_handoff_status(handoff_id, HandoffStatus.COMPLETED)
    
    def serialize_handoff(self, handoff_id: str) -> Optional[str]:
        """
        Serialize a handoff to YAML format.
        
        Args:
            handoff_id: ID of the handoff to serialize
            
        Returns:
            YAML string representation or None if handoff not found
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            return None
        
        schema_dict = YAMLGoalNowNextSchema.to_dict(handoff)
        return yaml.dump(schema_dict, default_flow_style=False)
    
    def deserialize_handoff(self, yaml_str: str) -> Optional[tuple[str, GoalNowNextSchema]]:
        """
        Deserialize a handoff from YAML format.
        
        Args:
            yaml_str: YAML string to deserialize
            
        Returns:
            Tuple of (handoff_id, GoalNowNextSchema) or None if invalid
        """
        try:
            data = yaml.safe_load(yaml_str)
            schema = YAMLGoalNowNextSchema.from_dict(data)
            if schema:
                import uuid
                handoff_id = str(uuid.uuid4())
                self.active_handoffs[handoff_id] = schema
                return handoff_id, schema
        except Exception:
            pass
        
        return None
    
    def save_to_storage(self):
        """Save active handoffs to persistent storage."""
        if not self.storage_path:
            return
        
        try:
            # Create parent directory if it doesn't exist
            self.storage_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Prepare data for storage
            storage_data = {}
            for handoff_id, schema in self.active_handoffs.items():
                storage_data[handoff_id] = YAMLGoalNowNextSchema.to_dict(schema)
            
            # Write to file
            with open(self.storage_path, 'w', encoding='utf-8') as f:
                yaml.dump(storage_data, f, default_flow_style=False)
        except Exception as e:
            print(f"Error saving handoffs to storage: {e}")
    
    def load_from_storage(self):
        """Load active handoffs from persistent storage."""
        if not self.storage_path or not self.storage_path.exists():
            return
        
        try:
            with open(self.storage_path, 'r', encoding='utf-8') as f:
                storage_data = yaml.safe_load(f)
                
            if storage_data:
                for handoff_id, schema_dict in storage_data.items():
                    schema = YAMLGoalNowNextSchema.from_dict(schema_dict)
                    if schema:
                        self.active_handoffs[handoff_id] = schema
        except Exception as e:
            print(f"Error loading handoffs from storage: {e}")
    
    def get_active_handoffs(self) -> Dict[str, GoalNowNextSchema]:
        """
        Get all active handoffs.
        
        Returns:
            Dictionary of active handoffs
        """
        return self.active_handoffs.copy()
    
    def cleanup_completed_handoffs(self) -> int:
        """
        Remove completed handoffs from memory and storage.
        
        Returns:
            Number of handoffs removed
        """
        completed_ids = [
            hid for hid, handoff in self.active_handoffs.items()
            if handoff.status == HandoffStatus.COMPLETED
        ]
        
        for handoff_id in completed_ids:
            del self.active_handoffs[handoff_id]
        
        if self.storage_path:
            self.save_to_storage()
        
        return len(completed_ids)


# Convenience functions for common handoff operations
def create_simple_handoff(goal: str, now: str, next_step: str) -> str:
    """
    Create a simple handoff with minimal parameters.

    Args:
        goal: The ultimate objective
        now: Current focus/action
        next_step: Immediate next step

    Returns:
        Unique ID for the handoff
    """
    manager = HandoffManager()
    return manager.create_handoff(goal, now, next_step)


def load_handoff_from_yaml(yaml_content: str) -> Optional[tuple[str, GoalNowNextSchema]]:
    """
    Load a handoff directly from YAML content.
    
    Args:
        yaml_content: YAML string containing handoff data
        
    Returns:
        Tuple of (handoff_id, GoalNowNextSchema) or None if invalid
    """
    manager = HandoffManager()
    return manager.deserialize_handoff(yaml_content)