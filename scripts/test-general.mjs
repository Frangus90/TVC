import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

const defaults = {
  startup_page: 'calendar', remember_calendar: false, close_action: 'tray',
  start_minimized: false, release_country: 'US', last_page: 'calendar',
  calendar_view: 'month', calendar_filter: 'all',
};
function load(invoke) {
  const code = ts.transpileModule(readFileSync('src/lib/stores/general.svelte.ts', 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
  }).outputText;
  const exports = {}, errors = [];
  vm.runInNewContext(code, {
    exports, $state: value => value,
    require: name => ({
      '@tauri-apps/api/core': { invoke },
      '../utils/logger': { logger: { error() {} } },
      './toast.svelte': { showError: message => errors.push(message) },
    })[name],
  });
  return { api: exports, errors };
}
const tick = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); };

test('general preferences serialize edits and navigation, including returning to the saved page during a slow save', async () => {
  const writes = [];
  const { api } = load((command, args) => command === 'get_general_preferences'
    ? Promise.resolve({ ...defaults })
    : new Promise((resolve, reject) => writes.push({ args, resolve, reject })));
  await api.loadGeneralPreferences();
  const first = api.updateGeneralPreferences({ release_country: 'NO' });
  const away = api.rememberLastPage('movies');
  const back = api.rememberLastPage('calendar');
  await tick();
  assert.equal(writes.length, 1, 'only one save may be pending');
  writes[0].resolve(writes[0].args.preferences);
  await first; await tick();
  assert.equal(writes[1].args.preferences.release_country, 'NO');
  assert.equal(writes[1].args.preferences.last_page, 'movies');
  writes[1].resolve(writes[1].args.preferences);
  await away; await tick();
  assert.equal(writes[2].args.preferences.last_page, 'calendar');
  writes[2].resolve(writes[2].args.preferences);
  await back;
  assert.equal(api.getGeneralPreferences().release_country, 'NO');
  assert.equal(api.getGeneralPreferences().last_page, 'calendar');
});

test('failed preference saves keep confirmed values and allow later successful changes', async () => {
  let fail = true;
  const { api } = load(async (command, args) => {
    if (command === 'get_general_preferences') return { ...defaults };
    if (fail) throw new Error('write failed');
    return args.preferences;
  });
  await api.loadGeneralPreferences();
  await assert.rejects(api.updateGeneralPreferences({ close_action: 'quit' }), /write failed/);
  assert.equal(api.getGeneralPreferences().close_action, 'tray');
  fail = false;
  await api.updateGeneralPreferences({ release_country: 'NO' });
  assert.equal(api.getGeneralPreferences().release_country, 'NO');
  assert.equal(api.getGeneralPreferences().close_action, 'tray');
});

test('startup restoration uses saved page and calendar choices only when requested', async () => {
  const { api } = load(async (command, args) => command === 'get_general_preferences'
    ? { ...defaults, last_page: 'tiers', calendar_view: 'week', calendar_filter: 'movies' }
    : args.preferences);
  await api.loadGeneralPreferences();
  assert.deepEqual(JSON.parse(JSON.stringify(api.getStartupState())), { page: 'calendar', viewMode: 'month', filter: 'all' });
  await api.updateGeneralPreferences({ startup_page: 'last_used', remember_calendar: true });
  assert.deepEqual(JSON.parse(JSON.stringify(api.getStartupState())), { page: 'tiers', viewMode: 'week', filter: 'movies' });
  await api.rememberCalendarState('agenda', 'shows');
  assert.equal(api.getStartupState().viewMode, 'agenda');
  assert.equal(api.getStartupState().filter, 'shows');
});
