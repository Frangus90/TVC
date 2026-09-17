import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import test from "node:test";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/utils/backup.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2021 },
});
const { parseBackup, describeBackup } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const personal = () => ({
  ceremonies: [], categories: [], nominees: [], predictions: [],
  title_mappings: [], scrobble_history: [], change_history: [],
});
const backup = () => ({
  version: "3.0", exported_at: "2026-09-17T10:00:00Z",
  shows: [], episodes: [], movies: [], tiers: [], personal_data: personal(),
});

test("new backup preview reports included counts and unchanged settings", () => {
  const value = backup();
  value.shows = [{ id: 1 }];
  value.personal_data.predictions = [{ id: 1 }, { id: 2 }];
  const message = describeBackup(parseBackup(JSON.stringify(value)));
  assert.match(message, /1 shows, 0 episodes, 0 movies, and 0 tiers/);
  assert.match(message, /2 saved predictions/);
  assert.match(message, /Settings, credentials, and racing data stay unchanged/);
});

test("older backups explain the records they cannot recover before import", () => {
  for (const version of [undefined, "1.0", "2.0"]) {
    const value = backup();
    value.version = version;
    delete value.personal_data;
    delete value.tiers;
    const parsed = parseBackup(JSON.stringify(value));
    assert.equal(parsed.version, version ?? "1.0");
    assert.deepEqual(parsed.tiers, []);
    const message = describeBackup(parsed);
    assert.match(message, /Existing awards and predictions will be kept/);
    assert.match(message, /Plex title corrections, scrobble logs, and change history will be cleared/);
  }
});

test("unknown versions and incomplete current backups are rejected", () => {
  const badVersion = backup(); badVersion.version = "4.0";
  const badRecords = backup(); badRecords.shows = {};
  const noPersonal = backup(); delete noPersonal.personal_data;
  const noTiers = backup(); delete noTiers.tiers;
  const noPicks = backup(); delete noPicks.personal_data.predictions;
  const misleadingVersion = backup(); misleadingVersion.version = "2.0";
  for (const value of [null, [], {}, badVersion, badRecords, noPersonal, noTiers, noPicks, misleadingVersion]) {
    assert.throws(() => parseBackup(JSON.stringify(value)));
  }
  assert.throws(() => parseBackup("{not valid JSON"));
});
