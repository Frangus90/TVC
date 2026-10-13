use super::{MovieReleaseDates, ReleaseDatesResponse};
use chrono::NaiveDate;

pub(super) fn select_movie_release_dates(
    release_data: &ReleaseDatesResponse,
    country: &str,
) -> MovieReleaseDates {
    // TMDB can list re-releases before the original, including within one type.
    let earliest = |types: &[i32], preferred_country_only: bool| {
        release_data
            .results
            .iter()
            .filter(|region| !preferred_country_only || region.iso_3166_1 == country)
            .flat_map(|region| region.release_dates.iter())
            .filter(|release| types.contains(&release.release_type))
            .filter_map(|release| {
                // Release dates represent calendar days, not local-time instants.
                let date = release.release_date.split('T').next()?;
                NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
            })
            .min()
            .map(|date| date.to_string())
    };

    MovieReleaseDates {
        // Original public release/year stays stable when the preferred country changes.
        theatrical: earliest(&[2, 3], false).or_else(|| earliest(&[1], false)),
        digital: earliest(&[4], true),
        physical: earliest(&[5], true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmdb::{CountryReleaseDates, ReleaseDate};

    fn releases(entries: &[(i32, &str)]) -> ReleaseDatesResponse {
        ReleaseDatesResponse {
            id: 120,
            results: vec![CountryReleaseDates {
                iso_3166_1: "US".into(),
                release_dates: entries
                    .iter()
                    .map(|(release_type, date)| ReleaseDate {
                        certification: None,
                        iso_639_1: None,
                        note: None,
                        release_date: (*date).into(),
                        release_type: *release_type,
                    })
                    .collect(),
            }],
        }
    }

    // Relevant entries in TMDB's response order, checked on 10.10.2026.
    #[test]
    fn fellowship_uses_original_theatrical_and_physical_releases() {
        let data = releases(&[
            (2, "2026-01-16T00:00:00Z"),
            (3, "2001-12-19T00:00:00Z"),
            (5, "2002-11-12T00:00:00Z"),
            (5, "2002-08-06T00:00:00Z"),
        ]);
        let dates = select_movie_release_dates(&data, "US");
        assert_eq!(dates.theatrical.as_deref(), Some("2001-12-19"));
        assert_eq!(dates.physical.as_deref(), Some("2002-08-06"));
        assert!(dates.digital.is_none());
    }

    #[test]
    fn hellraiser_uses_original_theatrical_release() {
        let data = releases(&[
            (2, "2025-02-05T00:00:00Z"),
            (3, "1987-09-18T00:00:00Z"),
            (5, "1988-01-01T00:00:00Z"),
            (5, "2016-12-20T00:00:00Z"),
        ]);
        let dates = select_movie_release_dates(&data, "US");
        assert_eq!(dates.theatrical.as_deref(), Some("1987-09-18"));
        assert_eq!(dates.physical.as_deref(), Some("1988-01-01"));
    }

    #[test]
    fn last_crusade_uses_original_theatrical_release() {
        let data = releases(&[
            (2, "2025-01-10T00:00:00Z"),
            (3, "1989-05-24T00:00:00Z"),
            (3, "2025-06-14T00:00:00Z"),
        ]);
        let dates = select_movie_release_dates(&data, "US");
        assert_eq!(dates.theatrical.as_deref(), Some("1989-05-24"));
        assert!(dates.digital.is_none());
        assert!(dates.physical.is_none());
    }

    #[test]
    fn release_order_does_not_change_any_date() {
        let mut data = releases(&[
            (3, "2026-01-16T00:00:00Z"),
            (3, "2001-12-19T00:00:00Z"),
            (4, "2025-01-01T00:00:00Z"),
            (4, "2010-04-06T00:00:00Z"),
            (5, "2020-12-01T00:00:00Z"),
            (5, "2002-08-06T00:00:00Z"),
        ]);
        for _ in 0..2 {
            for _ in 0..data.results[0].release_dates.len() {
                let dates = select_movie_release_dates(&data, "US");
                assert_eq!(dates.theatrical.as_deref(), Some("2001-12-19"));
                assert_eq!(dates.digital.as_deref(), Some("2010-04-06"));
                assert_eq!(dates.physical.as_deref(), Some("2002-08-06"));
                data.results[0].release_dates.rotate_left(1);
            }
            data.results[0].release_dates.reverse();
        }
    }

    #[test]
    fn earliest_public_release_beats_premiere_and_may_be_in_the_future() {
        let data = releases(&[
            (1, "2099-01-01T00:00:00Z"),
            (3, "2099-04-01T00:00:00Z"),
            (2, "2099-03-01T00:00:00Z"),
        ]);
        assert_eq!(
            select_movie_release_dates(&data, "US")
                .theatrical
                .as_deref(),
            Some("2099-03-01")
        );
    }

    #[test]
    fn premiere_is_a_fallback_only_when_no_valid_theatrical_date_exists() {
        let data = releases(&[
            (1, "2026-02-01T00:00:00Z"),
            (1, "2026-01-01T00:00:00Z"),
            (3, ""),
            (4, "2025-12-01T00:00:00Z"),
            (6, "2025-11-01T00:00:00Z"),
        ]);
        assert_eq!(
            select_movie_release_dates(&data, "US")
                .theatrical
                .as_deref(),
            Some("2026-01-01")
        );
    }

    #[test]
    fn invalid_dates_are_skipped_and_calendar_dates_are_not_timezone_shifted() {
        let data = releases(&[
            (3, ""),
            (3, "not a date"),
            (3, "2025-02-29T00:00:00Z"),
            (3, "2024-02-29T00:00:00+14:00"),
            (4, "2024-13-01"),
            (4, "2024-04-01"),
            (5, "2024-02-30T00:00:00Z"),
        ]);
        let dates = select_movie_release_dates(&data, "US");
        assert_eq!(dates.theatrical.as_deref(), Some("2024-02-29"));
        assert_eq!(dates.digital.as_deref(), Some("2024-04-01"));
        assert!(dates.physical.is_none());
    }

    #[test]
    fn missing_country_keeps_original_theatrical_but_does_not_borrow_digital_dates() {
        let mut data = releases(&[(3, "1987-09-11T00:00:00Z")]);
        data.results[0].iso_3166_1 = "GB".into();
        data.results.push(CountryReleaseDates {
            iso_3166_1: "US".into(),
            release_dates: vec![],
        });
        for country in ["US", "NO"] {
            let dates = select_movie_release_dates(&data, country);
            assert_eq!(dates.theatrical.as_deref(), Some("1987-09-11"));
            assert!(dates.digital.is_none());
            assert!(dates.physical.is_none());
        }
    }

    #[test]
    fn country_changes_regional_dates_without_replacing_original_year_with_a_rerelease() {
        let mut data = releases(&[(3, "1990-05-01"), (4, "2010-01-01"), (5, "1999-02-01")]);
        let mut norway = releases(&[(3, "2026-01-16"), (4, "2026-04-01"), (5, "2026-06-01")]);
        norway.results[0].iso_3166_1 = "NO".into();
        data.results.extend(norway.results);
        let mut original = releases(&[(1, "1988-12-01"), (3, "1989-01-01")]);
        original.results[0].iso_3166_1 = "JP".into();
        data.results.extend(original.results);
        let us = select_movie_release_dates(&data, "US");
        let no = select_movie_release_dates(&data, "NO");
        assert_eq!(us.theatrical.as_deref(), Some("1989-01-01"));
        assert_eq!(no.theatrical, us.theatrical);
        assert_eq!(us.digital.as_deref(), Some("2010-01-01"));
        assert_eq!(no.digital.as_deref(), Some("2026-04-01"));
        assert_eq!(no.physical.as_deref(), Some("2026-06-01"));
    }
}
