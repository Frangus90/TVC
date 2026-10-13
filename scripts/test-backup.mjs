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
    value.tiers = null;
    assert.deepEqual(parseBackup(JSON.stringify(value)).tiers, [], "old Option<Tiers> serialized as null stays compatible");
  }
});

test("unknown versions and incomplete current backups are rejected", () => {
  const badVersion = backup(); badVersion.version = "99.0";
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

const portable = () => ({
  appearance: { accentColor:"#bba4ff",fontSize:1,compactSpacing:false,colorblindFriendly:false,hidePosters:false,posterSize:"medium" },
  general: { startup_page:"calendar",remember_calendar:false,close_action:"tray",start_minimized:false,release_country:"US",last_page:"calendar",calendar_view:"month",calendar_filter:"all" },
  notifications: { enabled:true,sound_enabled:true,sound_volume:50,sound_choice:"chime",popup_position:"top-right",popup_duration:8000,max_visible:3,os_fallback:false,tray_notifications:true,racing_enabled:true,plex_enabled:true,premiere_enabled:true,update_enabled:true,system_enabled:true,quiet_hours_enabled:false,quiet_hours_start:"22:00",quiet_hours_end:"08:00" },
  racing: { notifications_enabled:true,default_notify_minutes:30,series:[] }, tier_preset:"10-star",
});
test("portable backup preview explains transferred settings and machine exclusions", () => {
  const value = { ...backup(), version:"4.0", portable_settings:portable() };
  const parsed = parseBackup(JSON.stringify(value));
  const message = describeBackup(parsed);
  assert.match(message, /Appearance, General and calendar preferences/);
  assert.match(message, /including quiet hours/);
  assert.match(message, /Racing series\/reminder preferences/);
  assert.match(message, /0 supplied series; series absent from the backup are disabled/);
  assert.match(message, /Credentials, Windows startup registration, backup folders and schedules/);
  assert.match(message, /custom Racing feed URLs/);
  assert.match(message, /Restart TVC/);
});
test("portable backup rejects missing groups and unsupported preference keys before confirmation", () => {
  const value = () => ({ ...backup(), version:"4.0", portable_settings:portable() });
  const missing = value(); delete missing.portable_settings.notifications;
  const secret = value(); secret.portable_settings.general.api_key = "secret";
  const url = value(); url.portable_settings.racing.series = [{ slug:"f1",enabled:true,notify_enabled:true,notify_minutes:30,custom_color:null,custom_ics_url:"secret" }];
  const unsupported = value(); unsupported.version = "3.0";
  const absent = value(); delete absent.portable_settings;
  for (const data of [missing, secret, url, unsupported, absent]) assert.throws(() => parseBackup(JSON.stringify(data)));
});
