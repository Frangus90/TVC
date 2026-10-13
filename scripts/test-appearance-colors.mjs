import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/utils/appearanceColors.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2021 },
});
const { accentAppearance, contrastRatio, mixColors } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const css = await readFile(new URL("../src/app.css", import.meta.url), "utf8");
const variables = block => Object.fromEntries([...block.matchAll(/--([\w-]+):\s*(#[0-9a-f]{6});/gi)].map(match => [match[1], match[2]]));
const standard = variables(css.match(/@theme\s*\{([\s\S]*?)\}/)[1]);
const alternative = Object.assign({}, standard, ...[...css.matchAll(/\.colorblind-friendly\s*\{([\s\S]*?)\}/g)].map(match => variables(match[1])));
const backgrounds = [standard["color-background"], standard["color-surface"], standard["color-surface-hover"]];

test("standard and red–green status text stays readable on actual dark surfaces and tinted panels", () => {
  assert.notEqual(alternative["color-success"], standard["color-success"], "include the alternative status block, not only watched badge colours");
  assert.notEqual(alternative["color-error"], standard["color-error"]);
  const positive = alternative["color-success"], negative = alternative["color-error"];
  assert.ok(parseInt(positive.slice(5, 7), 16) > parseInt(positive.slice(1, 3), 16), "alternative positive states use blue");
  assert.ok(parseInt(negative.slice(1, 3), 16) > parseInt(negative.slice(5, 7), 16), "alternative negative states use orange");
  for (const [name, palette] of [["standard", standard], ["red–green", alternative]]) {
    for (const status of ["upcoming", "available", "watched", "premiere", "success", "warning", "error", "info"]) {
      const color = palette[`color-${status}`];
      assert.ok(color, `missing ${name} ${status}`);
      for (const background of backgrounds) {
        for (const tint of [0, 0.1, 0.15, 0.2, 0.3]) {
          const actual = mixColors(color, background, tint);
          const ratio = contrastRatio(color, actual);
          assert.ok(ratio >= 4.5, `${name} ${status} on ${background} at ${tint * 100}%: ${ratio.toFixed(2)}:1`);
        }
      }
    }
  }
});

test("watched badge text and check contrast remain strong in both palettes", () => {
  const normal = variables(css.match(/\/\* Theme variables \*\/\s*:root\s*\{([\s\S]*?)\}/)[1]);
  const colourblind = variables(css.match(/:root\.colorblind-friendly\s*\{([\s\S]*?)\}/)[1]);
  for (const palette of [normal, colourblind]) {
    assert.ok(contrastRatio(palette["focus-watched-ink"], palette["focus-watched"]) >= 4.5);
  }
});

test("every accent preset and dark custom colour has readable normal and hover buttons, labels and focus", async () => {
  const appearance = await readFile(new URL("../src/lib/components/settings/AppearanceTab.svelte", import.meta.url), "utf8");
  const presets = [...appearance.matchAll(/color:\s*"(#[0-9a-f]{6})"/gi)].map(match => match[1]);
  assert.equal(presets.length, 9, "cover every offered accent preset");
  for (const chosen of [...presets, "#000000", "#121318", "#ffffff", "#777777", "#888888", "#fff000", "#001122"]) {
    const accent = accentAppearance(chosen);
    assert.equal(accent.color, chosen, "preserve the chosen accent fill");
    assert.ok(contrastRatio(accent.foreground, chosen) >= 4.5, `${chosen} normal button`);
    assert.ok(contrastRatio(accent.hoverForeground, accent.hover) >= 4.5, `${chosen} hover button`);
    for (const background of backgrounds) {
      for (const tint of [0, 0.1, 0.2, 0.3]) {
        assert.ok(contrastRatio(accent.text, mixColors(chosen, background, tint)) >= 4.5, `${chosen} label on ${background} at ${tint}`);
      }
      assert.ok(contrastRatio(accent.focus, background) >= 3, `${chosen} keyboard focus on ${background}`);
    }
  }
});
