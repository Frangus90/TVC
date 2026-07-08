//! Pure prediction scoring — no DB, no Tauri — so it is trivially unit-tested.

use std::collections::HashMap;

/// Score the user's picks against revealed winners.
///
/// * `picks`   — `(category_id, predicted_nominee_id)` pairs, one per category.
/// * `winners` — `category_id -> winning_nominee_id`, only for categories whose
///   winner has been announced.
///
/// Only categories with a known winner are scored (an undecided category can't be
/// right or wrong yet). Returns `(correct, scored_total)`.
pub fn score_predictions(picks: &[(i64, i64)], winners: &HashMap<i64, i64>) -> (u32, u32) {
    let mut correct = 0;
    let mut total = 0;
    for (category_id, predicted) in picks {
        if let Some(winner) = winners.get(category_id) {
            total += 1;
            if predicted == winner {
                correct += 1;
            }
        }
    }
    (correct, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn winners(pairs: &[(i64, i64)]) -> HashMap<i64, i64> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn all_correct() {
        let picks = [(1, 10), (2, 20), (3, 30)];
        let w = winners(&[(1, 10), (2, 20), (3, 30)]);
        assert_eq!(score_predictions(&picks, &w), (3, 3));
    }

    #[test]
    fn some_wrong() {
        let picks = [(1, 10), (2, 99), (3, 30)];
        let w = winners(&[(1, 10), (2, 20), (3, 30)]);
        assert_eq!(score_predictions(&picks, &w), (2, 3));
    }

    #[test]
    fn undecided_categories_are_not_scored() {
        // Picked 3 categories, but only categories 1 and 2 have winners announced.
        let picks = [(1, 10), (2, 20), (3, 30)];
        let w = winners(&[(1, 10), (2, 99)]);
        assert_eq!(score_predictions(&picks, &w), (1, 2));
    }

    #[test]
    fn no_winners_yet_scores_zero_of_zero() {
        let picks = [(1, 10), (2, 20)];
        let w = winners(&[]);
        assert_eq!(score_predictions(&picks, &w), (0, 0));
    }

    #[test]
    fn winner_without_a_pick_is_ignored() {
        // User only picked category 1; category 2 has a winner but no pick.
        let picks = [(1, 10)];
        let w = winners(&[(1, 10), (2, 20)]);
        assert_eq!(score_predictions(&picks, &w), (1, 1));
    }
}
