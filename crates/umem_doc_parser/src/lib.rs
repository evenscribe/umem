use anyhow::Result;
use pandoc::{OutputFormat, OutputKind, PandocOutput};
use std::path::Path;

pub fn parse_to_plaintext<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut pandoc = pandoc::new();
    pandoc.add_input(&path);
    pandoc.set_output_format(OutputFormat::Plain, Vec::new());
    pandoc.set_output(OutputKind::Pipe);
    if let PandocOutput::ToBuffer(buffer) = pandoc.execute()? {
        return Ok(buffer);
    };
    unreachable!()
}
