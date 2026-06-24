use std::fmt::Write as _;

use iced::Point;

#[derive(Default)]
pub(super) struct PdfContent {
    value: String,
}

impl PdfContent {
    pub(super) fn finish(self) -> String {
        self.value
    }

    pub(super) fn fill_color(&mut self, red: f32, green: f32, blue: f32) {
        let _ = writeln!(self.value, "{red:.3} {green:.3} {blue:.3} rg");
    }

    pub(super) fn stroke_color(&mut self, red: f32, green: f32, blue: f32) {
        let _ = writeln!(self.value, "{red:.3} {green:.3} {blue:.3} RG");
    }

    pub(super) fn line_width(&mut self, width: f32) {
        let _ = writeln!(self.value, "{width:.2} w");
    }

    pub(super) fn rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let _ = writeln!(self.value, "{x:.2} {y:.2} {width:.2} {height:.2} re");
    }

    pub(super) fn polygon(&mut self, points: &[Point]) {
        let Some((first, remaining)) = points.split_first() else {
            return;
        };

        let _ = writeln!(self.value, "{:.2} {:.2} m", first.x, first.y);
        for point in remaining {
            let _ = writeln!(self.value, "{:.2} {:.2} l", point.x, point.y);
        }
        self.value.push_str("h\n");
    }

    pub(super) fn fill(&mut self) {
        self.value.push_str("f\n");
    }

    pub(super) fn stroke(&mut self) {
        self.value.push_str("S\n");
    }

    pub(super) fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let _ = writeln!(self.value, "{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S");
    }

    pub(super) fn text(&mut self, font: &str, size: f32, x: f32, y: f32, value: &str) {
        let _ = writeln!(
            self.value,
            "BT {font} {size:.2} Tf {x:.2} {y:.2} Td ({}) Tj ET",
            escape_pdf_text(value)
        );
    }

    pub(super) fn text_centered(
        &mut self,
        font: &str,
        size: f32,
        center_x: f32,
        y: f32,
        value: &str,
    ) {
        let value = pdf_safe_text(value);
        let width = approximate_text_width(&value, size);
        self.text(font, size, center_x - width / 2.0, y, &value);
    }
}

fn approximate_text_width(value: &str, size: f32) -> f32 {
    value.chars().count() as f32 * size * 0.52
}

fn escape_pdf_text(value: &str) -> String {
    pdf_safe_text(value)
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

pub(super) fn pdf_safe_text(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\u{0103}' | '\u{00e2}' | '\u{00e1}' | '\u{00e0}' | '\u{00e4}' => 'a',
            '\u{0102}' | '\u{00c2}' | '\u{00c1}' | '\u{00c0}' | '\u{00c4}' => 'A',
            '\u{00e9}' | '\u{00e8}' | '\u{00eb}' => 'e',
            '\u{00c9}' | '\u{00c8}' | '\u{00cb}' => 'E',
            '\u{00ee}' | '\u{00ed}' | '\u{00ec}' | '\u{00ef}' => 'i',
            '\u{00ce}' | '\u{00cd}' | '\u{00cc}' | '\u{00cf}' => 'I',
            '\u{0219}' | '\u{015f}' => 's',
            '\u{0218}' | '\u{015e}' => 'S',
            '\u{021b}' | '\u{0163}' => 't',
            '\u{021a}' | '\u{0162}' => 'T',
            character if character.is_ascii() && !character.is_control() => character,
            _ => '?',
        })
        .collect()
}
