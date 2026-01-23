"""
Maestro Track Repository

Handles file-based track storage and retrieval operations.
"""

import json
import os
from datetime import datetime, UTC
from typing import Optional, Dict, Any, List
from pathlib import Path

from maestro.core.tracks.models import TrackSpec, TrackPlan, TrackMetadata, TrackStatus


class TrackRepository:
    """
    Repository for track file operations

    Manages track directory structure, metadata files,
    and track document storage.
    """

    def __init__(self, tracks_dir: str):
        """
        Initialize track repository

        Args:
            tracks_dir: Path to the tracks directory
        """
        self.tracks_dir = Path(tracks_dir)
        self.tracks_dir.mkdir(parents=True, exist_ok=True)

    def track_exists(self, track_id: str) -> bool:
        """
        Check if track directory exists

        Args:
            track_id: Track identifier

        Returns:
            True if track exists
        """
        track_path = self.tracks_dir / track_id
        return track_path.exists() and track_path.is_dir()

    def create_track(
        self,
        track_id: str,
        track_type: str,
        description: str,
    ) -> str:
        """
        Create a new track directory and metadata

        Args:
            track_id: Track identifier
            track_type: Type of track (feature, bugfix, etc.)
            description: Track description

        Returns:
            Path to created track directory
        """
        track_path = self.tracks_dir / track_id
        track_path.mkdir(parents=True, exist_ok=True)

        now = datetime.now(UTC).isoformat()

        metadata = TrackMetadata(
            track_id=track_id,
            type=track_type,
            status="new",
            description=description,
            created_at=now,
            updated_at=now,
        )

        self.save_metadata(track_id, metadata)

        return str(track_path)

    def get_track_path(self, track_id: str) -> str:
        """
        Get path to track directory

        Args:
            track_id: Track identifier

        Returns:
            Path to track directory

        Raises:
            FileNotFoundError: If track doesn't exist
        """
        track_path = self.tracks_dir / track_id
        if not track_path.exists():
            raise FileNotFoundError(f"Track {track_id} not found")
        return str(track_path)

    def load_metadata(self, track_id: str) -> TrackMetadata:
        """
        Load track metadata from JSON file

        Args:
            track_id: Track identifier

        Returns:
            TrackMetadata instance

        Raises:
            FileNotFoundError: If metadata file doesn't exist
        """
        metadata_path = self.tracks_dir / track_id / "metadata.json"

        if not metadata_path.exists():
            raise FileNotFoundError(f"Metadata for track {track_id} not found")

        with open(metadata_path, "r", encoding="utf-8") as f:
            data = json.load(f)

        return TrackMetadata.from_dict(data)

    def save_metadata(self, track_id: str, metadata: TrackMetadata) -> None:
        """
        Save track metadata to JSON file

        Args:
            track_id: Track identifier
            metadata: TrackMetadata instance
        """
        metadata_path = self.tracks_dir / track_id / "metadata.json"

        with open(metadata_path, "w", encoding="utf-8") as f:
            json.dump(metadata.to_dict(), f, indent=2)

    def update_metadata(
        self,
        track_id: str,
        status: Optional[str] = None,
        maestro_project_id: Optional[int] = None,
        maestro_track_id: Optional[int] = None,
        current_handoff_id: Optional[str] = None,
        tldr_analysis_id: Optional[str] = None,
    ) -> TrackMetadata:
        """
        Update track metadata fields

        Args:
            track_id: Track identifier
            status: New status (optional)
            maestro_project_id: Project ID from memory system (optional)
            maestro_track_id: Track ID from memory system (optional)
            current_handoff_id: Current handoff ID (optional)
            tldr_analysis_id: TLDR analysis ID (optional)

        Returns:
            Updated TrackMetadata instance
        """
        metadata = self.load_metadata(track_id)

        if status is not None:
            metadata.status = status
        if maestro_project_id is not None:
            metadata.maestro_project_id = maestro_project_id
        if maestro_track_id is not None:
            metadata.maestro_track_id = maestro_track_id
        if current_handoff_id is not None:
            metadata.current_handoff_id = current_handoff_id
        if tldr_analysis_id is not None:
            metadata.tldr_analysis_id = tldr_analysis_id

        metadata.updated_at = datetime.now(UTC).isoformat()

        self.save_metadata(track_id, metadata)
        return metadata

    def load_spec(self, track_id: str) -> str:
        """
        Load track specification from file

        Args:
            track_id: Track identifier

        Returns:
            Specification content

        Raises:
            FileNotFoundError: If spec file doesn't exist
        """
        spec_path = self.tracks_dir / track_id / "spec.md"

        if not spec_path.exists():
            raise FileNotFoundError(f"Spec for track {track_id} not found")

        with open(spec_path, "r", encoding="utf-8") as f:
            return f.read()

    def save_spec(self, track_id: str, spec_content: str) -> None:
        """
        Save track specification to file

        Args:
            track_id: Track identifier
            spec_content: Specification markdown content
        """
        spec_path = self.tracks_dir / track_id / "spec.md"

        with open(spec_path, "w", encoding="utf-8") as f:
            f.write(spec_content)

    def load_plan(self, track_id: str) -> str:
        """
        Load track plan from file

        Args:
            track_id: Track identifier

        Returns:
            Plan content

        Raises:
            FileNotFoundError: If plan file doesn't exist
        """
        plan_path = self.tracks_dir / track_id / "plan.md"

        if not plan_path.exists():
            raise FileNotFoundError(f"Plan for track {track_id} not found")

        with open(plan_path, "r", encoding="utf-8") as f:
            return f.read()

    def save_plan(self, track_id: str, plan_content: str) -> None:
        """
        Save track plan to file

        Args:
            track_id: Track identifier
            plan_content: Plan markdown content
        """
        plan_path = self.tracks_dir / track_id / "plan.md"

        with open(plan_path, "w", encoding="utf-8") as f:
            f.write(plan_content)

    def list_tracks(self) -> List[Dict[str, Any]]:
        """
        List all tracks in the repository

        Returns:
            List of track summary dictionaries
        """
        tracks = []

        for track_path in self.tracks_dir.iterdir():
            if not track_path.is_dir():
                continue

            track_id = track_path.name

            try:
                metadata = self.load_metadata(track_id)
                tracks.append({
                    "track_id": track_id,
                    "type": metadata.type,
                    "status": metadata.status,
                    "description": metadata.description,
                    "created_at": metadata.created_at,
                    "updated_at": metadata.updated_at,
                    "maestro_project_id": metadata.maestro_project_id,
                    "maestro_track_id": metadata.maestro_track_id,
                })
            except FileNotFoundError:
                # Track directory exists but no metadata
                tracks.append({
                    "track_id": track_id,
                    "type": "unknown",
                    "status": "unknown",
                    "description": "",
                })

        return sorted(tracks, key=lambda x: x.get("created_at", ""))

    def delete_track(self, track_id: str, db_session: Any = None) -> None:
        """
        Delete a track directory

        Issue #19: Also cleanup database records if db_session provided

        Args:
            track_id: Track identifier
            db_session: Optional database session for cleanup

        Raises:
            FileNotFoundError: If track doesn't exist
        """
        track_path = self.tracks_dir / track_id

        if not track_path.exists():
            raise FileNotFoundError(f"Track {track_id} not found")

        # Issue #19: Cleanup database records if session provided
        if db_session is not None:
            from maestro.memory.database.managers import TrackManager
            track_manager = TrackManager(db_session)
            track_manager.delete_track_by_id(track_id)
            db_session.flush()

        # Remove all files in directory
        for item in track_path.iterdir():
            if item.is_file():
                item.unlink()
            elif item.is_dir():
                # Remove subdirectories recursively
                for sub_item in item.rglob("*"):
                    if sub_item.is_file():
                        sub_item.unlink()
                for sub_dir in sorted(item.rglob("*"), key=lambda x: len(x.parts), reverse=True):
                    if sub_dir.is_dir():
                        sub_dir.rmdir()

        # Remove the directory itself
        track_path.rmdir()

    def move_track(self, track_id: str, destination: str) -> None:
        """
        Move a track directory to another location

        Args:
            track_id: Track identifier
            destination: Destination directory path

        Raises:
            FileNotFoundError: If track doesn't exist
        """
        track_path = self.tracks_dir / track_id

        if not track_path.exists():
            raise FileNotFoundError(f"Track {track_id} not found")

        dest_path = Path(destination)
        dest_path.mkdir(parents=True, exist_ok=True)

        import shutil
        shutil.move(str(track_path), str(dest_path / track_id))
