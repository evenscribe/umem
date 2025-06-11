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
    fn new(self, path: PathBuf) -> Self {
        Document {
            content: self.extract_content(&path),
            mime_type: self.extract_mime_type(&path),
        }
    }

    fn extract_mime_type(&self, path: &PathBuf) -> MimeType {
        todo!()
    }

    fn extract_content(&self, path: &PathBuf) -> Vec<u8> {
        todo!()
    }
}
