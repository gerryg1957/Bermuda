pub(crate) fn normalise_played_date(date: &str) -> String {
    let trimmed = date.trim();

    let approximate = trimmed.strip_prefix("c.").map(str::trim);

    if let Some(year) = approximate
        && is_four_digit_year(year)
    {
        return format!("{year}-01-01");
    }

    if is_four_digit_year(trimmed) {
        return format!("{trimmed}-01-01");
    }

    if let Some((year, month)) = trimmed.split_once('-')
        && is_four_digit_year(year)
        && is_two_digit_month(month)
    {
        return format!("{year}-{month}-01");
    }

    trimmed.to_owned()
}

pub(crate) fn played_date_sort_key(date: &str) -> Option<String> {
    let trimmed = date.trim();

    let candidate = trimmed.strip_prefix("Published ").unwrap_or(trimmed);

    let year = candidate.get(0..4)?;

    if !is_four_digit_year(year) {
        return None;
    }

    if candidate.len() == 4 {
        return Some(format!("{year}-01-01"));
    }

    if candidate.as_bytes().get(4) != Some(&b'-') {
        return None;
    }

    let month = candidate.get(5..7)?;

    if !is_two_digit_month(month) {
        return None;
    }

    if candidate.as_bytes().get(7) != Some(&b'-') {
        return Some(format!("{year}-{month}-01"));
    }

    let day = candidate.get(8..10)?;

    if day.len() != 2
        || !day.bytes().all(|byte| byte.is_ascii_digit())
        || !matches!(day.parse::<u8>(), Ok(1..=31))
    {
        return None;
    }

    Some(format!("{year}-{month}-{day}"))
}

fn is_four_digit_year(value: &str) -> bool {
    value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_two_digit_month(value: &str) -> bool {
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }

    matches!(value.parse::<u8>(), Ok(1..=12))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_sort_key_from_iso_dates() {
        assert_eq!(played_date_sort_key("1683"), Some("1683-01-01".to_owned()));

        assert_eq!(
            played_date_sort_key("1683-07"),
            Some("1683-07-01".to_owned())
        );

        assert_eq!(
            played_date_sort_key("1683-07-12"),
            Some("1683-07-12".to_owned())
        );
    }

    #[test]
    fn derives_sort_key_from_publication_dates() {
        assert_eq!(
            played_date_sort_key("Published 1634"),
            Some("1634-01-01".to_owned())
        );

        assert_eq!(
            played_date_sort_key("Published 1947-07 in Kido"),
            Some("1947-07-01".to_owned())
        );

        assert_eq!(
            played_date_sort_key("Published 1903-11-13~26"),
            Some("1903-11-13".to_owned())
        );
    }

    #[test]
    fn uses_first_date_in_complex_publication_value() {
        assert_eq!(
            played_date_sort_key("Published 1936-09-21~10-04 in Tokyo and 09-24~10-07 in Osaka"),
            Some("1936-09-21".to_owned())
        );
    }

    #[test]
    fn rejects_unrecognised_dates() {
        assert_eq!(played_date_sort_key("Unknown"), None);
        assert_eq!(played_date_sort_key("Spring 1683"), None);
        assert_eq!(played_date_sort_key("Published unknown"), None);
    }
}
