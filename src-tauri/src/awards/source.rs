//! Award types and the runtime derivation of Wikipedia page titles.
//!
//! The app never ships a hardcoded list of ceremonies: it computes the ceremony
//! ordinal from the year and builds the page title, so new ceremonies are picked
//! up automatically on the next sync (no app update). Verified for the modern
//! window — Academy Awards 1st = 1929, Primetime Emmys 1st = 1949.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AwardType {
    Oscars,
    Emmys,
}

impl AwardType {
    pub fn as_str(self) -> &'static str {
        match self {
            AwardType::Oscars => "oscars",
            AwardType::Emmys => "emmys",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "oscars" => Some(AwardType::Oscars),
            "emmys" => Some(AwardType::Emmys),
            _ => None,
        }
    }

    /// Ceremony ordinal for a given ceremony year.
    pub fn edition_for_year(self, year: i32) -> i32 {
        match self {
            AwardType::Oscars => year - 1928,
            AwardType::Emmys => year - 1948,
        }
    }

    /// Ceremony year for a given ordinal (inverse of `edition_for_year`).
    pub fn year_for_edition(self, edition: i32) -> i32 {
        match self {
            AwardType::Oscars => edition + 1928,
            AwardType::Emmys => edition + 1948,
        }
    }

    /// The English Wikipedia page title for a given edition.
    pub fn page_title(self, edition: i32) -> String {
        let ord = ordinal(edition);
        match self {
            AwardType::Oscars => format!("{ord} Academy Awards"),
            AwardType::Emmys => format!("{ord} Primetime Emmy Awards"),
        }
    }
}

/// English ordinal string: 1 -> "1st", 2 -> "2nd", 3 -> "3rd", 11 -> "11th",
/// 23 -> "23rd", 97 -> "97th", 111 -> "111th".
pub fn ordinal(n: i32) -> String {
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(13), "13th");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(23), "23rd");
        assert_eq!(ordinal(97), "97th");
        assert_eq!(ordinal(111), "111th");
    }

    #[test]
    fn edition_mapping_matches_known_ceremonies() {
        assert_eq!(AwardType::Oscars.edition_for_year(2025), 97);
        assert_eq!(AwardType::Oscars.year_for_edition(97), 2025);
        assert_eq!(AwardType::Emmys.edition_for_year(2025), 77);
        assert_eq!(AwardType::Emmys.year_for_edition(77), 2025);
    }

    #[test]
    fn page_titles_match_wikipedia() {
        assert_eq!(AwardType::Oscars.page_title(97), "97th Academy Awards");
        assert_eq!(AwardType::Emmys.page_title(77), "77th Primetime Emmy Awards");
    }
}
