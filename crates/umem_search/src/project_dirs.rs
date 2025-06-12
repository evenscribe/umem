use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_NAME: &str = "umem";
const TRACE_INDEX_PATH: &str = "trace_index";

pub struct ProjectDirs;

impl ProjectDirs {
    pub fn get_trace_index_path() -> Result<PathBuf> {
        Ok(Self::get_dirs()?.data_dir().join(TRACE_INDEX_PATH))
    }

    fn get_dirs() -> Result<directories::ProjectDirs> {
        directories::ProjectDirs::from("", "", APP_NAME).context("cannot get project_dirs for")
    }
}
