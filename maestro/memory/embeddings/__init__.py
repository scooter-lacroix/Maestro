"""
Embeddings Service for Semantic Memory Search

Provides vector embeddings for memories using sentence-transformers
and stores them in sqlite-vec for efficient similarity search.
"""

from maestro.memory.embeddings.service import (
    EmbeddingsService,
    SimpleEmbeddingsService,
    get_embeddings_service,
    get_simple_embeddings_service,
)

__all__ = [
    "EmbeddingsService",
    "SimpleEmbeddingsService",
    "get_embeddings_service",
    "get_simple_embeddings_service",
]
