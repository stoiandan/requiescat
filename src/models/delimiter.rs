use iced::Point;

use super::{DelimiterId, GraveColor, GraveRectangle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterType {
    Wall,
    Road,
}

impl DelimiterType {
    pub const DEFAULT: Self = Self::Wall;

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wall => "wall",
            Self::Road => "road",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "wall" => Some(Self::Wall),
            "road" => Some(Self::Road),
            _ => None,
        }
    }
}

impl Default for DelimiterType {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Delimiter {
    id: DelimiterId,
    rectangle: GraveRectangle,
    color: GraveColor,
    delimiter_type: DelimiterType,
}

impl Delimiter {
    pub fn with_color_and_type(
        id: DelimiterId,
        rectangle: GraveRectangle,
        color: GraveColor,
        delimiter_type: DelimiterType,
    ) -> Self {
        Self {
            id,
            rectangle,
            color,
            delimiter_type,
        }
    }

    pub fn id(&self) -> DelimiterId {
        self.id
    }

    pub fn rectangle(&self) -> GraveRectangle {
        self.rectangle
    }

    pub fn color(&self) -> GraveColor {
        self.color
    }

    pub fn delimiter_type(&self) -> DelimiterType {
        self.delimiter_type
    }

    pub fn contains(&self, point: Point) -> bool {
        self.rectangle.contains(point)
    }
}
