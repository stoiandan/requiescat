use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraveGps {
    latitude: DmsCoordinate,
    longitude: DmsCoordinate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DmsCoordinate {
    degrees: u16,
    minutes: u8,
    seconds: f32,
    direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveGpsParseError {
    Format,
    Latitude,
    Longitude,
}

impl GraveGps {
    pub fn parse(value: &str) -> Result<Self, GraveGpsParseError> {
        let (latitude, longitude) = value.split_once(',').ok_or(GraveGpsParseError::Format)?;
        Self::parse_parts(latitude, longitude)
    }

    pub fn parse_parts(latitude: &str, longitude: &str) -> Result<Self, GraveGpsParseError> {
        let latitude = DmsCoordinate::parse(latitude, CoordinateAxis::Latitude)
            .map_err(|_| GraveGpsParseError::Latitude)?;
        let longitude = DmsCoordinate::parse(longitude, CoordinateAxis::Longitude)
            .map_err(|_| GraveGpsParseError::Longitude)?;

        Ok(Self {
            latitude,
            longitude,
        })
    }

    pub fn latitude_text(&self) -> String {
        self.latitude.to_string()
    }

    pub fn longitude_text(&self) -> String {
        self.longitude.to_string()
    }
}

impl fmt::Display for GraveGps {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}, {}", self.latitude, self.longitude)
    }
}

#[derive(Debug, Clone, Copy)]
enum CoordinateAxis {
    Latitude,
    Longitude,
}

impl CoordinateAxis {
    fn max_degrees(self) -> u16 {
        match self {
            Self::Latitude => 90,
            Self::Longitude => 180,
        }
    }

    fn accepts(self, direction: Direction) -> bool {
        matches!(
            (self, direction),
            (Self::Latitude, Direction::North | Direction::South)
                | (Self::Longitude, Direction::East | Direction::West)
        )
    }
}

impl DmsCoordinate {
    fn parse(value: &str, axis: CoordinateAxis) -> Result<Self, GraveGpsParseError> {
        let tokens = coordinate_tokens(value);
        if tokens.len() != 4 {
            return Err(GraveGpsParseError::Format);
        }

        let degrees = tokens[0]
            .parse::<u16>()
            .map_err(|_| GraveGpsParseError::Format)?;
        let minutes = tokens[1]
            .parse::<u8>()
            .map_err(|_| GraveGpsParseError::Format)?;
        let seconds = tokens[2]
            .parse::<f32>()
            .map_err(|_| GraveGpsParseError::Format)?;
        let direction = Direction::parse(&tokens[3]).ok_or(GraveGpsParseError::Format)?;

        if !axis.accepts(direction)
            || degrees > axis.max_degrees()
            || minutes >= 60
            || !(0.0..60.0).contains(&seconds)
            || (degrees == axis.max_degrees() && (minutes != 0 || seconds != 0.0))
        {
            return Err(GraveGpsParseError::Format);
        }

        Ok(Self {
            degrees,
            minutes,
            seconds,
            direction,
        })
    }
}

impl fmt::Display for DmsCoordinate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}\u{00b0} {}\u{2032} {}\u{2033} {}",
            self.degrees,
            self.minutes,
            format_seconds(self.seconds),
            self.direction
        )
    }
}

impl Direction {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "N" => Some(Self::North),
            "S" => Some(Self::South),
            "E" => Some(Self::East),
            "W" => Some(Self::West),
            _ => None,
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::North => "N",
            Self::South => "S",
            Self::East => "E",
            Self::West => "W",
        })
    }
}

fn coordinate_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in value.chars() {
        if character.is_ascii_digit() || character == '.' || character.is_ascii_alphabetic() {
            current.push(character);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn format_seconds(seconds: f32) -> String {
    let formatted = format!("{seconds:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dms_coordinates() {
        let gps = GraveGps::parse("51° 30′ 26.64″ N, 0° 7′ 40.08″ W").unwrap();

        assert_eq!(gps.to_string(), "51° 30′ 26.64″ N, 0° 7′ 40.08″ W");
    }

    #[test]
    fn accepts_ascii_dms_separators() {
        let gps = GraveGps::parse("51 30 26.64 N, 0 7 40.08 W").unwrap();

        assert_eq!(gps.to_string(), "51° 30′ 26.64″ N, 0° 7′ 40.08″ W");
    }

    #[test]
    fn rejects_invalid_dms_coordinates() {
        assert_eq!(
            GraveGps::parse("91° 0′ 0″ N, 0° 0′ 0″ W"),
            Err(GraveGpsParseError::Latitude)
        );
        assert_eq!(
            GraveGps::parse("51° 60′ 0″ N, 0° 0′ 0″ W"),
            Err(GraveGpsParseError::Latitude)
        );
        assert_eq!(
            GraveGps::parse("51° 0′ 0″ E, 0° 0′ 0″ W"),
            Err(GraveGpsParseError::Latitude)
        );
        assert_eq!(
            GraveGps::parse("51° 0′ 0″ N"),
            Err(GraveGpsParseError::Format)
        );
    }

    #[test]
    fn allows_only_exact_axis_limits() {
        assert!(GraveGps::parse("90° 0′ 0″ N, 180° 0′ 0″ W").is_ok());
        assert_eq!(
            GraveGps::parse("90° 0′ 0.01″ N, 180° 0′ 0″ W"),
            Err(GraveGpsParseError::Latitude)
        );
        assert_eq!(
            GraveGps::parse("90° 0′ 0″ N, 180° 0′ 0.01″ W"),
            Err(GraveGpsParseError::Longitude)
        );
    }
}
