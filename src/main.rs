use anyhow::Result;
use umem_search::ProjectDirs;
use umem_search::TraceIndex;

fn main() -> Result<()> {
    TraceIndex::create_index(ProjectDirs::get_trace_index_path()?)?;
    Ok(())
}
