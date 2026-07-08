//! Fetch award ceremony wikitext from the MediaWiki API and parse it into a
//! `ParsedCeremony`. The parsing algorithm was validated against 5 real pages
//! (see the `tests` module + `fixtures/`) before being ported here.
//!
//! Parsing keys off Wikipedia's `{{Award category|…|[[link|Name]]}}` template
//! (identical on Oscar and Emmy pages — headings vary, the template does not) and
//! the winner marker (`‡` on Oscars, `{{double dagger}}` on Emmys).

use crate::awards::models::{ParsedCategory, ParsedCeremony, ParsedNominee};
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

    let is_winner = item.contains('\u{2021}') || item.contains("double dagger");

    let piped = LINK_PIPED.get_or_init(|| re(r"\[\[[^\]|]+\|([^\]]+)\]\]"));
    let plain = LINK_PLAIN.get_or_init(|| re(r"\[\[([^\]]+)\]\]"));
    let tmpl = TEMPLATE.get_or_init(|| re(r"\{\{[^{}]*\}\}"));
    let ws = WS.get_or_init(|| re(r"\s+"));

    // [[a|b]] -> b, then [[a]] -> a
    let mut out = piped.replace_all(item, "$1").to_string();
    out = plain.replace_all(&out, "$1").to_string();
    // Drop {{…}} templates (a few passes unwrap simple nesting).
    for _ in 0..3 {
        out = tmpl.replace_all(&out, "").to_string();
    }
    out = out
        .replace("'''''", "")
        .replace("'''", "")
        .replace("''", "")
        .replace('\u{2021}', "");
    out = ws.replace_all(&out, " ").to_string();
    let out = out
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{2013}' || c == '-' || c == '•')
        .to_string();
    (out, is_winner)
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

    // Per-category winner resolution: if a category has a marked winner, non-winners
    // are `Some(false)`; if it has none, the results aren't out so all become `None`.
    let mut has_winners = false;
    for cat in &mut categories {
        let winner_present = cat.nominees.iter().any(|n| n.is_winner == Some(true));
        if winner_present {
            has_winners = true;
        } else {
            for n in &mut cat.nominees {
                n.is_winner = None;
            }
        }
    }

    Some(ParsedCeremony {
        has_winners,
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
}
