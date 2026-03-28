use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use std::io::BufReader;

use super::models::RacingEvent;

/// Fetch an ICS calendar file from a URL
pub async fn fetch_ics(url: &str) -> Result<String, String> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch ICS from {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "ICS fetch returned status {} for {}",
            response.status(),
            url
        ));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read ICS response: {}", e))
}

/// Parse an ICS calendar string into racing events
pub fn parse_ics(ics_text: &str, series_slug: &str) -> Vec<RacingEvent> {
    let reader = BufReader::new(ics_text.as_bytes());
    let parser = IcalParser::new(reader);

    let mut events = Vec::new();

    for calendar_result in parser {
        let calendar: IcalCalendar = match calendar_result {
            Ok(cal) => cal,
            Err(_) => continue,
        };

        for vevent in calendar.events {
            let mut summary = None;
            let mut dtstart = None;
            let mut dtstart_tzid = None;
            let mut dtend = None;
            let mut dtend_tzid = None;
            let mut location = None;
            let mut uid = None;
            let mut description = None;

            for prop in &vevent.properties {
                match prop.name.as_str() {
                    "SUMMARY" => summary = prop.value.clone(),
                    "DTSTART" => {
                        dtstart = prop.value.clone();
                        dtstart_tzid = extract_tzid(&prop.params);
                    }
                    "DTEND" => {
                        dtend = prop.value.clone();
                        dtend_tzid = extract_tzid(&prop.params);
                    }
                    "LOCATION" => location = prop.value.clone(),
                    "UID" => uid = prop.value.clone(),
                    "DESCRIPTION" => description = prop.value.clone(),
                    _ => {}
                }
            }

            let summary_str = match summary {
                Some(s) => s,
                None => continue,
            };

            let start_raw = match dtstart {
                Some(s) => s,
                None => continue,
            };

            let uid_str = uid.unwrap_or_else(|| format!("{}_{}", series_slug, start_raw));

            // Parse the summary to extract event title and session name
            let (event_title, session_name) = parse_summary(&summary_str);

            // Convert ICS datetime to ISO 8601 UTC
            let start_time = ics_datetime_to_iso(&start_raw, dtstart_tzid.as_deref());
            let end_time = dtend.map(|d| ics_datetime_to_iso(&d, dtend_tzid.as_deref()));

            events.push(RacingEvent {
                id: 0, // Will be set by database
                series_slug: series_slug.to_string(),
                uid: uid_str,
                event_title,
                session_name,
                circuit: location,
                start_time,
                end_time,
                description,
                notified: false,
            });
        }
    }

    events
}

/// Parse a SUMMARY field into (event_title, session_name)
///
/// Handles various formats:
/// - "🇦🇺 Australian GP: Race" → ("Australian GP", "Race")
/// - "F1 Australian GP - Practice 1" → ("Australian GP", "Practice 1")
/// - "[MotoGP] FP1 - #ThaiGP" → ("#ThaiGP", "FP1")
/// - "Coca-Cola 600" → ("Coca-Cola 600", None)
fn parse_summary(summary: &str) -> (String, Option<String>) {
    // Strip emoji flags (country flags are two regional indicator chars)
    let cleaned = strip_emoji_flags(summary).trim().to_string();

    // Try "Title: Session" format (f1.vidmar.net style)
    if let Some(idx) = cleaned.find(": ") {
        let title = cleaned[..idx].trim().to_string();
        let session = cleaned[idx + 2..].trim().to_string();
        if !session.is_empty() {
            return (title, Some(session));
        }
    }

    // Try "Title - Session" format (better-f1-calendar style)
    if let Some(idx) = cleaned.find(" - ") {
        let left = cleaned[..idx].trim().to_string();
        let right = cleaned[idx + 3..].trim().to_string();

        // Check if left looks like a session name (for "[MotoGP] FP1 - #ThaiGP" style)
        if is_session_name(&left) || left.starts_with('[') {
            // Extract session from left, title from right
            let session = left
                .trim_start_matches(|c: char| c == '[')
                .split(']')
                .last()
                .unwrap_or(&left)
                .trim()
                .to_string();
            return (right, Some(session));
        }

        // Standard "Title - Session" format
        if !right.is_empty() {
            return (left, Some(right));
        }
    }

    // No separator found — treat whole thing as event title
    (cleaned, None)
}

/// Check if a string looks like a session name
fn is_session_name(s: &str) -> bool {
    let lower = s.to_lowercase();
    let stripped = lower
        .trim_start_matches(|c: char| c == '[')
        .split(']')
        .last()
        .unwrap_or(&lower)
        .trim();

    matches!(
        stripped,
        "fp1" | "fp2" | "fp3" | "practice 1" | "practice 2" | "practice 3"
            | "qualifying" | "q1" | "q2" | "race" | "sprint"
            | "sprint qualifying" | "sprint shootout"
            | "warm up" | "wup" | "pr" | "spr" | "rac"
    )
}

/// Strip emoji country flags from a string
fn strip_emoji_flags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        // Regional indicator symbols are U+1F1E6 to U+1F1FF
        if ('\u{1F1E6}'..='\u{1F1FF}').contains(&c) {
            // Skip the next char too if it's also a regional indicator (flag pair)
            if let Some(&next) = chars.peek() {
                if ('\u{1F1E6}'..='\u{1F1FF}').contains(&next) {
                    chars.next();
                    continue;
                }
            }
            continue;
        }
        result.push(c);
    }

    result
}

/// Extract TZID parameter from ICS property params
fn extract_tzid(params: &Option<Vec<(String, Vec<String>)>>) -> Option<String> {
    params.as_ref().and_then(|p| {
        p.iter()
            .find(|(k, _)| k == "TZID")
            .and_then(|(_, v)| v.first().cloned())
    })
}

/// Parse a UTC offset string like "UTC+0700", "UTC-0300", "+05:30", "-03" into total minutes
fn parse_utc_offset_minutes(tzid: &str) -> Option<i32> {
    // Strip "UTC" prefix if present
    let offset_str = tzid
        .strip_prefix("UTC")
        .or_else(|| tzid.strip_prefix("utc"))
        .unwrap_or(tzid);

    if offset_str.is_empty() {
        return Some(0);
    }

    let (sign, rest) = if let Some(r) = offset_str.strip_prefix('+') {
        (1, r)
    } else if let Some(r) = offset_str.strip_prefix('-') {
        (-1, r)
    } else {
        return None;
    };

    // Remove colons: "05:30" → "0530"
    let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();

    let (hours, minutes) = match digits.len() {
        1 | 2 => (digits.parse::<i32>().ok()?, 0),
        3 | 4 => {
            let h = digits[..digits.len() - 2].parse::<i32>().ok()?;
            let m = digits[digits.len() - 2..].parse::<i32>().ok()?;
            (h, m)
        }
        _ => return None,
    };

    Some(sign * (hours * 60 + minutes))
}

/// Convert ICS datetime format to ISO 8601, always normalizing to UTC
///
/// Handles:
/// - "20260329T050000Z" → "2026-03-29T05:00:00Z"
/// - "20260329T050000" with TZID "UTC+0700" → converted to UTC with Z suffix
/// - "20260329T050000" without TZID → "2026-03-29T05:00:00" (kept as-is)
/// - "20260329" → "2026-03-29"
fn ics_datetime_to_iso(dt: &str, tzid: Option<&str>) -> String {
    let clean = dt.trim();

    // Full datetime: 20260329T050000Z or 20260329T050000
    if clean.len() >= 15 && clean.contains('T') {
        let date_part = &clean[..8];
        let time_part = &clean[9..15];

        // Already UTC
        if clean.ends_with('Z') {
            return format!(
                "{}-{}-{}T{}:{}:{}Z",
                &date_part[..4],
                &date_part[4..6],
                &date_part[6..8],
                &time_part[..2],
                &time_part[2..4],
                &time_part[4..6],
            );
        }

        // Has TZID — convert to UTC
        if let Some(tz) = tzid {
            if let Some(offset_minutes) = parse_utc_offset_minutes(tz) {
                return apply_utc_offset(date_part, time_part, offset_minutes);
            }
        }

        // No timezone info — keep as-is (floating time)
        return format!(
            "{}-{}-{}T{}:{}:{}",
            &date_part[..4],
            &date_part[4..6],
            &date_part[6..8],
            &time_part[..2],
            &time_part[2..4],
            &time_part[4..6],
        );
    }

    // Date only: 20260329
    if clean.len() == 8 && clean.chars().all(|c| c.is_ascii_digit()) {
        return format!("{}-{}-{}", &clean[..4], &clean[4..6], &clean[6..8]);
    }

    // Fallback: return as-is
    clean.to_string()
}

/// Apply a UTC offset (in minutes) to a date+time and return an ISO 8601 UTC string
fn apply_utc_offset(date_part: &str, time_part: &str, offset_minutes: i32) -> String {
    let year: i32 = date_part[..4].parse().unwrap_or(2026);
    let month: u32 = date_part[4..6].parse().unwrap_or(1);
    let day: u32 = date_part[6..8].parse().unwrap_or(1);
    let hour: i32 = time_part[..2].parse().unwrap_or(0);
    let min: i32 = time_part[2..4].parse().unwrap_or(0);
    let sec: i32 = time_part[4..6].parse().unwrap_or(0);

    // Convert to total minutes from midnight, subtract offset to get UTC
    let total_minutes = hour * 60 + min - offset_minutes;
    let mut utc_day = day as i32;
    let mut utc_hour = total_minutes / 60;
    let mut utc_min = total_minutes % 60;

    // Handle negative minutes
    if utc_min < 0 {
        utc_min += 60;
        utc_hour -= 1;
    }

    // Handle day rollover
    if utc_hour < 0 {
        utc_hour += 24;
        utc_day -= 1;
    } else if utc_hour >= 24 {
        utc_hour -= 24;
        utc_day += 1;
    }

    // Handle month boundaries
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    };

    let (final_year, final_month, final_day) = if utc_day < 1 {
        // Rolled back to previous month
        let prev_month = if month == 1 { 12 } else { month - 1 };
        let prev_year = if month == 1 { year - 1 } else { year };
        let prev_days = match prev_month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (prev_year % 4 == 0 && prev_year % 100 != 0) || prev_year % 400 == 0 {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        };
        (prev_year, prev_month, prev_days as i32 + utc_day)
    } else if utc_day > days_in_month as i32 {
        // Rolled forward to next month
        let next_month = if month == 12 { 1 } else { month + 1 };
        let next_year = if month == 12 { year + 1 } else { year };
        (next_year, next_month, utc_day - days_in_month as i32)
    } else {
        (year, month, utc_day)
    };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        final_year, final_month, final_day, utc_hour, utc_min, sec
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summary_colon_format() {
        let (title, session) = parse_summary("Australian GP: Race");
        assert_eq!(title, "Australian GP");
        assert_eq!(session, Some("Race".to_string()));
    }

    #[test]
    fn test_parse_summary_dash_format() {
        let (title, session) = parse_summary("F1 Australian GP - Practice 1");
        assert_eq!(title, "F1 Australian GP");
        assert_eq!(session, Some("Practice 1".to_string()));
    }

    #[test]
    fn test_parse_summary_with_emoji_flag() {
        let (title, session) = parse_summary("🇦🇺 Australian GP: Race");
        assert_eq!(title, "Australian GP");
        assert_eq!(session, Some("Race".to_string()));
    }

    #[test]
    fn test_parse_summary_plain() {
        let (title, session) = parse_summary("Coca-Cola 600");
        assert_eq!(title, "Coca-Cola 600");
        assert_eq!(session, None);
    }

    #[test]
    fn test_ics_datetime_full_utc() {
        assert_eq!(ics_datetime_to_iso("20260329T050000Z", None), "2026-03-29T05:00:00Z");
    }

    #[test]
    fn test_ics_datetime_no_tz() {
        assert_eq!(ics_datetime_to_iso("20260329T050000", None), "2026-03-29T05:00:00");
    }

    #[test]
    fn test_ics_datetime_date_only() {
        assert_eq!(ics_datetime_to_iso("20260329", None), "2026-03-29");
    }

    #[test]
    fn test_ics_datetime_with_positive_tzid() {
        // TZID=UTC+0700, 10:45 local → 03:45 UTC
        assert_eq!(
            ics_datetime_to_iso("20260227T104500", Some("UTC+0700")),
            "2026-02-27T03:45:00Z"
        );
    }

    #[test]
    fn test_ics_datetime_with_negative_tzid() {
        // TZID=UTC-0300, 12:05 local → 15:05 UTC
        assert_eq!(
            ics_datetime_to_iso("20260320T120500", Some("UTC-0300")),
            "2026-03-20T15:05:00Z"
        );
    }

    #[test]
    fn test_ics_datetime_tzid_day_rollback() {
        // TZID=UTC+0900, 02:00 local → 17:00 UTC previous day
        assert_eq!(
            ics_datetime_to_iso("20260315T020000", Some("UTC+0900")),
            "2026-03-14T17:00:00Z"
        );
    }

    #[test]
    fn test_ics_datetime_tzid_day_rollforward() {
        // TZID=UTC-0500, 22:00 local → 03:00 UTC next day
        assert_eq!(
            ics_datetime_to_iso("20260315T220000", Some("UTC-0500")),
            "2026-03-16T03:00:00Z"
        );
    }

    #[test]
    fn test_parse_utc_offset() {
        assert_eq!(parse_utc_offset_minutes("UTC+0700"), Some(420));
        assert_eq!(parse_utc_offset_minutes("UTC-0300"), Some(-180));
        assert_eq!(parse_utc_offset_minutes("UTC+0000"), Some(0));
        assert_eq!(parse_utc_offset_minutes("UTC"), Some(0));
        assert_eq!(parse_utc_offset_minutes("+05:30"), Some(330));
    }
}
