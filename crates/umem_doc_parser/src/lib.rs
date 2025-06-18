use anyhow::Result;
use extractous::{PdfParserConfig, TesseractOcrConfig};
use std::path::Path;

lazy_static::lazy_static! {
    static ref EXTRACTOR: extractous::Extractor = {
        extractous::Extractor::new()
            .set_ocr_config(TesseractOcrConfig::new())
            .set_pdf_config(PdfParserConfig::new())
    };
}

pub struct Extractor;

impl Extractor {
    pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
        Ok(EXTRACTOR
            .extract_file_to_string(path.as_ref().to_string_lossy().as_ref())?
            .0)
    }

    pub fn url_to_string(url: String) -> Result<String> {
        Ok(EXTRACTOR.extract_url_to_string(&url)?.0)
    }
}
