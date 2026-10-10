export function matchesTierSearch(
  title: string,
  releaseDate: string | null | undefined,
  query: string,
): boolean {
  const search = query.trim().toLowerCase();
  if (title.toLowerCase().includes(search)) return true;

  const years = search.split(",").map(year => year.trim()).filter(Boolean);
  return years.length > 0
    && years.every(year => /^\d{4}$/.test(year))
    && years.includes(releaseDate?.slice(0, 4) ?? "");
}
