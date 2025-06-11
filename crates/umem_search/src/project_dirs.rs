use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct ProjectDirs;

const TRACE_INDEX_PATH: &str = "trace_index";

impl ProjectDirs {
    pub fn get_trace_index_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir().context("Couldn't locate data_dir.")?;
        Ok(data_dir.join(TRACE_INDEX_PATH))
    }
}
