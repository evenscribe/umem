use std::path::PathBuf;

enum MimeType {
    Pdf,
    PlainText,
}

struct Document {
    // TODO: Use <https://docs.rs/memmap2/latest/memmap2/> later for file interaction to speed
    // things up
    content: Vec<u8>,
    mime_type: MimeType,
}

impl Document {
    fn new(path: PathBuf) -> Self {
        Document {
            content: Document::extract_content(&path),
            mime_type: Document::extract_mime_type(&path),
        }
    }

    fn extract_mime_type(path: &PathBuf) -> MimeType {
        todo!()
    }

    fn extract_content(path: &PathBuf) -> Vec<u8> {
        todo!()
    }
}
