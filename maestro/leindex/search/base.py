from abc import ABC, abstractmethod
from typing import List, Dict, Any, Optional

class BaseSearchBackend(ABC):
    """Abstract base class for search backends."""

    @abstractmethod
    def index_document(self, doc_id: str, content: str, metadata: Dict[str, Any]) -> None:
        """
        Index a document.

        Args:
            doc_id: Unique identifier for the document.
            content: The text content of the document.
            metadata: Additional metadata associated with the document.
        """
        pass

    @abstractmethod
    def search(self, query: str, limit: int = 10) -> List[Dict[str, Any]]:
        """
        Search for documents matching the query.

        Args:
            query: The search query.
            limit: Maximum number of results to return.

        Returns:
            A list of search results.
        """
        pass

    @abstractmethod
    def delete_document(self, doc_id: str) -> None:
        """
        Delete a document from the index.

        Args:
            doc_id: Unique identifier of the document to delete.
        """
        pass
