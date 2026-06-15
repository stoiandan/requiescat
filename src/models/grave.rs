use iced::{Point, Vector};

use super::{GraveId, GraveRectangle};

#[derive(Debug, Clone, Copy)]
pub struct Grave {
    id: GraveId,
    rectangle: GraveRectangle,
    color: GraveColor,
}

impl Grave {
    pub fn with_color(id: GraveId, rectangle: GraveRectangle, color: GraveColor) -> Self {
        Self {
            id,
            rectangle,
            color,
        }
    }

    pub fn id(&self) -> GraveId {
        self.id
    }

    pub fn rectangle(&self) -> GraveRectangle {
        self.rectangle
    }

    pub fn color(&self) -> GraveColor {
        self.color
    }

    pub fn contains(&self, point: Point) -> bool {
        self.rectangle.contains(point)
    }

    pub fn translate(&mut self, delta: Vector) {
        self.rectangle.translate(delta);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraveColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl GraveColor {
    pub const DEFAULT: Self = Self::from_rgb8(166, 31, 40);
    pub const PALETTE: [Self; 6] = [
        Self::from_rgb8(166, 31, 40),
        Self::from_rgb8(218, 128, 40),
        Self::from_rgb8(203, 176, 54),
        Self::from_rgb8(71, 141, 86),
        Self::from_rgb8(50, 123, 171),
        Self::from_rgb8(122, 77, 161),
    ];

    pub const fn from_rgb8(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub fn to_iced(self) -> iced::Color {
        iced::Color::from_rgb8(self.red, self.green, self.blue)
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let value = value.strip_prefix('#').unwrap_or(value);
        if value.len() != 6 {
            return None;
        }

        let red = u8::from_str_radix(&value[0..2], 16).ok()?;
        let green = u8::from_str_radix(&value[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
        Some(Self::from_rgb8(red, green, blue))
    }
}

impl Default for GraveColor {
    fn default() -> Self {
        Self::DEFAULT
    }
}
