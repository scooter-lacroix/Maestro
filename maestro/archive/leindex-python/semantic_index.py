"""
Semantic Index for LeIndex

Provides natural language search over code using embeddings.
This module brings TLDR's semantic search capabilities to LeIndex.
"""

import hashlib
import json
import os
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from .analyzers.ast import ASTAnalyzer


@dataclass
class CodeEntity:
    """A code entity (function, class, method) that can be searched"""
    id: str
    name: str
    type: str  # function, method, class, module
    file: str
    line: int
    end_line: int
    signature: str
    docstring: Optional[str] = None
    class_name: Optional[str] = None
    module: str = ""

    def to_search_text(self) -> str:
        """Generate text for embedding"""
        parts = [self.signature]
        if self.docstring:
            parts.append(self.docstring)
        if self.class_name:
            parts.append(f"Class: {self.class_name}")
        return " ".join(parts)

    def to_dict(self) -> Dict:
        """Convert to dictionary for serialization"""
        return {
            "id": self.id,
            "name": self.name,
            "type": self.type,
            "file": self.file,
            "line": self.line,
            "end_line": self.end_line,
            "signature": self.signature,
            "docstring": self.docstring,
            "class_name": self.class_name,
            "module": self.module,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "CodeEntity":
        """Create from dictionary"""
        return cls(**data)


@dataclass
class IndexStats:
    """Statistics about the semantic index"""
    total_entities: int = 0
    files_indexed: int = 0
    last_updated: Optional[datetime] = None
    index_path: str = ""


class SemanticIndex:
    """
    Semantic Index for natural language code search.

    Uses embeddings to enable natural language queries like
    "find functions that handle authentication" or "search for
    database connection code".

    This is integrated with LeIndex for unified code intelligence.
    """

    def __init__(
        self,
        index_path: Optional[str] = None,
        model_name: str = "sentence-transformers/all-MiniLM-L6-v2",
    ):
        """
        Initialize the semantic index.

        Args:
            index_path: Path to store index data
            model_name: Name of the embedding model
        """
        if index_path is None:
            index_path = os.path.expanduser("~/.maestro/leindex/semantic")

        self.index_path = index_path
        self.model_name = model_name
        self.entities: Dict[str, CodeEntity] = {}
        self.embeddings: List[List[float]] = []
        self._model = None
        self._dirty = False

        # Create index directory
        os.makedirs(self.index_path, exist_ok=True)

    @property
    def index_file(self) -> str:
        """Path to the index file"""
        return os.path.join(self.index_path, "semantic_index.json")

    @property
    def embeddings_file(self) -> str:
        """Path to the embeddings file"""
        return os.path.join(self.index_path, "embeddings.npy")

    def load(self) -> bool:
        """
        Load the index from disk.

        Returns:
            True if loaded successfully
        """
        try:
            with open(self.index_file, "r", encoding="utf-8") as f:
                data = json.load(f)

            self.entities = {
                k: CodeEntity.from_dict(v) if isinstance(v, dict) else v
                for k, v in data.get("entities", {}).items()
            }

            # Load embeddings if available
            if os.path.exists(self.embeddings_file):
                import numpy as np
                self.embeddings = np.load(self.embeddings_file, allow_pickle=True).tolist()

            self._dirty = False
            return True
        except Exception:
            return False

    def save(self) -> bool:
        """
        Save the index to disk.

        Returns:
            True if saved successfully
        """
        try:
            os.makedirs(self.index_path, exist_ok=True)

            # Save entities
            data = {
                "entities": {
                    k: v.to_dict() if isinstance(v, CodeEntity) else v
                    for k, v in self.entities.items()
                },
                "stats": {
                    "total_entities": len(self.entities),
                    "last_updated": datetime.now().isoformat(),
                }
            }

            with open(self.index_file, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2)

            # Save embeddings if available
            if self.embeddings:
                import numpy as np
                np.save(self.embeddings_file, self.embeddings)

            self._dirty = False
            return True
        except Exception:
            return False

    def index_single_file(self, path: str, force: bool = False) -> int:
        """
        Index a single file.

        Args:
            path: Path to the file
            force: Re-index even if unchanged

        Returns:
            Number of entities indexed
        """
        analyzer = ASTAnalyzer()

        # Read file
        try:
            with open(path, "r", encoding="utf-8") as f:
                source = f.read()
        except Exception:
            return 0

        analysis = analyzer.analyze(source, path)

        if "error" in analysis:
            return 0

        count = 0
        module = os.path.splitext(os.path.basename(path))[0]

        # Index functions
        for func_name, func_info in analysis.get("functions", {}).items():
            entity_id = f"{path}:{func_name}"
            signature = self._make_function_signature(func_info)

            entity = CodeEntity(
                id=entity_id,
                name=func_name,
                type="function",
                file=path,
                line=func_info.get("line", 0),
                end_line=func_info.get("end_line", 0),
                signature=signature,
                docstring=func_info.get("docstring"),
                module=module,
            )

            self.entities[entity_id] = entity
            count += 1

        # Index classes and methods
        for cls_name, cls_info in analysis.get("classes", {}).items():
            cls_id = f"{path}:{cls_name}"

            # Index the class itself
            cls_entity = CodeEntity(
                id=cls_id,
                name=cls_name,
                type="class",
                file=path,
                line=cls_info.get("line", 0),
                end_line=cls_info.get("end_line", 0),
                signature=f"class {cls_name}",
                docstring=cls_info.get("docstring"),
                module=module,
            )
            self.entities[cls_id] = cls_entity
            count += 1

            # Index methods
            for method_name, method_info in cls_info.get("methods", {}).items():
                method_id = f"{path}:{cls_name}.{method_name}"
                signature = self._make_function_signature(method_info, cls_name)

                entity = CodeEntity(
                    id=method_id,
                    name=method_name,
                    type="method",
                    file=path,
                    line=method_info.get("line", 0),
                    end_line=method_info.get("end_line", 0),
                    signature=signature,
                    docstring=method_info.get("docstring"),
                    class_name=cls_name,
                    module=module,
                )

                self.entities[method_id] = entity
                count += 1

        if count > 0:
            self._dirty = True

        return count

    def index_project(
        self,
        root_path: str,
        pattern: str = "*.py",
        exclude_dirs: Optional[List[str]] = None,
    ) -> int:
        """
        Index an entire project.

        Args:
            root_path: Root directory of the project
            pattern: File pattern to include
            exclude_dirs: Directories to exclude

        Returns:
            Number of entities indexed
        """
        if exclude_dirs is None:
            exclude_dirs = ["__pycache__", ".venv", "venv", "node_modules", ".git", "dist", "build"]

        root_path = os.path.abspath(root_path)
        count = 0

        for py_file in Path(root_path).rglob(pattern):
            file_path = str(py_file)

            # Skip excluded directories
            if any(excl in file_path for excl in exclude_dirs):
                continue

            count += self.index_single_file(file_path)

        # Generate embeddings for new entities
        if self._dirty:
            self._generate_embeddings()

        return count

    def search(
        self,
        query: str,
        limit: int = 10,
        threshold: float = 0.3,
    ) -> List[Tuple[CodeEntity, float]]:
        """
        Search for code using natural language.

        Args:
            query: Natural language query
            limit: Maximum results
            threshold: Minimum similarity threshold

        Returns:
            List of (entity, score) tuples
        """
        if not self.entities:
            return []

        # Generate query embedding
        query_emb = self._encode_text(query)
        if query_emb is None:
            # Fallback to text matching
            return self._text_search(query, limit)

        # Calculate similarities
        results = []
        for entity_id, entity in self.entities.items():
            entity_idx = list(self.entities.keys()).index(entity_id)
            if entity_idx < len(self.embeddings):
                entity_emb = self.embeddings[entity_idx]
                score = self._cosine_similarity(query_emb, entity_emb)
                if score >= threshold:
                    results.append((entity, score))

        # Sort by score
        results.sort(key=lambda x: x[1], reverse=True)

        return results[:limit]

    def _text_search(self, query: str, limit: int) -> List[Tuple[CodeEntity, float]]:
        """Fallback text-based search"""
        query_lower = query.lower()
        results = []

        for entity in self.entities.values():
            text = f"{entity.name} {entity.signature} {entity.docstring or ''}".lower()
            score = 0.0

            # Exact name match
            if query_lower in entity.name.lower():
                score = 1.0
            # Partial name match
            elif query_lower in entity.name.lower():
                score = 0.8
            # Text match
            elif query_lower in text:
                score = 0.5

            if score > 0:
                results.append((entity, score))

        results.sort(key=lambda x: x[1], reverse=True)
        return results[:limit]

    def _hash_file(self, path: str) -> str:
        """Generate hash of file for dirty checking"""
        try:
            with open(path, "rb") as f:
                return hashlib.md5(f.read()).hexdigest()
        except Exception:
            return ""

    def _make_function_signature(
        self,
        func_info: Dict,
        class_name: Optional[str] = None
    ) -> str:
        """Create a function signature string"""
        prefix = f"{class_name}." if class_name else ""
        async_str = "async " if func_info.get("is_async") else ""
        args = ", ".join(func_info.get("args", []))
        returns = f" -> {func_info.get('returns')}" if func_info.get("returns") else ""
        return f"{async_str}def {prefix}{func_info['name']}({args}){returns}"

    def _generate_embeddings(self) -> None:
        """Generate embeddings for all entities"""
        texts = [entity.to_search_text() for entity in self.entities.values()]

        # Try to use sentence-transformers
        try:
            from sentence_transformers import SentenceTransformer
            model = SentenceTransformer(self.model_name)
            self.embeddings = model.encode(texts).tolist()
        except ImportError:
            self.embeddings = []

    def _encode_text(self, text: str) -> Optional[List[float]]:
        """Encode a single text to embedding"""
        if not self.embeddings:
            return None

        try:
            from sentence_transformers import SentenceTransformer
            model = SentenceTransformer(self.model_name)
            result = model.encode([text])[0].tolist()
            return list(result) if result is not None else None
        except ImportError:
            return None

    def _cosine_similarity(self, a: List[float], b: List[float]) -> float:
        """Calculate cosine similarity between two vectors"""
        import math
        dot = sum(x * y for x, y in zip(a, b))
        mag_a = math.sqrt(sum(x * x for x in a))
        mag_b = math.sqrt(sum(y * y for y in b))

        if mag_a == 0 or mag_b == 0:
            return 0.0

        return dot / (mag_a * mag_b)

    def get_stats(self) -> IndexStats:
        """Get index statistics"""
        stat_file = self.index_file
        last_updated = None

        try:
            if os.path.exists(stat_file):
                last_updated = datetime.fromtimestamp(os.path.getmtime(stat_file))
        except Exception:
            pass

        return IndexStats(
            total_entities=len(self.entities),
            files_indexed=len(set(e.file for e in self.entities.values())),
            last_updated=last_updated,
            index_path=self.index_path,
        )

    def clear(self) -> None:
        """Clear the index"""
        self.entities.clear()
        self.embeddings.clear()
        self._dirty = True

        # Remove files
        try:
            if os.path.exists(self.index_file):
                os.remove(self.index_file)
            if os.path.exists(self.embeddings_file):
                os.remove(self.embeddings_file)
        except Exception:
            pass


def get_semantic_index(
    index_path: Optional[str] = None,
    model_name: str = "sentence-transformers/all-MiniLM-L6-v2",
) -> SemanticIndex:
    """
    Get a semantic index instance.

    Args:
        index_path: Path to store index data
        model_name: Name of the embedding model

    Returns:
        SemanticIndex instance
    """
    index = SemanticIndex(index_path=index_path, model_name=model_name)
    index.load()
    return index
