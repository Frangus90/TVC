// Focused frontend regression checks; uses the project's existing TypeScript compiler.
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/utils/awardPredictions.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2021 },
});
const { predictionOutcome } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const category = (winners) => ({
  id: 1, name: "Award", nominees: winners.map((is_winner, i) => ({ id: i + 1, title: `Nominee ${i + 1}`, is_winner })),
});

test("unreported winners remain pending instead of wrong", () => {
  assert.equal(predictionOutcome(category([null, null]), 1), null);
  assert.equal(predictionOutcome(category([false, false]), 1), null);
});
test("confirmed winner and losing pick have distinct results", () => {
  assert.equal(predictionOutcome(category([true, false]), 1), "win");
  assert.equal(predictionOutcome(category([true, false]), 2), "miss");
});
test("either tied winner is correct", () => {
  assert.equal(predictionOutcome(category([true, true]), 1), "win");
  assert.equal(predictionOutcome(category([true, true]), 2), "win");
});
test("missing or unmatched saved nominees remain pending", () => {
  assert.equal(predictionOutcome(category([true, null]), 2), null);
  assert.equal(predictionOutcome(category([true, false]), 99), null);
  assert.equal(predictionOutcome(category([true, false]), undefined), null);
});
