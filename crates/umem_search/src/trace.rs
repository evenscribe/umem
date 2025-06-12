use anyhow::Result;
use memmap2::Mmap;
use std::{fs::File, path::PathBuf};

/// Traces represent documents that are matched against a user-prompt.
pub struct Trace {
    pub content: Mmap,
}

impl Trace {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Trace {
            content: Self::extract_content(&path)?,
        })
    }

    fn extract_content(path: &PathBuf) -> Result<Mmap> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(mmap)
    }
}
