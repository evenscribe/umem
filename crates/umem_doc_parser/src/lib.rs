use anyhow::{Context, Result};
use extractous::{PdfParserConfig, TesseractOcrConfig};
use std::path::Path;

pub struct Extractor;

impl Extractor {
    pub fn file_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
        Ok(extractous::Extractor::new()
            .set_ocr_config(TesseractOcrConfig::new())
            .set_pdf_config(PdfParserConfig::new())
            .extract_file_to_string(path.as_ref().to_str().context("path to str thing failed")?)?
            .0)
    }

    pub fn url_to_string(url: String) -> Result<String> {
        Ok(extractous::Extractor::new()
            .set_ocr_config(TesseractOcrConfig::new())
            .set_pdf_config(PdfParserConfig::new())
            .extract_url_to_string(&url)?
            .0)
    }
}
