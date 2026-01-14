"""
Semantic Index for TLDR

Provides natural language search over code using embeddings.
"""

import os
import json
import hashlib
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Any, Tuple, TYPE_CHECKING
from pathlib import Path
from datetime import datetime

if TYPE_CHECKING:
    from maestro.tldr.ast import FunctionInfo


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


@dataclass
class IndexStats:
    """Statistics about the semantic index"""
    total_entities: int = 0
    files_indexed: int = 0
    last_updated: Optional[datetime] = None
    index_path: str = ""


class SemanticIndex:
    """
    Semantic Index for natural language code search

    Uses embeddings to enable natural language queries like
    "find functions that handle authentication" or "search for
    database connection code".
    """

    def __init__(
        self,
        index_path: Optional[str] = None,
        model_name: str = "sentence-transformers/all-MiniLM-L6-v2",
    ):
        """
        Initialize the semantic index

        Args:
            index_path: Path to store index data
            model_name: Name of the embedding model
        """
        if index_path is None:
            index_path = os.path.expanduser("~/.maestro/tldr-index")

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
        Load the index from disk

        Returns:
            True if loaded successfully
        """
        try:
            with open(self.index_file, "r", encoding="utf-8") as f:
                data = json.load(f)

            self.entities = {
                k: CodeEntity(**v) for k, v in data.get("entities", {}).items()
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
        Save the index to disk

        Returns:
            True if saved successfully
        """
        try:
            os.makedirs(self.index_path, exist_ok=True)

            # Save entities
            data = {
                "entities": {
                    k: {
                        "id": v.id,
                        "name": v.name,
                        "type": v.type,
                        "file": v.file,
                        "line": v.line,
                        "end_line": v.end_line,
                        "signature": v.signature,
                        "docstring": v.docstring,
                        "class_name": v.class_name,
                        "module": v.module,
                    }
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
        Index a single file

        Args:
            path: Path to the file
            force: Re-index even if unchanged

        Returns:
            Number of entities indexed
        """
        from maestro.tldr.ast import ASTAnalyzer

        analyzer = ASTAnalyzer()
        analysis = analyzer.analyze_file(path)

        if not analysis:
            return 0

        # Generate file hash for dirty checking
        file_hash = self._hash_file(path)

        count = 0
        module = os.path.splitext(os.path.basename(path))[0]

        # Index functions
        for func_name, func_info in analysis.functions.items():
            entity_id = f"{path}:{func_name}"
            signature = self._make_function_signature(func_info)

            entity = CodeEntity(
                id=entity_id,
                name=func_name,
                type="function",
                file=path,
                line=func_info.line,
                end_line=func_info.end_line,
                signature=signature,
                docstring=func_info.docstring,
                module=module,
            )

            self.entities[entity_id] = entity
            count += 1

        # Index classes and methods
        for cls_name, cls_info in analysis.classes.items():
            cls_id = f"{path}:{cls_name}"

            # Index the class itself
            cls_entity = CodeEntity(
                id=cls_id,
                name=cls_name,
                type="class",
                file=path,
                line=cls_info.line,
                end_line=cls_info.end_line,
                signature=f"class {cls_name}",
                docstring=cls_info.docstring,
                module=module,
            )
            self.entities[cls_id] = cls_entity
            count += 1

            # Index methods
            for method_name, method_info in cls_info.methods.items():
                method_id = f"{path}:{cls_name}.{method_name}"
                signature = self._make_function_signature(method_info, cls_name)

                entity = CodeEntity(
                    id=method_id,
                    name=method_name,
                    type="method",
                    file=path,
                    line=method_info.line,
                    end_line=method_info.end_line,
                    signature=signature,
                    docstring=method_info.docstring,
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
        Index an entire project

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
        Search for code using natural language

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

    def _make_function_signature(self, func_info: "FunctionInfo", class_name: Optional[str] = None) -> str:
        """Create a function signature string"""
        prefix = f"{class_name}." if class_name else ""
        async_str = "async " if func_info.is_async else ""
        args = ", ".join(func_info.args)
        returns = f" -> {func_info.returns}" if func_info.returns else ""
        return f"{async_str}def {prefix}{func_info.name}({args}){returns}"

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
