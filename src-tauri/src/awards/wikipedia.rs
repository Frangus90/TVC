//! Fetch award ceremony wikitext from the MediaWiki API and parse it into a
//! `ParsedCeremony`. The parsing algorithm was validated against 5 real pages
//! (see the `tests` module + `fixtures/`) before being ported here.
//!
//! Parsing keys off Wikipedia's `{{Award category|…|[[link|Name]]}}` template
//! (identical on Oscar and Emmy pages — headings vary, the template does not) and
//! the winner marker (`‡` on Oscars, `{{double dagger}}` on Emmys).

use crate::awards::models::{ParsedCategory, ParsedCeremony, ParsedNominee};
use chrono::NaiveDate;
use regex::Regex;
use std::sync::OnceLock;

const API: &str = "https://en.wikipedia.org/w/api.php";
const USER_AGENT: &str = "TVC/0.13 (+https://github.com/Frangus90/TVC) awards-sync";

pub struct WikipediaAwardSource {
    client: reqwest::Client,
}

impl WikipediaAwardSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch raw wikitext for a page title. Returns `Ok(None)` when the page does
    /// not exist yet (e.g. a probed future ceremony) so the caller can skip it
    /// while other ceremonies still succeed.
    pub async fn fetch_wikitext(&self, title: &str) -> Result<Option<String>, String> {
        let resp = self
            .client
            .get(API)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .query(&[
                ("action", "parse"),
                ("page", title),
                ("prop", "wikitext"),
                ("format", "json"),
                ("formatversion", "2"),
                ("redirects", "1"),
            ])
            .send()
            .await
            .map_err(|e| format!("Wikipedia request failed for '{title}': {e}"))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid JSON from Wikipedia for '{title}': {e}"))?;

        if let Some(err) = json.get("error") {
            let code = err.get("code").and_then(|c| c.as_str()).unwrap_or("");
            // A not-yet-created ceremony page: skippable, not an error.
            if code == "missingtitle" {
                return Ok(None);
            }
            return Err(format!("Wikipedia API error for '{title}': {err}"));
        }

        let wikitext = json
            .pointer("/parse/wikitext")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Missing parse.wikitext in response for '{title}'"))?;

        Ok(Some(wikitext.to_string()))
    }
}

impl Default for WikipediaAwardSource {
    fn default() -> Self {
        Self::new()
    }
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static award-parser regex should compile")
}

/// Slice out the "Winners and nominees" section: from its `==` heading to the next
/// `==` H2 heading (or end of page).
fn winner_section(text: &str) -> Option<&str> {
    static START: OnceLock<Regex> = OnceLock::new();
    static NEXT: OnceLock<Regex> = OnceLock::new();
    let start = START.get_or_init(|| re(r"(?m)^==\s*Winners and nominees\s*==\s*$"));
    let next = NEXT.get_or_init(|| re(r"(?m)^==[^=].*==\s*$"));

    let m = start.find(text)?;
    let tail = &text[m.end()..];
    let end = next.find(tail).map(|mm| mm.start()).unwrap_or(tail.len());
    Some(&tail[..end])
}

/// Ceremony date (ISO `YYYY-MM-DD`) from the page's "Key dates" table, if present.
/// The table lists (Date, Event) rows; the ceremony is the row whose event is the
/// ceremony/telecast. Returns `None` for pages without a Key-dates section (most
/// past ceremonies) or an unparseable date.
fn parse_ceremony_date(text: &str) -> Option<String> {
    static START: OnceLock<Regex> = OnceLock::new();
    static NEXT: OnceLock<Regex> = OnceLock::new();
    let start = START.get_or_init(|| re(r"(?m)^==\s*Key dates\s*==\s*$"));
    let next = NEXT.get_or_init(|| re(r"(?m)^==[^=].*==\s*$"));

    let m = start.find(text)?;
    let tail = &text[m.end()..];
    let end = next.find(tail).map(|mm| mm.start()).unwrap_or(tail.len());
    let section = &tail[..end];

    // Data cells are lines starting with a single '|' (not |-, |+, |}); within the
    // table they alternate (date, event).
    let cells: Vec<String> = section
        .lines()
        .map(|l| l.trim())
        .filter(|l| {
            l.starts_with('|')
                && !l.starts_with("|-")
                && !l.starts_with("|+")
                && !l.starts_with("|}")
        })
        .map(|l| l.trim_start_matches('|').trim().to_string())
        .collect();

    for pair in cells.chunks(2) {
        if let [date, event] = pair {
            let e = event.to_lowercase();
            if e.contains("ceremony") || e.contains("telecast") {
                if let Some(iso) = parse_date_to_iso(date) {
                    return Some(iso);
                }
            }
        }
    }
    None
}

/// Parse a "Month D, YYYY" date cell to ISO `YYYY-MM-DD`.
fn parse_date_to_iso(cell: &str) -> Option<String> {
    let (cleaned, _) = clean_entry(cell);
    NaiveDate::parse_from_str(cleaned.trim(), "%B %d, %Y")
        .ok()
        .map(|d| d.format("%Y-%m-%d").to_string())
}

/// Category display name from the last `[[link|Display]]` (or `[[link]]`) inside an
/// `{{Award category}}` template — i.e. the text after the final pipe.
fn category_name(link_inner: &str) -> String {
    link_inner
        .rsplit('|')
        .next()
        .unwrap_or(link_inner)
        .trim()
        .to_string()
}

/// Clean one wikitext list item down to display text, returning `(text, is_winner)`.
fn clean_entry(item: &str) -> (String, bool) {
    static LINK_PIPED: OnceLock<Regex> = OnceLock::new();
    static LINK_PLAIN: OnceLock<Regex> = OnceLock::new();
    static TEMPLATE: OnceLock<Regex> = OnceLock::new();
    static WS: OnceLock<Regex> = OnceLock::new();

    static HTML_TAG: OnceLock<Regex> = OnceLock::new();
    static EMPTY_PARENS: OnceLock<Regex> = OnceLock::new();

    // Winners are bold ('''…'''); nominees are italic only (''…''). Some eras also
    // tag the winner with a dagger — literal ‡, {{double dagger}}, or {{double-dagger}}.
    let is_winner = item.contains("'''")
        || item.contains('\u{2021}')
        || item.contains("double dagger")
        || item.contains("double-dagger");

    let piped = LINK_PIPED.get_or_init(|| re(r"\[\[[^\]|]+\|([^\]]+)\]\]"));
    let plain = LINK_PLAIN.get_or_init(|| re(r"\[\[([^\]]+)\]\]"));
    let tmpl = TEMPLATE.get_or_init(|| re(r"\{\{[^{}]*\}\}"));
    let html_tag = HTML_TAG.get_or_init(|| re(r"<[^>]+>"));
    // Also eat a leading space so "Name (), Rest" -> "Name, Rest" (the () is often
    // an emptied <small>(…)</small> after tag stripping).
    let empty_parens = EMPTY_PARENS.get_or_init(|| re(r" *\(\s*\)"));
    let ws = WS.get_or_init(|| re(r"\s+"));

    // [[a|b]] -> b, then [[a]] -> a
    let mut out = piped.replace_all(item, "$1").to_string();
    out = plain.replace_all(&out, "$1").to_string();
    // Drop {{…}} templates (a few passes unwrap simple nesting).
    for _ in 0..3 {
        out = tmpl.replace_all(&out, "").to_string();
    }
    // Strip HTML tags (e.g. <small>…</small>) and decode common entities.
    out = html_tag.replace_all(&out, "").to_string();
    out = decode_entities(&out);
    out = out
        .replace("'''''", "")
        .replace("'''", "")
        .replace("''", "")
        .replace('\u{2021}', "");
    out = empty_parens.replace_all(&out, "").to_string();
    out = ws.replace_all(&out, " ").to_string();
    out = out.replace(" ,", ",");
    let out = out
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{2013}' || c == '-' || c == '•')
        .to_string();
    (out, is_winner)
}

/// Decode the handful of HTML entities that appear in award wikitext.
fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&ndash;", "\u{2013}")
        .replace("&mdash;", "\u{2014}")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn normalize_key(text: &str) -> String {
    text.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a ceremony page's wikitext into structured categories + nominees.
/// Returns `None` if the page has no "Winners and nominees" section.
pub fn parse_wikitext(text: &str) -> Option<ParsedCeremony> {
    let section = winner_section(text)?;

    // Strip multi-line refs and comments first: their bulleted `* {{cite}}` lines
    // would otherwise leak in as nominees, and refs can sit between a category's
    // `[[link]]` and its closing `}}`.
    static REF: OnceLock<Regex> = OnceLock::new();
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    static CAT: OnceLock<Regex> = OnceLock::new();
    let ref_re = REF.get_or_init(|| re(r"(?s)<ref[^>]*>.*?</ref>|<ref[^>]*/>"));
    let comment_re = COMMENT.get_or_init(|| re(r"(?s)<!--.*?-->"));
    let cat_re = CAT.get_or_init(|| re(r"(?s)\{\{Award category.*?\[\[([^\]]+?)\]\]"));

    let section = ref_re.replace_all(section, "");
    let section = comment_re.replace_all(&section, "");

    let mut categories: Vec<ParsedCategory> = Vec::new();

    for raw_line in section.lines() {
        let line = raw_line.trim_start();
        if line.contains("{{Award category") {
            let name = cat_re
                .captures(line)
                .map(|c| category_name(&c[1]))
                .unwrap_or_else(|| "(unparsed category)".to_string());
            let display_order = categories.len() as i64;
            categories.push(ParsedCategory {
                name,
                display_order,
                nominees: Vec::new(),
            });
            continue;
        }
        if line.starts_with('*') {
            if let Some(cat) = categories.last_mut() {
                let item = line.trim_start_matches('*').trim();
                let (text, is_winner) = clean_entry(item);
                if !text.is_empty() {
                    let source_key = normalize_key(&text);
                    cat.nominees.push(ParsedNominee {
                        title: text,
                        detail: None,
                        is_winner: Some(is_winner), // provisional; resolved below
                        source_key,
                    });
                }
            }
        }
    }

    // Winners are bold (Wikipedia lists them first, in boldface, sometimes with a
    // dagger); nominees are italic only. A category with no bold/dagger item is
    // pre-ceremony, so its winners aren't known yet → all `is_winner` become None.
    let mut has_winners = false;
    for cat in &mut categories {
        if cat.nominees.iter().any(|n| n.is_winner == Some(true)) {
            has_winners = true;
        } else {
            for n in &mut cat.nominees {
                n.is_winner = None;
            }
        }
    }

    Some(ParsedCeremony {
        has_winners,
        ceremony_date: parse_ceremony_date(text),
        categories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Expect {
        fixture: &'static str,
        has_winners: bool,
        categories: usize,
        nominees: usize,
    }

    const CASES: &[Expect] = &[
        Expect { fixture: include_str!("fixtures/emmys_76.wikitext"), has_winners: true,  categories: 25, nominees: 147 },
        Expect { fixture: include_str!("fixtures/emmys_77.wikitext"), has_winners: true,  categories: 26, nominees: 144 },
        Expect { fixture: include_str!("fixtures/emmys_78.wikitext"), has_winners: false, categories: 24, nominees: 137 },
        Expect { fixture: include_str!("fixtures/oscars_96.wikitext"), has_winners: true, categories: 23, nominees: 122 },
        Expect { fixture: include_str!("fixtures/oscars_97.wikitext"), has_winners: true, categories: 23, nominees: 123 },
    ];

    #[test]
    fn parses_all_fixtures_with_expected_shape() {
        for c in CASES {
            let parsed = parse_wikitext(c.fixture).expect("has Winners and nominees section");
            assert_eq!(parsed.categories.len(), c.categories, "category count");
            assert_eq!(parsed.has_winners, c.has_winners, "has_winners");

            let total_nominees: usize = parsed.categories.iter().map(|cat| cat.nominees.len()).sum();
            assert_eq!(total_nominees, c.nominees, "nominee count");

            // No category should fail to parse its name.
            assert!(
                parsed.categories.iter().all(|cat| cat.name != "(unparsed category)"),
                "every category name parsed"
            );

            if c.has_winners {
                // A completed ceremony: exactly one winner per category.
                for cat in &parsed.categories {
                    let winners = cat.nominees.iter().filter(|n| n.is_winner == Some(true)).count();
                    assert_eq!(winners, 1, "exactly one winner in '{}'", cat.name);
                }
            } else {
                // Pre-ceremony: nominees present, but no winner is marked anywhere.
                assert!(
                    parsed.categories.iter().all(|cat| cat.nominees.iter().all(|n| n.is_winner.is_none())),
                    "no winners marked before the ceremony"
                );
            }
        }
    }

    #[test]
    fn known_winners_are_correct() {
        let o97 = parse_wikitext(include_str!("fixtures/oscars_97.wikitext")).unwrap();
        let best_pic = o97.categories.iter().find(|c| c.name == "Best Picture").unwrap();
        let winner = best_pic.nominees.iter().find(|n| n.is_winner == Some(true)).unwrap();
        assert!(winner.title.contains("Anora"), "Best Picture winner was {}", winner.title);

        let e77 = parse_wikitext(include_str!("fixtures/emmys_77.wikitext")).unwrap();
        let drama = e77.categories.iter().find(|c| c.name == "Outstanding Drama Series").unwrap();
        let winner = drama.nominees.iter().find(|n| n.is_winner == Some(true)).unwrap();
        assert!(winner.title.contains("The Pitt"), "Drama winner was {}", winner.title);
    }

    #[test]
    fn no_section_returns_none() {
        assert!(parse_wikitext("== Something else ==\nnot an awards page").is_none());
    }

    #[test]
    fn parses_ceremony_date_from_key_dates() {
        // 78th Emmys "Key dates" lists "September 14, 2026 | NBC Telecast …".
        let e78 = parse_wikitext(include_str!("fixtures/emmys_78.wikitext")).unwrap();
        assert_eq!(e78.ceremony_date.as_deref(), Some("2026-09-14"));
    }

    #[test]
    fn old_format_winners_and_html_cleaning() {
        // 89th Oscars uses {{double-dagger}} (hyphen); 60th Emmys use bold-only
        // winners with no dagger. Both must be detected as past via list depth,
        // and their &nbsp; / <small> markup must be cleaned out.
        for (name, wt) in [
            ("oscars_89", include_str!("fixtures/oscars_89.wikitext")),
            ("emmys_60", include_str!("fixtures/emmys_60.wikitext")),
        ] {
            let p = parse_wikitext(wt).expect("has section");
            assert!(p.has_winners, "{name}: should be detected as a past ceremony");
            assert!(p.categories.len() > 5, "{name}: expected many categories");

            let total_winners: usize = p
                .categories
                .iter()
                .map(|c| c.nominees.iter().filter(|n| n.is_winner == Some(true)).count())
                .sum();
            assert!(
                total_winners >= p.categories.len().saturating_sub(1),
                "{name}: resolved {total_winners} winners across {} categories",
                p.categories.len()
            );

            for cat in &p.categories {
                for n in &cat.nominees {
                    assert!(!n.title.contains('<'), "{name}: HTML tag leaked in '{}'", n.title);
                    assert!(!n.title.contains("&nbsp;"), "{name}: entity not decoded in '{}'", n.title);
                }
            }
        }
    }
}
