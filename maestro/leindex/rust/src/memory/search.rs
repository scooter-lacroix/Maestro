use anyhow::Result;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};
// Unused tracing imports removed

/// Memory Search Index
pub struct MemorySearchIndex {
    index: Index,
    reader: IndexReader,
    schema: Schema,
}

impl MemorySearchIndex {
    /// Create or open a search index
    pub fn new(path: Option<PathBuf>) -> Result<Self> {
        let path = path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".maestro");
            p.push("search_index");
            p
        });

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let mut schema_builder = Schema::builder();
        schema_builder.add_i64_field("id", STORED | FAST);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("category", TEXT | STORED);
        schema_builder.add_text_field("tags", TEXT | STORED);
        let schema = schema_builder.build();

        let index = if Index::exists(&tantivy::directory::MmapDirectory::open(&path)?)? {
            Index::open_in_dir(&path)?
        } else {
            Index::create_in_dir(&path, schema.clone())?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self { index, reader, schema })
    }

    /// Index a memory
    pub fn index_memory(&self, id: i64, content: &str, category: &str, tags: Option<&str>) -> Result<()> {
        let mut index_writer: IndexWriter = self.index.writer(50_000_000)?; // 50MB heap
        
        let id_field = self.schema.get_field("id")?;
        let content_field = self.schema.get_field("content")?;
        let category_field = self.schema.get_field("category")?;
        let tags_field = self.schema.get_field("tags")?;

        let mut doc = TantivyDocument::default();
        doc.add_i64(id_field, id);
        doc.add_text(content_field, content);
        doc.add_text(category_field, category);
        if let Some(t) = tags {
            doc.add_text(tags_field, t);
        }

        index_writer.add_document(doc)?;
        index_writer.commit()?;
        Ok(())
    }

    /// Search memories
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<i64>> {
        let searcher = self.reader.searcher();
        let content_field = self.schema.get_field("content")?;
        let category_field = self.schema.get_field("category")?;
        
        let query_parser = QueryParser::for_index(&self.index, vec![content_field, category_field]);
        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        let id_field = self.schema.get_field("id")?;

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_val) = retrieved_doc.get_first(id_field).and_then(|v| v.as_i64()) {
                results.push(id_val);
            }
        }

        Ok(results)
    }

    /// Delete a memory from index
    pub fn delete_memory(&self, id: i64) -> Result<()> {
        let mut index_writer: IndexWriter = self.index.writer(10_000_000)?;
        let id_field = self.schema.get_field("id")?;
        index_writer.delete_term(Term::from_field_i64(id_field, id));
        index_writer.commit()?;
        Ok(())
    }
}
