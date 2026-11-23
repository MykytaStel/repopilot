use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::{Offset, TimeZone};

/// UTC timestamp at whole-second precision for baselines and snapshots.
pub fn now_seconds() -> String {
    format_seconds(Timestamp::now())
}

pub(crate) fn now_receipt() -> String {
    format_receipt(Timestamp::now())
}

pub(crate) fn today() -> Date {
    Timestamp::now().to_zoned(TimeZone::UTC).date()
}

pub(crate) fn parse_date(input: &str) -> Result<Date, jiff::Error> {
    Date::strptime("%Y-%m-%d", input)
}

fn format_seconds(timestamp: Timestamp) -> String {
    format!("{timestamp:.0}")
}

fn format_receipt(timestamp: Timestamp) -> String {
    let nanos = timestamp.subsec_nanosecond();
    let precision = if nanos == 0 {
        0
    } else if nanos % 1_000_000 == 0 {
        3
    } else if nanos % 1_000 == 0 {
        6
    } else {
        9
    };
    format!("{:.precision$}", timestamp.display_with_offset(Offset::UTC))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v028_utc_formats_match_existing_records() {
        for (fraction, expected) in [
            ("", ""),
            (".1", ".100"),
            (".1234", ".123400"),
            (".12345678", ".123456780"),
        ] {
            let timestamp = format!("2024-02-29T23:59:59{fraction}Z").parse().unwrap();
            assert_eq!(format_seconds(timestamp), "2024-02-29T23:59:59Z");
            assert_eq!(
                format_receipt(timestamp),
                format!("2024-02-29T23:59:59{expected}+00:00")
            );
        }
    }

    #[test]
    fn v028_expiry_requires_a_valid_calendar_date() {
        assert_eq!(parse_date("2024-02-29").unwrap().to_string(), "2024-02-29");
        assert!(parse_date("2025-02-29").is_err());
        assert!(parse_date("2024-02-29T12:00:00").is_err());
    }
}
