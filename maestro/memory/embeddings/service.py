"""
Embeddings Service for Semantic Memory Search

Provides vector embeddings for memories using sentence-transformers
and stores them in sqlite-vec for efficient similarity search.

Includes circuit breaker pattern for external service calls with
timeout and retry logic with exponential backoff.
"""

import os
import time
import threading
from typing import Optional, Dict, Any, List, Tuple, TYPE_CHECKING, Union
from threading import Lock, RLock
from datetime import datetime, timedelta, UTC
from enum import Enum
from builtins import BaseException

import numpy as np

if TYPE_CHECKING:
    from sentence_transformers import SentenceTransformer


class CircuitBreakerState(Enum):
    """Circuit breaker states"""
    CLOSED = "closed"  # Normal operation
    OPEN = "open"  # Circuit is open, calls fail immediately
    HALF_OPEN = "half_open"  # Testing if service has recovered


class CircuitBreaker:
    """
    Circuit breaker pattern for external service calls

    Prevents cascading failures by stopping calls to a failing service
    after a threshold of failures is reached.
    """

    def __init__(
        self,
        failure_threshold: int = 5,
        recovery_timeout: int = 60,
        expected_exception: Union[type[BaseException], Tuple[type[BaseException], ...]] = Exception,
    ):
        """
        Initialize the circuit breaker

        Args:
            failure_threshold: Number of failures before opening circuit
            recovery_timeout: Seconds to wait before trying again
            expected_exception: Exception type that indicates failure
        """
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.expected_exception = expected_exception

        self._failure_count = 0
        self._last_failure_time: Optional[datetime] = None
        self._state = CircuitBreakerState.CLOSED
        self._lock = RLock()

    def call(self, func: Any, *args: Any, **kwargs: Any) -> Any:
        """
        Execute a function with circuit breaker protection

        Args:
            func: Function to call
            *args: Function arguments
            **kwargs: Function keyword arguments

        Returns:
            Function result

        Raises:
            Exception: If circuit is open or function fails
        """
        with self._lock:
            if self._state == CircuitBreakerState.OPEN:
                if self._should_attempt_reset():
                    self._state = CircuitBreakerState.HALF_OPEN
                else:
                    raise Exception(
                        f"Circuit breaker is OPEN. "
                        f"Recovery timeout: {self.recovery_timeout}s"
                    )

        try:
            result = func(*args, **kwargs)
            self._on_success()
            return result
        except self.expected_exception as e:
            self._on_failure()
            raise e

    def _should_attempt_reset(self) -> bool:
        """Check if enough time has passed to attempt recovery"""
        if self._last_failure_time is None:
            return True
        elapsed = (datetime.now(UTC) - self._last_failure_time).total_seconds()
        return elapsed >= self.recovery_timeout

    def _on_success(self) -> None:
        """Handle successful call"""
        with self._lock:
            self._failure_count = 0
            if self._state == CircuitBreakerState.HALF_OPEN:
                self._state = CircuitBreakerState.CLOSED

    def _on_failure(self) -> None:
        """Handle failed call"""
        with self._lock:
            self._failure_count += 1
            self._last_failure_time = datetime.now(UTC)

            if self._failure_count >= self.failure_threshold:
                self._state = CircuitBreakerState.OPEN

    def get_state(self) -> CircuitBreakerState:
        """Get current circuit breaker state"""
        with self._lock:
            return self._state

    def reset(self) -> None:
        """Reset the circuit breaker to closed state"""
        with self._lock:
            self._failure_count = 0
            self._last_failure_time = None
            self._state = CircuitBreakerState.CLOSED


class RetryPolicy:
    """
    Retry policy with exponential backoff for external service calls
    """

    def __init__(
        self,
        max_attempts: int = 3,
        base_delay: float = 1.0,
        max_delay: float = 10.0,
        exponential_base: float = 2.0,
    ):
        """
        Initialize retry policy

        Args:
            max_attempts: Maximum number of retry attempts
            base_delay: Initial delay between retries in seconds
            max_delay: Maximum delay between retries in seconds
            exponential_base: Base for exponential backoff calculation
        """
        self.max_attempts = max_attempts
        self.base_delay = base_delay
        self.max_delay = max_delay
        self.exponential_base = exponential_base

    def execute(self, func: Any, *args: Any, **kwargs: Any) -> Any:
        """
        Execute function with retry logic

        Args:
            func: Function to execute
            *args: Function arguments
            **kwargs: Function keyword arguments

        Returns:
            Function result

        Raises:
            Exception: If all retry attempts fail
        """
        last_exception = None

        for attempt in range(self.max_attempts):
            try:
                return func(*args, **kwargs)
            except Exception as e:
                last_exception = e

                if attempt < self.max_attempts - 1:
                    # Calculate delay with exponential backoff
                    delay = min(
                        self.base_delay * (self.exponential_base ** attempt),
                        self.max_delay
                    )
                    time.sleep(delay)

        raise last_exception or Exception("All retry attempts failed")


class EmbeddingsService:
    """
    Service for generating and managing embeddings

    Uses sentence-transformers for embedding generation and
    sqlite-vec for vector storage and similarity search.

    Includes circuit breaker and retry logic for external service calls.
    """

    def __init__(
        self,
        model_name: str = "sentence-transformers/all-MiniLM-L6-v2",
        dimensions: int = 384,
        db_path: Optional[str] = None,
        config: Optional[Dict[str, Any]] = None,
        timeout: int = 30,
        max_retries: int = 3,
        circuit_breaker_threshold: int = 5,
        circuit_breaker_timeout: int = 60,
    ):
        """
        Initialize the embeddings service

        Args:
            model_name: Name of the sentence-transformers model
            dimensions: Embedding dimensions
            db_path: Path to database (for vec table)
            config: Optional configuration
            timeout: Timeout in seconds for encoding operations
            max_retries: Maximum number of retry attempts
            circuit_breaker_threshold: Failures before opening circuit
            circuit_breaker_timeout: Seconds before circuit recovery attempt
        """
        self.model_name = model_name
        self.dimensions = dimensions
        self.db_path = db_path or os.path.expanduser("~/.maestro/memory.db")
        self.config = config or {}
        self.timeout = timeout

        self._model: Optional['SentenceTransformer'] = None
        self._lock = Lock()
        self._vec_available = False

        # Initialize circuit breaker for encoding operations
        self._circuit_breaker = CircuitBreaker(
            failure_threshold=circuit_breaker_threshold,
            recovery_timeout=circuit_breaker_timeout,
        )

        # Initialize retry policy
        self._retry_policy = RetryPolicy(
            max_attempts=max_retries,
            base_delay=1.0,
            max_delay=10.0,
        )

        # Check if vec is available
        self._check_vec_available()

    def _check_vec_available(self) -> None:
        """Check if sqlite-vec extension is available"""
        try:
            import sqlite3
            conn = sqlite3.connect(self.db_path)
            conn.enable_load_extension(True)
            conn.load_extension("vec0")
            conn.close()
            self._vec_available = True
        except Exception:
            self._vec_available = False

    @property
    def model(self) -> Optional['SentenceTransformer']:
        """Lazy load the embedding model"""
        if self._model is None:
            with self._lock:
                if self._model is None:
                    self._model = self._load_model()
        return self._model

    def _load_model(self) -> Optional['SentenceTransformer']:
        """
        Load the sentence-transformers model

        First checks if torch is available in the venv or system Python.
        Uses importlib to load from system packages if needed.

        Returns:
            SentenceTransformer model or None if dependencies unavailable
        """
        import sys
        import importlib.util
        from pathlib import Path

        # First check if torch is available in the current venv
        torch_available = False
        system_site_packages = None

        try:
            import torch
            torch_available = True
        except ImportError:
            # Not in venv - check system Python for torch
            system_pythons = [
                "/usr/bin/python3",
                "/usr/local/bin/python3",
                "/opt/homebrew/bin/python3",
                "/home/stan/anaconda3/bin/python",
                "/home/stan/.local/bin/python3",
            ]

            for py_path in system_pythons:
                if Path(py_path).exists():
                    try:
                        import subprocess
                        import json
                        # Get ALL site-packages paths from sys.path (includes user paths)
                        result = subprocess.run(
                            [py_path, "-c", "import sys, json; print(json.dumps([p for p in sys.path if 'site-packages' in p]))"],
                            capture_output=True, text=True, timeout=5
                        )
                        if result.returncode == 0:
                            site_packages_paths = json.loads(result.stdout.strip())
                            # Verify torch is actually there
                            torch_check = subprocess.run(
                                [py_path, "-c", "import torch; print('OK')"],
                                capture_output=True, text=True, timeout=5
                            )
                            if torch_check.returncode == 0 and "OK" in torch_check.stdout:
                                torch_available = True
                                system_site_packages = ":".join(site_packages_paths)
                                # Add all to sys.path so subsequent imports work
                                for sp in site_packages_paths:
                                    if sp not in sys.path:
                                        sys.path.insert(0, sp)
                                break
                    except (FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError):
                        continue

        if not torch_available:
            # Torch not available anywhere
            return None

        # Torch is available (in venv or system), try to load sentence-transformers
        try:
            from sentence_transformers import SentenceTransformer
            return SentenceTransformer(self.model_name)
        except ImportError:
            # sentence-transformers not installed where torch is
            # Try to import it explicitly from system site-packages
            if system_site_packages:
                try:
                    # Find sentence_transformers module
                    for sp in system_site_packages.split(":"):
                        st_path = Path(sp) / "sentence_transformers"
                        if st_path.exists():
                            # Add to path if not already there
                            import sys
                            if str(sp) not in sys.path:
                                sys.path.insert(0, sp)
                            # Try import again
                            from sentence_transformers import SentenceTransformer
                            return SentenceTransformer(self.model_name)
                except Exception:
                    pass
            return None

    def is_available(self) -> bool:
        """
        Check if the embeddings service is available

        Returns:
            True if both sentence-transformers and sqlite-vec are available
        """
        return self.model is not None and self._vec_available

    def encode(
        self,
        texts: List[str],
        batch_size: int = 32,
        show_progress: bool = False,
    ) -> Any:
        """
        Encode texts to embeddings with timeout, retry, and circuit breaker

        Args:
            texts: List of texts to encode
            batch_size: Batch size for encoding
            show_progress: Whether to show progress

        Returns:
            Embeddings array or None if model not available
        """
        if self.model is None:
            return None

        def _do_encode() -> np.ndarray:
            """Internal encoding function with timeout protection"""
            import signal

            def timeout_handler(signum: int, frame: Any) -> None:
                raise TimeoutError(f"Encoding timed out after {self.timeout}s")

            # Set timeout if supported
            try:
                old_handler = signal.signal(signal.SIGALRM, timeout_handler)
                signal.alarm(self.timeout)
            except (AttributeError, ValueError):
                # Windows or no SIGALRM support
                old_handler = None

            # At this point, self.model is guaranteed to not be None
            # since we checked before calling _do_encode
            assert self.model is not None
            try:
                embeddings = self.model.encode(
                    texts,
                    batch_size=batch_size,
                    show_progress_bar=show_progress,
                    convert_to_numpy=True,
                )
                return embeddings
            finally:
                if old_handler is not None:
                    signal.alarm(0)
                    signal.signal(signal.SIGALRM, old_handler)

        # Apply retry policy and circuit breaker
        try:
            return self._retry_policy.execute(
                lambda: self._circuit_breaker.call(_do_encode)
            )
        except (TimeoutError, Exception):
            # Log error but don't crash
            return None

    def encode_single(
        self,
        text: str,
    ) -> Any:
        """
        Encode a single text to embedding

        Args:
            text: Text to encode

        Returns:
            Embedding array or None if model not available
        """
        result = self.encode([text])
        return result[0] if result is not None else None

    def store_embedding(
        self,
        memory_id: int,
        embedding: np.ndarray,
    ) -> bool:
        """
        Store an embedding in the vec table

        Args:
            memory_id: Memory ID
            embedding: Embedding vector

        Returns:
            True if stored successfully
        """
        if not self._vec_available:
            return False

        try:
            import sqlite3
            conn = sqlite3.connect(self.db_path)
            conn.enable_load_extension(True)
            conn.load_extension("vec0")

            # Create vec table if not exists
            conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS memory_embeddings
                USING vec0(embedding_id INTEGER PRIMARY KEY, embedding FLOAT[384])
            """)

            # Insert embedding
            embedding_bytes = np.float32(embedding).tobytes()
            conn.execute(
                "INSERT OR REPLACE INTO memory_embeddings (embedding_id, embedding) VALUES (?, ?)",
                (memory_id, embedding_bytes)
            )
            conn.commit()
            conn.close()
            return True
        except Exception:
            return False

    def search_similar(
        self,
        query_embedding: np.ndarray,
        limit: int = 10,
        threshold: float = 0.0,
    ) -> List[Tuple[int, float]]:
        """
        Search for similar embeddings

        Args:
            query_embedding: Query vector
            limit: Maximum results
            threshold: Minimum similarity threshold

        Returns:
            List of (memory_id, score) tuples
        """
        if not self._vec_available:
            return []

        try:
            import sqlite3
            conn = sqlite3.connect(self.db_path)
            conn.enable_load_extension(True)
            conn.load_extension("vec0")

            query_bytes = np.float32(query_embedding).tobytes()

            # Use vec distance search
            # Lower distance = more similar
            results = conn.execute("""
                SELECT embedding_id, distance
                FROM memory_embeddings
                WHERE embedding MATCH ?
                ORDER BY distance
                LIMIT ?
            """, (query_bytes, limit * 2)).fetchall()

            conn.close()

            # Convert distance to similarity and filter by threshold
            # Distance is Euclidean, convert to cosine-like score
            output = []
            for memory_id, distance in results:
                # Simple conversion: max(0, 1 - distance/sqrt(2))
                similarity: float = max(0.0, 1.0 - float(distance) / 1.414)
                if similarity >= threshold:
                    output.append((memory_id, similarity))

            return sorted(output, key=lambda x: x[1], reverse=True)[:limit]

        except Exception:
            return []

    def semantic_search_memories(
        self,
        query: str,
        session: Any,  # SQLAlchemy session
        limit: int = 10,
        threshold: float = 0.75,
        category: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """
        Perform semantic search on memories

        Args:
            query: Search query
            session: SQLAlchemy session
            limit: Maximum results
            threshold: Minimum similarity threshold
            category: Optional category filter

        Returns:
            List of matching memory dictionaries
        """
        # Generate query embedding
        query_embedding = self.encode_single(query)
        if query_embedding is None:
            return []

        # Search similar embeddings
        similar_ids = self.search_similar(
            query_embedding,
            limit=limit * 2,
            threshold=threshold,
        )

        if not similar_ids:
            return []

        # Fetch memory records
        from maestro.memory.database.models import Memory
        from sqlalchemy import select, and_
        from datetime import datetime, UTC

        memory_ids = [mid for mid, _ in similar_ids]
        scores = {mid: score for mid, score in similar_ids}

        stmt = select(Memory).where(
            and_(
                Memory.id.in_(memory_ids),
                Memory.expires_at > datetime.now(UTC),
            )
        )

        if category:
            stmt = stmt.where(Memory.category == category)

        memories = session.execute(stmt).scalars().all()

        # Combine with scores
        results = []
        for memory in memories:
            if memory.id in scores:
                result = memory.to_dict()
                result["similarity"] = scores[memory.id]
                results.append(result)

        # Sort by similarity
        results.sort(key=lambda x: x["similarity"], reverse=True)

        return results[:limit]

    def index_memory(
        self,
        memory_id: int,
        content: str,
    ) -> bool:
        """
        Index a memory for semantic search

        Args:
            memory_id: Memory ID
            content: Memory content

        Returns:
            True if indexed successfully
        """
        embedding = self.encode_single(content)
        if embedding is None:
            return False

        return self.store_embedding(memory_id, embedding)

    def batch_index_memories(
        self,
        memories: List[Tuple[int, str]],
        batch_size: int = 32,
    ) -> int:
        """
        Batch index memories

        Args:
            memories: List of (memory_id, content) tuples
            batch_size: Batch size for encoding

        Returns:
            Number of memories successfully indexed
        """
        indexed = 0

        for i in range(0, len(memories), batch_size):
            batch = memories[i:i + batch_size]
            contents = [content for _, content in batch]

            embeddings = self.encode(contents, batch_size=batch_size)
            if embeddings is None:
                continue

            for (memory_id, _), embedding in zip(batch, embeddings):
                if self.store_embedding(memory_id, embedding):
                    indexed += 1

        return indexed

    def delete_embedding(self, memory_id: int) -> bool:
        """
        Delete an embedding from the vec table

        Args:
            memory_id: Memory ID

        Returns:
            True if deleted successfully
        """
        if not self._vec_available:
            return False

        try:
            import sqlite3
            conn = sqlite3.connect(self.db_path)
            conn.enable_load_extension(True)
            conn.load_extension("vec0")

            conn.execute(
                "DELETE FROM memory_embeddings WHERE embedding_id = ?",
                (memory_id,)
            )
            conn.commit()
            conn.close()
            return True
        except Exception:
            return False

    def get_stats(self) -> Dict[str, Any]:
        """
        Get statistics about the embeddings

        Returns:
            Statistics dictionary including circuit breaker state
        """
        stats = {
            "model_available": self.model is not None,
            "model_name": self.model_name,
            "dimensions": self.dimensions,
            "vec_available": self._vec_available,
            "total_embeddings": 0,
            "circuit_breaker_state": self._circuit_breaker.get_state().value,
            "timeout": self.timeout,
        }

        if self._vec_available:
            try:
                import sqlite3
                conn = sqlite3.connect(self.db_path)
                conn.enable_load_extension(True)
                conn.load_extension("vec0")

                # Check if table exists
                table_check = conn.execute(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='memory_embeddings'"
                ).fetchone()

                if table_check:
                    count = conn.execute("SELECT COUNT(*) FROM memory_embeddings").fetchone()[0]
                    stats["total_embeddings"] = count

                conn.close()
            except Exception:
                pass

        return stats

    def reset_circuit_breaker(self) -> None:
        """
        Reset the circuit breaker to closed state

        Use this if the external service has recovered and you want to
        immediately resume operations.
        """
        self._circuit_breaker.reset()

    def get_circuit_breaker_state(self) -> CircuitBreakerState:
        """Get the current circuit breaker state"""
        return self._circuit_breaker.get_state()


class SimpleEmbeddingsService:
    """
    Simple fallback embeddings service

    Uses basic text similarity when sentence-transformers
    is not available. Provides degraded but functional semantic search.
    """

    def __init__(
        self,
        db_path: Optional[str] = None,
    ):
        """
        Initialize the simple embeddings service

        Args:
            db_path: Path to database
        """
        self.db_path = db_path or os.path.expanduser("~/.maestro/memory.db")

    def encode(self, texts: List[str]) -> List[List[float]]:
        """
        Encode texts using simple word overlap

        This is a fallback when sentence-transformers is not available.

        Args:
            texts: List of texts to encode

        Returns:
            List of encoded vectors
        """
        # Simple character-level encoding for fallback
        vectors = []
        for text in texts:
            # Create a simple character frequency vector
            text_lower = text.lower()
            vector = [0.0] * 256  # 256 dimensions for ASCII chars

            for char in text_lower:
                idx = ord(char) % 256
                vector[idx] += 1.0

            # Normalize
            total = sum(vector)
            if total > 0:
                vector = [v / total for v in vector]

            vectors.append(vector)

        return vectors

    def encode_single(self, text: str) -> List[float]:
        """
        Encode a single text

        Args:
            text: Text to encode

        Returns:
            Encoded vector
        """
        result = self.encode([text])
        return result[0] if result else []

    def cosine_similarity(
        self,
        vec1: List[float],
        vec2: List[float],
    ) -> float:
        """
        Calculate cosine similarity between two vectors

        Args:
            vec1: First vector
            vec2: Second vector

        Returns:
            Similarity score
        """
        dot = sum(a * b for a, b in zip(vec1, vec2))
        mag1 = sum(a * a for a in vec1) ** 0.5
        mag2 = sum(b * b for b in vec2) ** 0.5

        if mag1 == 0 or mag2 == 0:
            return 0.0

        result: float = dot / (mag1 * mag2)
        return result

    def search_similar_memories(
        self,
        query: str,
        memories: List[Any],
        limit: int = 10,
        threshold: float = 0.1,
    ) -> List[Tuple[Any, float]]:
        """
        Search for similar memories using simple similarity

        Args:
            query: Search query
            memories: List of Memory objects
            limit: Maximum results
            threshold: Minimum similarity threshold

        Returns:
            List of (memory, score) tuples
        """
        query_vec = self.encode_single(query)

        results = []
        for memory in memories:
            # Simple encoding of memory content
            memory_vec = self.encode_single(memory.content)
            score = self.cosine_similarity(query_vec, memory_vec)

            if score >= threshold:
                results.append((memory, score))

        # Sort by score
        results.sort(key=lambda x: x[1], reverse=True)

        return results[:limit]


def get_embeddings_service(
    model_name: Optional[str] = None,
    db_path: Optional[str] = None,
    config: Optional[Dict[str, Any]] = None,
) -> EmbeddingsService:
    """
    Get the embeddings service

    Args:
        model_name: Optional model name
        db_path: Optional database path
        config: Optional configuration

    Returns:
        EmbeddingsService instance
    """
    if model_name is None:
        model_name = "sentence-transformers/all-MiniLM-L6-v2"

    return EmbeddingsService(
        model_name=model_name,
        db_path=db_path,
        config=config,
    )


def check_embeddings_setup() -> Dict[str, Any]:
    """
    Check the embeddings setup and provide helpful information

    Checks both venv and system Python for torch/sentence-transformers.

    Returns:
        Dictionary with setup status and recommendations
    """
    import sys
    import subprocess
    from pathlib import Path

    result: Dict[str, Any] = {
        "torch_available": False,
        "torch_version": None,
        "torch_location": None,
        "sentence_transformers_available": False,
        "st_version": None,
        "st_location": None,
        "service_available": False,
        "recommendation": None,
    }

    # Check torch in current venv
    try:
        import torch
        result["torch_available"] = True
        result["torch_version"] = torch.__version__
        result["torch_location"] = "venv"
    except ImportError:
        pass

    # If not in venv, check system Python
    if not result["torch_available"]:
        system_pythons = [
            "/usr/bin/python3",
            "/usr/local/bin/python3",
            "/opt/homebrew/bin/python3",
            "/home/stan/anaconda3/bin/python",
            "/home/stan/.local/bin/python3",
        ]
        for py_path in system_pythons:
            if Path(py_path).exists():
                try:
                    test_cmd = f"{py_path} -c \"import torch; print(torch.__version__)\""
                    proc = subprocess.run(
                        test_cmd, shell=True, capture_output=True, text=True, timeout=5
                    )
                    if proc.returncode == 0 and "Traceback" not in proc.stderr:
                        result["torch_available"] = True
                        result["torch_version"] = proc.stdout.strip()
                        result["torch_location"] = f"system ({py_path})"
                        break
                except (FileNotFoundError, subprocess.TimeoutExpired):
                    continue

    if not result["torch_available"]:
        result["recommendation"] = (
            "PyTorch not found. For GPU support (ROCm/CUDA), install PyTorch in system Python. "
            "Then install sentence-transformers in system Python with: pip install sentence-transformers"
        )
        return result

    # Check sentence-transformers in current venv
    try:
        from sentence_transformers import __version__ as st_version
        result["sentence_transformers_available"] = True
        result["st_version"] = st_version
        result["st_location"] = "venv"
    except ImportError:
        # Check system Python for sentence-transformers
        if result["torch_location"] != "venv":
            # torch is in system, check if st is there too
            for py_path in ["/home/stan/anaconda3/bin/python", "/usr/bin/python3", "/usr/local/bin/python3", "/home/stan/.local/bin/python3"]:
                if Path(py_path).exists():
                    try:
                        test_cmd = f"{py_path} -c \"from sentence_transformers import __version__; print(__version__)\""
                        proc = subprocess.run(
                            test_cmd, shell=True, capture_output=True, text=True, timeout=5
                        )
                        if proc.returncode == 0 and "Traceback" not in proc.stderr:
                            result["sentence_transformers_available"] = True
                            result["st_version"] = proc.stdout.strip()
                            result["st_location"] = f"system ({py_path})"
                            break
                    except (FileNotFoundError, subprocess.TimeoutExpired):
                        continue

    # Generate recommendation
    if result["torch_available"] and not result["sentence_transformers_available"]:
        if result["torch_location"] == "venv":
            result["recommendation"] = (
                f"PyTorch {result['torch_version']} in venv. "
                "Install sentence-transformers with: uv sync --extra embeddings"
            )
        else:
            result["recommendation"] = (
                f"PyTorch {result['torch_version']} detected in system Python! "
                "Install sentence-transformers in system Python with:\n"
                "  pip install sentence-transformers"
            )
        return result

    # Check if service works
    try:
        service = get_embeddings_service()
        result["service_available"] = service.is_available()
    except Exception:
        pass

    if result["service_available"]:
        result["recommendation"] = "Embeddings service fully operational!"
    elif result["sentence_transformers_available"]:
        result["recommendation"] = "Dependencies installed but service may have issues. Check sqlite-vec."

    return result


def get_simple_embeddings_service(
    db_path: Optional[str] = None,
) -> SimpleEmbeddingsService:
    """
    Get the simple fallback embeddings service

    Args:
        db_path: Optional database path

    Returns:
        SimpleEmbeddingsService instance
    """
    return SimpleEmbeddingsService(db_path=db_path)
