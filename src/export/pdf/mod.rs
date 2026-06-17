mod content;
mod layout;
mod render;
mod writer;

use std::fs;
use std::path::Path;

use crate::models::Cemetery;

use content::PdfContent;
use layout::PageLayout;
use writer::PdfBuilder;

#[derive(Debug, Clone)]
pub struct PdfExportOptions {
    pub title: String,
    pub subtitle: String,
    pub empty_message: String,
    pub footer: String,
}

#[derive(Debug)]
pub enum PdfExportError {
    Write(std::io::Error),
}

impl std::fmt::Display for PdfExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PdfExportError {}

impl From<std::io::Error> for PdfExportError {
    fn from(error: std::io::Error) -> Self {
        Self::Write(error)
    }
}

pub fn export_cemetery_map(
    cemetery: &Cemetery,
    destination: &Path,
    options: &PdfExportOptions,
) -> Result<(), PdfExportError> {
    fs::write(destination, build_pdf(cemetery, options))?;
    Ok(())
}

fn build_pdf(cemetery: &Cemetery, options: &PdfExportOptions) -> Vec<u8> {
    let layout = PageLayout::A0_LANDSCAPE;
    let mut content = PdfContent::default();

    render::header(&mut content, options, layout);
    render::map(cemetery, &mut content, options, layout);
    render::footer(&mut content, options, layout);

    PdfBuilder::new(layout, content.finish()).finish()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use iced::{Point, Size};

    use crate::models::{Cemetery, GraveColor, GraveRectangle, PersonDate};

    use super::*;

    fn options() -> PdfExportOptions {
        PdfExportOptions {
            title: "Central Cemetery".to_owned(),
            subtitle: "Printable cemetery map".to_owned(),
            empty_message: "No graves to export".to_owned(),
            footer: "1 grave".to_owned(),
        }
    }

    #[test]
    fn exports_non_empty_cemetery_as_pdf() {
        let mut cemetery = Cemetery::default();
        let grave_id = cemetery.add_grave_with_color(
            GraveRectangle::from_top_left_size(Point::new(10.0, 20.0), Size::new(40.0, 24.0)),
            GraveColor::DEFAULT,
        );
        cemetery.create_person_with_details(
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            Some(grave_id),
        );

        let path = std::env::temp_dir().join("requiescat-map-export-test.pdf");
        export_cemetery_map(&cemetery, &path, &options()).unwrap();

        let bytes = fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("Ada Lovelace"));
        assert!(text.contains("Central Cemetery"));
        assert!(text.contains("/MediaBox [0 0 3370.39 2383.94]"));
    }

    #[test]
    fn exports_empty_cemetery_with_empty_state() {
        let path = std::env::temp_dir().join("requiescat-empty-map-export-test.pdf");
        export_cemetery_map(&Cemetery::default(), &path, &options()).unwrap();

        let bytes = fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("No graves to export"));
    }

    #[test]
    fn transliterates_romanian_characters_for_builtin_pdf_font() {
        assert_eq!(
            content::pdf_safe_text("Ștefănescu Țara Întâi"),
            "Stefanescu Tara Intai"
        );
    }
}
