import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/utils/tierSearch.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2021 },
});
const { matchesTierSearch } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("tier search matches single and multiple release years, including year boundaries", () => {
  const titles = [
    { title: "Older movie", date: "2024-12-31" },
    { title: "Last year's movie", date: "2025-12-31" },
    { title: "New Year's premiere", date: "2026-01-01" },
    { title: "Year-end premiere", date: "2026-12-31" },
    { title: "Future movie", date: "2027-01-01" },
  ];
  const matches = query => titles.filter(item => matchesTierSearch(item.title, item.date, query)).map(item => item.title);
  assert.deepEqual(matches("2026"), ["New Year's premiere", "Year-end premiere"]);
  for (const query of ["2025,2026", " 2025, 2026 ", "2025,2026,2025", "2025,2026,"]) {
    assert.deepEqual(matches(query), ["Last year's movie", "New Year's premiere", "Year-end premiere"]);
  }
  assert.deepEqual(matches("202"), []);
  assert.deepEqual(matches("2026,20"), []);
  assert.deepEqual(matches(", ,"), []);
});

test("tier search keeps case-insensitive titles, numeric titles, and commas in titles", () => {
  assert.equal(matchesTierSearch("Dune: Part Two", "2024-03-01", "  PART two "), true);
  assert.equal(matchesTierSearch("1917", "2019-12-25", "1917"), true);
  assert.equal(matchesTierSearch("Monsters, Inc.", "2001-11-02", "monsters, inc."), true);
  assert.equal(matchesTierSearch("Dune: Part Two", "2024-03-01", "different title"), false);
  assert.equal(matchesTierSearch("Any title", null, "   "), true);
});

test("missing release dates do not match years but titles remain searchable", () => {
  for (const date of [null, undefined, ""]) {
    assert.equal(matchesTierSearch("Manual entry", date, "2026"), false);
    assert.equal(matchesTierSearch("Manual entry", date, "2025,2026"), false);
    assert.equal(matchesTierSearch("Manual entry", date, "manual"), true);
  }
});
