#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonDate {
    day: u8,
    month: u8,
    year: u16,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateParseError {
    Format,
    Day,
    Month,
}

impl PersonDate {
    pub fn parse(value: &str) -> Result<Self, DateParseError> {
        let value = value.trim();
        let mut parts = value.split('-');

        let Some(day) = parts.next() else {
            return Err(DateParseError::Format);
        };
        let Some(month) = parts.next() else {
            return Err(DateParseError::Format);
        };
        let Some(year) = parts.next() else {
            return Err(DateParseError::Format);
        };

        if parts.next().is_some() || day.len() != 2 || month.len() != 2 || year.len() != 4 {
            return Err(DateParseError::Format);
        }

        let day = day.parse::<u8>().map_err(|_| DateParseError::Format)?;
        let month = month.parse::<u8>().map_err(|_| DateParseError::Format)?;
        let year = year.parse::<u16>().map_err(|_| DateParseError::Format)?;

        if !(1..=12).contains(&month) {
            return Err(DateParseError::Month);
        }

        if day == 0 || day > days_in_month(month, year) {
            return Err(DateParseError::Day);
        }

        Ok(Self {
            day,
            month,
            year,
            text: value.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for PersonDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn days_in_month(month: u8, year: u16) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_day_month_year_dates() {
        let date = PersonDate::parse("30-04-1996").expect("date should parse");

        assert_eq!(date.day, 30);
        assert_eq!(date.month, 4);
        assert_eq!(date.year, 1996);
        assert_eq!(date.as_str(), "30-04-1996");
    }

    #[test]
    fn rejects_invalid_dates() {
        assert_eq!(PersonDate::parse("1996-04-30"), Err(DateParseError::Format));
        assert_eq!(PersonDate::parse("31-04-1996"), Err(DateParseError::Day));
        assert_eq!(PersonDate::parse("30-13-1996"), Err(DateParseError::Month));
    }

    #[test]
    fn validates_leap_years() {
        assert!(PersonDate::parse("29-02-2000").is_ok());
        assert!(PersonDate::parse("29-02-1900").is_err());
        assert!(PersonDate::parse("29-02-1996").is_ok());
    }
}
