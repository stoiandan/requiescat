const APPROX_CHARACTER_WIDTH_RATIO: f32 = 0.52;

pub(crate) fn character_capacity(available_width: f32, font_size: f32) -> usize {
    (available_width / (font_size * APPROX_CHARACTER_WIDTH_RATIO))
        .floor()
        .max(0.0) as usize
}

pub(crate) fn visible_rows(rows: &[String], fallback: &str, max_rows: usize) -> Vec<String> {
    if max_rows == 0 {
        return Vec::new();
    }

    if rows.is_empty() {
        return vec![fallback.to_owned()];
    }

    if rows.len() <= max_rows {
        return rows.to_vec();
    }

    if max_rows == 1 {
        return vec!["...".to_owned()];
    }

    rows.iter()
        .take(max_rows - 1)
        .cloned()
        .chain(std::iter::once("...".to_owned()))
        .collect()
}

pub(crate) fn truncate(value: &str, max_characters: usize) -> String {
    if value.chars().count() <= max_characters {
        return value.to_owned();
    }

    if max_characters <= 3 {
        return String::new();
    }

    let mut truncated = value.chars().take(max_characters - 3).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn visible_rows_uses_fallback_when_grave_has_no_people() {
        assert_eq!(visible_rows(&Vec::new(), "grave 1", 2), rows(&["grave 1"]));
    }

    #[test]
    fn visible_rows_keeps_all_people_that_fit() {
        assert_eq!(
            visible_rows(&rows(&["Dan Stoian", "Maria Boto"]), "grave 1", 2),
            rows(&["Dan Stoian", "Maria Boto"])
        );
    }

    #[test]
    fn visible_rows_reserves_last_row_for_overflow_marker() {
        assert_eq!(
            visible_rows(
                &rows(&["Dan Stoian", "Maria Boto", "Ada Lovelace"]),
                "grave 1",
                2
            ),
            rows(&["Dan Stoian", "..."])
        );
    }

    #[test]
    fn visible_rows_returns_nothing_when_no_rows_fit() {
        assert!(visible_rows(&rows(&["Ada"]), "grave 1", 0).is_empty());
    }

    #[test]
    fn character_capacity_expands_with_available_width() {
        assert_eq!(character_capacity(46.8, 9.0), 10);
        assert_eq!(character_capacity(93.6, 9.0), 20);
    }

    #[test]
    fn truncate_uses_character_capacity() {
        assert_eq!(truncate("Dan Stoian", 20), "Dan Stoian");
        assert_eq!(truncate("Dan Stoian", 6), "Dan...");
    }
}
