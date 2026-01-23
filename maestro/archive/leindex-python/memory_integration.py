"""
LeIndex Memory Integration

Connects LeIndex code analysis with the Maestro memory system
for persistent storage and retrieval of code insights.
"""

import os
from datetime import UTC, datetime
from typing import Any, Dict, List, Optional

from .analyzers.ast import ASTAnalyzer
from .analyzers.callgraph import CallGraphAnalyzer
from .context_extraction import ContextExtractor
from .semantic_index import CodeEntity, SemanticIndex


class LeIndexMemoryBridge:
    """
    Bridge between LeIndex analysis and Maestro memory.

    Stores code analysis results as memories and enables
    semantic search over code insights.
    """

    def __init__(
        self,
        memory_manager=None,
        embeddings_service=None,
    ):
        """
        Initialize the LeIndex-Memory bridge.

        Args:
            memory_manager: Optional MemoryManager instance
            embeddings_service: Optional EmbeddingsService instance
        """
        self.memory_manager = memory_manager
        self.embeddings_service = embeddings_service
        self.context_extractor = ContextExtractor()
        self.semantic_index: Optional[SemanticIndex] = None

    def store_file_analysis(
        self,
        file_path: str,
        session_id: Optional[str] = None,
    ) -> Optional[int]:
        """
        Store file analysis result as a memory.

        Args:
            file_path: Path to the file
            session_id: Optional session ID

        Returns:
            Memory ID or None
        """
        if not self.memory_manager:
            return None

        result = self.context_extractor.extract_for_file(file_path)
        if not result:
            return None

        return self._store_context(result.context, session_id)

    def store_code_entity(
        self,
        entity: CodeEntity,
        session_id: Optional[str] = None,
    ) -> Optional[int]:
        """
        Store a code entity as a memory.

        Args:
            entity: CodeEntity to store
            session_id: Optional session ID

        Returns:
            Memory ID or None
        """
        if not self.memory_manager:
            return None

        content = f"""# {entity.type.capitalize()}: {entity.name}

File: {entity.file}
Line: {entity.line}

Signature:
{entity.signature}
"""

        if entity.docstring:
            content += f"\nDocumentation:\n{entity.docstring}"

        # Create memory
        try:
            memory = self.memory_manager.create_memory(
                content=content,
                summary=f"{entity.type}: {entity.name}",
                category="pattern",
                importance="normal",
                source="maestro:leindex",
                session_id=session_id,
            )
        except Exception:
            return None

        return int(memory.id) if memory else None

    def search_code_insights(
        self,
        query: str,
        limit: int = 10,
        session_id: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """
        Search for code insights using semantic search.

        Args:
            query: Search query
            limit: Maximum results
            session_id: Optional session ID

        Returns:
            List of matching memories
        """
        if not self.memory_manager or not self.embeddings_service:
            return []

        # Get a database session
        try:
            from maestro.memory.database.models import get_session
            db = get_session()
        except Exception:
            return []

        try:
            results = self.embeddings_service.semantic_search_memories(
                query=query,
                session=db,
                limit=limit,
                category="pattern",
            )
            return results
        finally:
            db.close()

    def get_context_for_file(
        self,
        file_path: str,
        session_id: Optional[str] = None,
    ) -> List:
        """
        Get stored memories for a file.

        Args:
            file_path: Path to the file
            session_id: Optional session ID

        Returns:
            List of memories related to the file
        """
        if not self.memory_manager:
            return []

        # Search for memories about this file
        try:
            results = self.memory_manager.search_memories(
                query=file_path,
                category="pattern",
                limit=10,
            )
            return results
        except Exception:
            return []

    def index_project_to_memory(
        self,
        project_path: str,
        session_id: Optional[str] = None,
    ) -> int:
        """
        Index project and store analysis results in memory.

        Args:
            project_path: Path to the project
            session_id: Optional session ID

        Returns:
            Number of memories created
        """
        count = 0

        # Get semantic index
        if self.semantic_index is None:
            self.semantic_index = SemanticIndex()

        self.semantic_index.index_project(project_path)

        # Store each entity
        for entity in self.semantic_index.entities.values():
            memory_id = self.store_code_entity(entity, session_id)
            if memory_id:
                count += 1

        return count

    def recall_code_context(
        self,
        query: str,
        project_path: Optional[str] = None,
        limit: int = 5,
    ) -> str:
        """
        Recall relevant code context for a query.

        Combines semantic search with LeIndex analysis to provide
        comprehensive context for LLM interactions.

        Args:
            query: User's query
            project_path: Optional project path
            limit: Maximum results

        Returns:
            Formatted context string
        """
        contexts = []

        # First, search memory for relevant code insights
        memories = self.search_code_insights(query, limit=limit)
        for memory in memories:
            contexts.append(f"## Memory: {memory.get('summary', '')}")
            content = memory.get('content', '')
            contexts.append(content[:500] if content else '')

        # Second, use semantic search if project provided
        if project_path:
            if self.semantic_index is None:
                self.semantic_index = SemanticIndex()
                self.semantic_index.load()

            results = self.semantic_index.search(query, limit=limit)
            for entity, score in results:
                contexts.append(f"## Code: {entity.name} ({entity.type})")
                contexts.append(f"Location: {entity.file}:{entity.line}")
                contexts.append(entity.signature)

        return "\n\n".join(contexts) if contexts else "# No relevant code context found"

    def _store_context(
        self,
        context,
        session_id: Optional[str] = None,
    ) -> Optional[int]:
        """Store a context object as memory"""
        content = context.to_llm_string()

        try:
            memory = self.memory_manager.create_memory(
                content=content,
                summary=f"LeIndex Analysis: {context.entry_point}",
                category="pattern",
                importance="normal",
                source="maestro:leindex",
                session_id=session_id,
            )
        except Exception:
            return None

        if not memory:
            return None

        memory_id: int = memory.id

        # Index with embeddings if available
        if self.embeddings_service:
            try:
                self.embeddings_service.index_memory(memory_id, content)
            except Exception:
                pass

        return memory_id


def get_leindex_memory_bridge(
    db_path: Optional[str] = None,
) -> LeIndexMemoryBridge:
    """
    Get a LeIndex-Memory bridge instance.

    Args:
        db_path: Optional database path

    Returns:
        LeIndexMemoryBridge instance
    """
    # Initialize memory manager if needed
    memory_manager = None
    try:
        from maestro.memory.database.managers import MemoryManager
        from maestro.memory.database.models import get_session, create_tables

        create_tables(db_path=db_path)
        session = get_session(db_path=db_path)
        memory_manager = MemoryManager(session)
    except Exception:
        pass

    # Initialize embeddings service if needed
    embeddings_service = None
    try:
        from maestro.memory.embeddings.service import get_embeddings_service
        embeddings_service = get_embeddings_service(db_path=db_path)
    except Exception:
        pass

    return LeIndexMemoryBridge(
        memory_manager=memory_manager,
        embeddings_service=embeddings_service,
    )
