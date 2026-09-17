use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use std::io::BufReader;

use super::models::RacingEvent;

/// Fetch an ICS calendar file from a URL
pub async fn fetch_ics(url: &str) -> Result<String, String> {
    let response = crate::http_client::client()
        .get(url)
        .send()
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
pub fn parse_ics(ics_text: &str, series_slug: &str) -> Result<Vec<RacingEvent>, String> {
    if !ics_text.trim_start().starts_with("BEGIN:VCALENDAR")
        || !ics_text.trim_end().ends_with("END:VCALENDAR")
    {
        return Err("The feed is not a complete iCalendar document; saved events were kept".into());
    }
    let reader = BufReader::new(ics_text.as_bytes());
    let parser = IcalParser::new(reader);

    let mut events = Vec::new();

    for calendar_result in parser {
        let calendar: IcalCalendar = match calendar_result {
            Ok(cal) => cal,
            Err(error) => return Err(format!("Invalid calendar: {error}")),
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
                None => return Err("Calendar event is missing SUMMARY".into()),
            };

            let start_raw = match dtstart {
                Some(s) => s,
                None => return Err("Calendar event is missing DTSTART".into()),
            };

            let uid_str = uid
                .filter(|value| !value.trim().is_empty())
                .ok_or("Calendar event is missing UID")?;

            // Parse the summary to extract event title and session name
            let (event_title, session_name) = parse_summary(&summary_str);

            // Convert ICS datetime to ISO 8601 UTC
            let start_time = event_time_to_utc(&start_raw, dtstart_tzid.as_deref())?;
            let end_time = dtend
                .map(|d| event_time_to_utc(&d, dtend_tzid.as_deref()))
                .transpose()?;

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

    if events.is_empty() {
        return Err("The feed contains no events; saved events were kept".into());
    }
    let mut uids = std::collections::HashSet::new();
    if events.iter().any(|event| !uids.insert(&event.uid)) {
        return Err("Duplicate event UID in feed".into());
    }
    Ok(events)
}

/// Parse a SUMMARY field into (event_title, session_name)
///
/// Handles various formats:
/// - "🇦🇺 Australian GP: Race" → ("Australian GP", "Race")
/// - "F1 Australian GP - Practice 1" → ("Australian GP", "Practice 1")
/// - "[MotoGP] FP1 - #ThaiGP" → ("#ThaiGP", "FP1")
/// - "WEC - Qatar 1812km, Free Practice 1*" → ("Qatar 1812km", "Free Practice 1")
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

    // Try motorsportradar.com format: "Series - Event, Session"
    // The series prefix is dropped; rfind on ", " keeps any embedded
    // " - " or "(...)" inside the session intact (e.g. "Qualifying - 1").
    // Trailing "*" marks unconfirmed schedules and is stripped.
    if let Some(dash_idx) = cleaned.find(" - ") {
        let after_dash = &cleaned[dash_idx + 3..];
        if let Some(comma_idx) = after_dash.rfind(", ") {
            let event = after_dash[..comma_idx].trim().to_string();
            let session = after_dash[comma_idx + 2..]
                .trim()
                .trim_end_matches('*')
                .trim()
                .to_string();
            if !event.is_empty() && !session.is_empty() {
                return (event, Some(session));
            }
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
        "fp1"
            | "fp2"
            | "fp3"
            | "practice 1"
            | "practice 2"
            | "practice 3"
            | "qualifying"
            | "q1"
            | "q2"
            | "race"
            | "sprint"
            | "sprint qualifying"
            | "sprint shootout"
            | "warm up"
            | "wup"
            | "pr"
            | "spr"
            | "rac"
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

    if !rest.chars().all(|c| c.is_ascii_digit() || c == ':') {
        return None;
    }
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

    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 60 + minutes))
}

/// Resolve UTC, numeric offsets, IANA zones, and floating local times to one UTC format.
fn event_time_to_utc(value: &str, tzid: Option<&str>) -> Result<String, String> {
    use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
    let raw = value.trim();
    if !raw.is_ascii() || !matches!(raw.len(), 8 | 15 | 16) {
        return Err("Invalid calendar date".into());
    }
    let naive = if raw.len() == 8 {
        NaiveDate::parse_from_str(raw, "%Y%m%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
    } else {
        NaiveDateTime::parse_from_str(raw.trim_end_matches('Z'), "%Y%m%dT%H%M%S").ok()
    }
    .ok_or_else(|| format!("Invalid calendar date: {raw}"))?;
    let utc = if raw.ends_with('Z') {
        Some(naive.and_utc())
    } else if let Some(zone) = tzid {
        let zone = zone.trim_matches('"');
        if let Some(minutes) = parse_utc_offset_minutes(zone) {
            chrono::FixedOffset::east_opt(minutes * 60)
                .and_then(|tz| tz.from_local_datetime(&naive).single())
                .map(|date| date.with_timezone(&Utc))
        } else {
            let tz: chrono_tz::Tz = zone
                .parse()
                .map_err(|_| format!("Unknown calendar timezone: {zone}"))?;
            tz.from_local_datetime(&naive)
                .single()
                .map(|date| date.with_timezone(&Utc))
        }
    } else {
        chrono::Local
            .from_local_datetime(&naive)
            .single()
            .map(|date| date.with_timezone(&Utc))
    }
    .ok_or("Ambiguous or nonexistent local calendar time; the feed must specify a UTC offset")?;
    Ok(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

#[cfg(test)]
fn ics_datetime_to_iso(value: &str, zone: Option<&str>) -> String {
    event_time_to_utc(value, zone).unwrap()
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
    fn test_parse_summary_motorsportradar_format() {
        let (title, session) = parse_summary("WEC - Qatar 1812km, Free Practice 1*");
        assert_eq!(title, "Qatar 1812km");
        assert_eq!(session, Some("Free Practice 1".to_string()));
    }

    #[test]
    fn test_parse_summary_motorsportradar_embedded_dash_in_session() {
        // Session can contain " - " (e.g. "Qualifying - 1"); rfind on ", "
        // ensures we split at the right place
        let (title, session) = parse_summary("Supercars - Sydney 500, Qualifying - 1");
        assert_eq!(title, "Sydney 500");
        assert_eq!(session, Some("Qualifying - 1".to_string()));
    }

    #[test]
    fn test_parse_summary_motorsportradar_parens_in_session() {
        let (title, session) = parse_summary("Super Formula - Suzuka 1, Qualifying (Race 1)");
        assert_eq!(title, "Suzuka 1");
        assert_eq!(session, Some("Qualifying (Race 1)".to_string()));
    }

    #[test]
    fn test_ics_datetime_full_utc() {
        assert_eq!(
            ics_datetime_to_iso("20260329T050000Z", None),
            "2026-03-29T05:00:00Z"
        );
    }

    #[test]
    fn test_ics_datetime_no_tz() {
        use chrono::TimeZone;
        let expected = chrono::Local
            .with_ymd_and_hms(2026, 3, 29, 5, 0, 0)
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            ics_datetime_to_iso("20260329T050000", None),
            expected.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        );
    }

    #[test]
    fn test_ics_datetime_date_only() {
        use chrono::TimeZone;
        let expected = chrono::Local
            .with_ymd_and_hms(2026, 3, 29, 0, 0, 0)
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            ics_datetime_to_iso("20260329", None),
            expected.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        );
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
    #[test]
    fn iana_zones_follow_dst_and_reject_ambiguous_wall_times() {
        assert_eq!(
            event_time_to_utc("20260117T120000", Some("Europe/Oslo")).unwrap(),
            "2026-01-17T11:00:00Z"
        );
        assert_eq!(
            event_time_to_utc("20260917T120000", Some("Europe/Oslo")).unwrap(),
            "2026-09-17T10:00:00Z"
        );
        assert!(event_time_to_utc("20260329T023000", Some("Europe/Oslo")).is_err());
        assert!(event_time_to_utc("20261025T023000", Some("Europe/Oslo")).is_err());
    }
}
