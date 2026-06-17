use std::fmt::Write as _;

use super::layout::PageLayout;

pub(super) struct PdfBuilder {
    objects: Vec<String>,
}

impl PdfBuilder {
    pub(super) fn new(layout: PageLayout, content: String) -> Self {
        let content_length = content.len();
        Self {
            objects: vec![
                "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>",
                    layout.width, layout.height
                ),
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                    .to_owned(),
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
                    .to_owned(),
                format!("<< /Length {content_length} >>\nstream\n{content}\nendstream"),
            ],
        }
    }

    pub(super) fn finish(self) -> Vec<u8> {
        let mut pdf = String::from("%PDF-1.4\n%\u{00e2}\u{00e3}\u{00cf}\u{00d3}\n");
        let mut offsets = Vec::with_capacity(self.objects.len());

        for (index, object) in self.objects.iter().enumerate() {
            offsets.push(pdf.len());
            let _ = writeln!(pdf, "{} 0 obj\n{}\nendobj", index + 1, object);
        }

        let xref_position = pdf.len();
        let _ = writeln!(pdf, "xref\n0 {}", self.objects.len() + 1);
        pdf.push_str("0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(pdf, "{offset:010} 00000 n ");
        }
        let _ = write!(
            pdf,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_position}\n%%EOF\n",
            self.objects.len() + 1
        );

        pdf.into_bytes()
    }
}
