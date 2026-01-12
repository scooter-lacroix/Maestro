"""
TLDR Memory Integration

Connects TLDR code analysis with the Maestro memory system
for persistent storage and retrieval of code insights.
"""

import os
from typing import Optional, List, Dict, Any
from datetime import datetime, UTC

from maestro.tldr.analyzer import TLRDAnalyzer, AnalysisResult
from maestro.tldr.semantic import SemanticIndex, CodeEntity
from maestro.memory.database.managers import MemoryManager
from maestro.memory.database.models import Memory
from maestro.memory.embeddings.service import EmbeddingsService


class TLDRMemoryBridge:
    """
    Bridge between TLDR analysis and Maestro memory

    Stores code analysis results as memories and enables
    semantic search over code insights.
    """

    def __init__(
        self,
        memory_manager: Optional[MemoryManager] = None,
        embeddings_service: Optional[EmbeddingsService] = None,
    ):
        """
        Initialize the TLDR-Memory bridge

        Args:
            memory_manager: Optional MemoryManager instance
            embeddings_service: Optional EmbeddingsService instance
        """
        self.memory_manager = memory_manager
        self.embeddings_service = embeddings_service
        self.analyzer = TLRDAnalyzer()

    def store_analysis(
        self,
        result: AnalysisResult,
        session_id: Optional[str] = None,
    ) -> Optional[int]:
        """
        Store analysis result as a memory

        Args:
            result: AnalysisResult to store
            session_id: Optional session ID

        Returns:
            Memory ID or None
        """
        if not self.memory_manager:
            return None

        # Convert analysis to memory content
        content = self._analysis_to_memory_content(result)

        # Create memory
        memory = self.memory_manager.create_memory(
            content=content,
            summary=f"TLDR Analysis: {result.context.file_path or result.context.project_path}",
            category="pattern",
            importance="normal",
            source="maestro:tldr",
            session_id=session_id,
            command_context=result.context.to_dict(),
        )

        if not memory:
            return None

        memory_id: int = memory.id  # type: ignore[assignment]

        # Index with embeddings if available
        if self.embeddings_service:
            self.embeddings_service.index_memory(memory_id, content)

        return memory_id

    def _analysis_to_memory_content(self, result: AnalysisResult) -> str:
        """Convert analysis result to memory content"""
        lines = []

        if result.ast_analysis:
            lines.append("# Code Structure")
            lines.append(f"File: {result.ast_analysis.path}")
            lines.append(f"Language: {result.ast_analysis.language}")
            lines.append(f"Lines: {result.ast_analysis.line_count}")

            if result.ast_analysis.imports:
                lines.append("\n## Imports")
                for imp in result.ast_analysis.imports[:10]:
                    if imp.name:
                        lines.append(f"from {imp.module} import {imp.name}")
                    else:
                        lines.append(f"import {imp.module}")

            if result.ast_analysis.classes:
                lines.append("\n## Classes")
                for name, cls in sorted(result.ast_analysis.classes.items())[:5]:
                    bases = f"({', '.join(cls.bases)})" if cls.bases else ""
                    lines.append(f"class {name}{bases}")

            if result.ast_analysis.functions:
                lines.append("\n## Functions")
                for name, func in sorted(result.ast_analysis.functions.items())[:10]:
                    args = ", ".join(func.args[:5])
                    lines.append(f"def {name}({args})")

        if result.cfg:
            lines.append("\n# Control Flow")
            lines.append(f"Complexity: {result.cfg.metrics.cyclomatic_complexity}")

        if result.call_graph:
            lines.append(f"\n# Call Graph")
            lines.append(f"Functions: {len(result.call_graph.functions)}")
            lines.append(f"Edges: {len(result.call_graph.edges)}")

        return "\n".join(lines)

    def store_code_entity(
        self,
        entity: CodeEntity,
        session_id: Optional[str] = None,
    ) -> Optional[int]:
        """
        Store a code entity as a memory

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
        memory = self.memory_manager.create_memory(
            content=content,
            summary=f"{entity.type}: {entity.name}",
            category="pattern",
            importance="normal",
            source="maestro:tldr",
            session_id=session_id,
        )

        return int(memory.id) if memory else None

    def search_code_insights(
        self,
        query: str,
        limit: int = 10,
        session_id: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """
        Search for code insights using semantic search

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
        from maestro.memory.database.models import get_session
        db = get_session()

        results = self.embeddings_service.semantic_search_memories(
            query=query,
            session=db,
            limit=limit,
            category="pattern",
        )

        db.close()

        return results

    def get_context_for_file(
        self,
        file_path: str,
        session_id: Optional[str] = None,
    ) -> List[Memory]:
        """
        Get stored memories for a file

        Args:
            file_path: Path to the file
            session_id: Optional session ID

        Returns:
            List of memories related to the file
        """
        if not self.memory_manager:
            return []

        # Search for memories about this file
        results = self.memory_manager.search_memories(
            query=file_path,
            category="pattern",
            limit=10,
        )

        return results

    def index_project_to_memory(
        self,
        project_path: str,
        session_id: Optional[str] = None,
    ) -> int:
        """
        Index project and store analysis results in memory

        Args:
            project_path: Path to the project
            session_id: Optional session ID

        Returns:
            Number of memories created
        """
        count = 0

        # Get semantic index
        semantic_index = self.analyzer.get_semantic_index()
        semantic_index.index_project(project_path)

        # Store each entity
        for entity in semantic_index.entities.values():
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
        Recall relevant code context for a query

        Combines semantic search with TLDR analysis to provide
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
            contexts.append(f"## Memory: {memory['summary']}")
            contexts.append(memory['content'][:500])

        # Second, use TLDR semantic search if project provided
        if project_path:
            results = self.analyzer.semantic_search(query, project_path, limit=limit)
            for entity, score in results:
                contexts.append(f"## Code: {entity.name} ({entity.type})")
                contexts.append(f"Location: {entity.file}:{entity.line}")
                contexts.append(entity.signature)

        return "\n\n".join(contexts) if contexts else "# No relevant code context found"


def get_tldr_memory_bridge(
    db_path: Optional[str] = None,
) -> TLDRMemoryBridge:
    """
    Get a TLDR-Memory bridge instance

    Args:
        db_path: Optional database path

    Returns:
        TLDRMemoryBridge instance
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

    return TLDRMemoryBridge(
        memory_manager=memory_manager,
        embeddings_service=embeddings_service,
    )
