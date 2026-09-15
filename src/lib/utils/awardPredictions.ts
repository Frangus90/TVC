import type { CategoryDetail } from "../stores/awards.svelte";

/** A missing category result is pending, even after the ceremony has ended. */
export function predictionOutcome(
  category: CategoryDetail,
  nomineeId: number | undefined,
): "win" | "miss" | null {
  if (nomineeId == null) return null;
  const nominee = category.nominees.find((n) => n.id === nomineeId);
  if (!nominee || nominee.is_winner == null
    || !category.nominees.some((n) => n.is_winner === true)) return null;
  return nominee.is_winner === true ? "win" : "miss";
}
