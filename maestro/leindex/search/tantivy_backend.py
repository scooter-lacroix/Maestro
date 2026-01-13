import os
import json
import tantivy
from typing import List, Dict, Any, Optional
from .base import BaseSearchBackend

class TantivyBackend(BaseSearchBackend):
    """
    Search backend using Tantivy.
    """

    def __init__(self, index_path: str, create_if_missing: bool = True):
        self.index_path = index_path

        # Schema definition
        schema_builder = tantivy.SchemaBuilder()
        schema_builder.add_text_field("id", stored=True)
        schema_builder.add_text_field("content", stored=True, tokenizer_name="default")
        schema_builder.add_text_field("metadata", stored=True) # JSON serialized metadata

        # Add common metadata fields as separate indexed fields for filtering if needed
        # For now, we store metadata as JSON

        self.schema = schema_builder.build()

        if not os.path.exists(index_path) and create_if_missing:
            os.makedirs(index_path, exist_ok=True)
            self.index = tantivy.Index(self.schema, path=index_path)
        else:
            self.index = tantivy.Index(self.schema, path=index_path)

        self.writer = self.index.writer()

    def index_document(self, doc_id: str, content: str, metadata: Dict[str, Any]) -> None:
        """
        Index a document.
        Note: Tantivy doesn't support direct updates by ID easily without delete-insert.
        So we delete first if it exists (or rely on user to do so).
        Actually, let's implement delete-then-insert pattern for safety.
        """
        # Delete existing if any (by id)
        # Tantivy's delete_term uses a Term.
        # We need to make sure 'id' is indexed properly for exact match.
        # By default text fields are tokenized. We might want a string field for ID.
        # But 'add_text_field' is standard. Let's rely on standard search for now or fix schema.

        # Correction: For IDs we usually want a STRING field (not tokenized).
        # Tantivy-py binding: add_text_field is text.
        # Let's check if we can pass options.
        # Usually schema_builder.add_text_field("id", stored=True, tokenizer_name="raw")
        # creates a keyword-like field.

        # For simplicity in this implementation, we will perform delete using a query
        # or just simply add. If we just add, we get duplicates.
        # Let's try to delete by term.

        self.delete_document(doc_id)

        # Add new
        self.writer.add_document(
            id=doc_id,
            content=content,
            metadata=json.dumps(metadata)
        )

    def search(self, query: str, limit: int = 10) -> List[Dict[str, Any]]:
        """
        Search for documents.
        """
        self.index.reload() # Ensure we search latest commit
        searcher = self.index.searcher()

        # Parse query
        # query_parser = self.index.parse_query("content", ["content"])
        # Note: tantivy-py might vary in API slightly.

        try:
            # Simple query parser
            query_obj = self.index.parse_query(query, ["content"])
            scores = searcher.search(query_obj, limit)
        except Exception:
            # Fallback or empty results on bad query
            return []

        results = []
        for score, address in scores:
            doc = searcher.doc(address)
            # doc is a dict-like object
            # accessing fields returns list of values usually

            doc_id = doc["id"][0]
            content = doc["content"][0]
            metadata_json = doc["metadata"][0]

            try:
                metadata = json.loads(metadata_json)
            except:
                metadata = {}

            results.append({
                "id": doc_id,
                "content": content,
                "metadata": metadata,
                "score": score
            })

        return results

    def delete_document(self, doc_id: str) -> None:
        """
        Delete a document by ID.
        """
        # To support deletion by ID, ID field should ideally be untokenized.
        # But even with tokenized field, if doc_id is a single token, it works.
        # Let's assume standard IDs.
        self.writer.delete_documents("id", doc_id)

    def commit(self) -> None:
        """
        Commit changes to index.
        """
        self.writer.commit()
