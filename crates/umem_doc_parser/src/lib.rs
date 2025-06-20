use anyhow::Result;
use lopdf::Document;
use pandoc::{OutputFormat, OutputKind, PandocOutput};
use std::path::Path;
use umem_web_scrapper::Scrapper;

pub enum FileExtractionSource {
    PDF,
    OTHER,
}

pub struct Extractor;

impl Extractor {
    pub fn extract_from_file<P: AsRef<Path>>(
        path: P,
        source: FileExtractionSource,
    ) -> Result<String> {
        match source {
            FileExtractionSource::PDF => Self::extract_pdf(path),
            FileExtractionSource::OTHER => Self::extract_other(path),
        }
    }

    pub async fn extract_from_website(url: &str) -> Result<String> {
        let html_text = Scrapper::scrape(url).await?;
        Ok(mdka::from_html(&html_text))
    }

    fn extract_other<P: AsRef<Path>>(path: P) -> Result<String> {
        let mut pandoc = pandoc::new();
        pandoc.add_input(&path);
        pandoc.set_output_format(OutputFormat::Plain, Vec::new());
        pandoc.set_output(OutputKind::Pipe);
        if let PandocOutput::ToBuffer(buffer) = pandoc.execute()? {
            return Ok(buffer);
        };
        unreachable!()
    }

    fn extract_pdf<P: AsRef<Path>>(path: P) -> Result<String> {
        let document = Document::load(path)?;
        let pages = document.get_pages();
        let mut texts = Vec::new();
        for (i, _) in pages.iter().enumerate() {
            let page_number = (i + 1) as u32;
            let text = document.extract_text(&[page_number]);
            texts.push(text.unwrap_or_default());
        }
        Ok(texts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::{Extractor, FileExtractionSource};
    use anyhow::Result;
    use lopdf::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };
    use std::io::Write;
    use tempfile::{Builder, NamedTempFile};

    fn write_temp_file(content: &str, ext: &str) -> Result<(NamedTempFile, String)> {
        let mut file = Builder::new().suffix(ext).tempfile()?;
        write!(file, "{}", content)?;
        let path = file.path().to_string_lossy().to_string();
        Ok((file, path))
    }

    #[test]
    fn test_parse_to_plaintext_md() -> Result<()> {
        let (_file, path) = write_temp_file("# Hello\n\nThis is markdown.", ".md")?;
        let output = Extractor::extract_from_file(&path, FileExtractionSource::OTHER)?;
        assert!(output.contains("Hello"));
        assert!(output.contains("This is markdown"));
        Ok(())
    }

    #[test]
    fn test_parse_to_plaintext_txt() -> Result<()> {
        let (_file, path) = write_temp_file("Just a plain text document.", ".txt")?;
        let output = Extractor::extract_from_file(&path, FileExtractionSource::OTHER)?;
        assert!(output.contains("Just a plain text"));
        Ok(())
    }

    #[test]
    fn test_parse_pdf_to_string_simple() -> Result<()> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Courier",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 48.into()]),
                Operation::new("Td", vec![100.into(), 600.into()]),
                Operation::new("Tj", vec![Object::string_literal("Hello World!")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.compress();
        let file = NamedTempFile::new()?;
        doc.save(file.path())?;
        let output = Extractor::extract_from_file(file.path(), FileExtractionSource::PDF)?;
        assert!(output.contains("Hello World"));
        Ok(())
    }

    #[tokio::test]
    async fn test_extract_from_books_website() -> Result<()> {
        let url = "https://openai.com";
        let output = Extractor::extract_from_website(url).await?;
        print!("{}", output);
        assert!(output.contains("Help"));
        Ok(())
    }
}
