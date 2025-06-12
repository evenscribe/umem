use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{IndexRecordOption, SchemaBuilder, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::TokenizerManager;
use tantivy::{doc, Document, Index, TantivyDocument};

use crate::trace::Trace;

pub struct TraceIndex;

const CONTENT: &str = "content";

impl TraceIndex {
    pub fn create_index(path: PathBuf) -> Result<()> {
        Self::create_index_path(&path)?;
        Self::build_schema(&path)
    }

    pub fn add_trace(index_path: PathBuf, trace: Trace) -> Result<()> {
        let index = Index::open_in_dir(&index_path)?;
        let schema = index.schema();
        let mut index_writer = index.writer_with_num_threads(num_cpus::get() / 2, 100_000_000)?;
        index_writer.add_document(doc!(
            schema.get_field(CONTENT)? => String::from_utf8_lossy(&trace.content[..]).to_string()
        ))?;
        index_writer.commit()?;
        index_writer.wait_merging_threads()?;
        Ok(())
    }

    pub fn parse_and_search(index_path: &PathBuf, query: &str) -> Result<String> {
        let index = Index::open_in_dir(index_path)?;
        let content = index.schema().get_field(CONTENT)?;
        let query_parser =
            QueryParser::new(index.schema(), vec![content], TokenizerManager::default());
        let reader = index.reader()?;
        let searcher = reader.searcher();
        let query = query_parser.parse_query(query)?;
        let (_, doc_address) = searcher
            .search(&query, &TopDocs::with_limit(1))?
            .into_iter()
            .next()
            .context("No search result found.")?;
        Ok(searcher
            .doc::<TantivyDocument>(doc_address)?
            .to_json(&index.schema()))
    }

    fn add_text_field(field_name: &str, schema_builder: &mut SchemaBuilder) {
        let mut text_options = TextOptions::default();
        text_options = text_options.set_stored();
        text_options = text_options.set_fast(None);
        let mut text_indexing_options = TextFieldIndexing::default()
            .set_index_option(IndexRecordOption::Basic)
            .set_tokenizer("en_stem");
        text_indexing_options =
            text_indexing_options.set_index_option(IndexRecordOption::WithFreqsAndPositions);
        text_options = text_options.set_indexing_options(text_indexing_options);
        schema_builder.add_text_field(field_name, text_options);
    }

    fn create_index_path(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            fs::create_dir(path)?;
        }
        Ok(())
    }

    fn build_schema(path: &PathBuf) -> Result<()> {
        if !Index::exists(&MmapDirectory::open(path)?)? {
            let mut schema_builder = SchemaBuilder::default();
            Self::add_text_field(CONTENT, &mut schema_builder);
            let schema = schema_builder.build();
            Index::create_in_dir(path, schema)?;
        }
        Ok(())
    }
}
