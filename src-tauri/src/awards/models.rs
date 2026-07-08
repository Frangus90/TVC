//! Structured shapes produced by parsing an award ceremony page, before they are
//! persisted into the `award_*` tables.

/// A single nominee within a category.
///
/// `is_winner`: `Some(true)` won, `Some(false)` lost, `None` = winner not yet
/// announced (the ceremony's nominations are out but results are not). The `None`
/// state is what the prediction game uses to keep a category open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNominee {
    pub title: String,
    pub detail: Option<String>,
    pub is_winner: Option<bool>,
    /// Normalized natural key, stable across re-syncs, for idempotent upsert.
    pub source_key: String,
}

/// A competitive category within a ceremony (e.g. "Best Picture").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCategory {
    pub name: String,
    pub display_order: i64,
    pub nominees: Vec<ParsedNominee>,
}

/// A ceremony as parsed from its Wikipedia page.
///
/// `has_winners` is true once any category has a marked winner; the sync layer maps
/// this to the ceremony's `status` (`past` vs `nominated`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCeremony {
    pub has_winners: bool,
    pub categories: Vec<ParsedCategory>,
}
