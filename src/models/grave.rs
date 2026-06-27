use iced::{Point, Vector};

use super::{GraveGps, GraveId, GraveRectangle, Tags};

#[derive(Debug, Clone)]
pub struct Grave {
    id: GraveId,
    rectangle: GraveRectangle,
    color: GraveColor,
    rotation_degrees: f32,
    gps: Option<GraveGps>,
    tags: Tags,
}

impl Grave {
    pub fn with_color(self, color: GraveColor) -> Self {
        Self { color, ..self }
    }

    pub fn new(id: GraveId, rectangle: GraveRectangle, color: GraveColor) -> Self {
        Self {
            id,
            rectangle,
            color,
            rotation_degrees: 0.0,
            gps: None,
            tags: Tags::default(),
        }
    }

    pub fn from_parts(
        id: GraveId,
        rectangle: GraveRectangle,
        color: GraveColor,
        rotation_degrees: f32,
        gps: Option<GraveGps>,
        tags: Tags,
    ) -> Self {
        Self {
            id,
            rectangle,
            color,
            rotation_degrees,
            gps,
            tags,
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

    pub fn rotation_degrees(&self) -> f32 {
        self.rotation_degrees
    }

    pub fn gps(&self) -> Option<GraveGps> {
        self.gps
    }

    pub fn gps_text(&self) -> String {
        self.gps.map(|gps| gps.to_string()).unwrap_or_default()
    }

    pub fn with_gps(self, gps: Option<GraveGps>) -> Self {
        Self { gps, ..self }
    }

    pub fn tags(&self) -> &Tags {
        &self.tags
    }

    pub fn tags_text(&self) -> String {
        self.tags.as_text()
    }

    pub fn with_tags(self, tags: Tags) -> Self {
        Self { tags, ..self }
    }

    pub fn matches_tag_query(&self, query: &str) -> bool {
        let query = query.trim();

        query.is_empty() || self.tags.matches_query(query)
    }

    pub fn with_rotation(self, rotation_degrees: f32) -> Self {
        Self {
            rotation_degrees: normalize_rotation(rotation_degrees),
            ..self
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        self.rectangle
            .contains_rotated(point, self.rotation_degrees)
    }

    pub fn translated(self, delta: Vector) -> Self {
        Self {
            rectangle: self.rectangle.translated(delta),
            ..self
        }
    }
}

pub fn normalize_rotation(rotation_degrees: f32) -> f32 {
    rotation_degrees.rem_euclid(360.0)
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

    pub fn to_rgb8(self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
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
