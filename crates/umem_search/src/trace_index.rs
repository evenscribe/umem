use crate::trace::Trace;
use anyhow::Result;
use std::{fs, path::Path};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    query::QueryParser,
    schema::{IndexRecordOption, SchemaBuilder, TextFieldIndexing, TextOptions},
    tokenizer::TokenizerManager,
    Document, Index, TantivyDocument,
};

pub struct TraceIndex;

const CONTENT: &str = "content";
const DEFAULT_SEARCH_COUNT: usize = 10;

impl TraceIndex {
    pub fn create_index<P: AsRef<Path>>(path: P) -> Result<()> {
        Self::create_index_path(&path)?;
        Self::build_schema(&path)
    }

    pub fn add_trace<P: AsRef<Path>>(index_path: P, trace: Trace) -> Result<()> {
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

    pub fn parse_and_search<P: AsRef<Path>>(
        index_path: P,
        query: &str,
        count: Option<usize>,
    ) -> Result<Vec<String>> {
        let count = count.unwrap_or(DEFAULT_SEARCH_COUNT);
        let index = Index::open_in_dir(index_path)?;
        let content = index.schema().get_field(CONTENT)?;
        let query_parser =
            QueryParser::new(index.schema(), vec![content], TokenizerManager::default());
        let reader = index.reader()?;
        let searcher = reader.searcher();
        let query = query_parser.parse_query(query)?;
        let mut results = Vec::with_capacity(DEFAULT_SEARCH_COUNT);
        for (_, doc_address) in searcher.search(&query, &TopDocs::with_limit(count))? {
            results.push(
                searcher
                    .doc::<TantivyDocument>(doc_address)?
                    .to_json(&index.schema()),
            );
        }
        Ok(results)
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

    fn create_index_path<P: AsRef<Path>>(path: &P) -> Result<()> {
        if !path.as_ref().exists() {
            fs::create_dir(path)?;
        }
        Ok(())
    }

    fn build_schema<P: AsRef<Path>>(path: &P) -> Result<()> {
        if !Index::exists(&MmapDirectory::open(path)?)? {
            let mut schema_builder = SchemaBuilder::default();
            Self::add_text_field(CONTENT, &mut schema_builder);
            let schema = schema_builder.build();
            Index::create_in_dir(path, schema)?;
        }
        Ok(())
    }
}
