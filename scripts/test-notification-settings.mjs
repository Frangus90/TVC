import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
function load(path, dependencies, globals = {}) {
  const exports = {};
  const code = ts.transpileModule(readFileSync(path, 'utf8'), { compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS } }).outputText;
  vm.runInNewContext(code, { exports, $state: value => value, console, setTimeout, clearTimeout, ...globals,
    require: name => { if (!(name in dependencies)) throw new Error(`Missing test dependency: ${name}`); return dependencies[name]; },
  }, { filename: path });
  return exports;
}
const logger = { error() {}, warn() {}, debug() {} };
const baseline = { enabled:true,sound_enabled:true,sound_volume:50,sound_choice:'chime',popup_position:'top-right',popup_duration:8000,max_visible:3,os_fallback:false,tray_notifications:true,racing_enabled:true,plex_enabled:true,premiere_enabled:true,update_enabled:true,system_enabled:true };
const settle = async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); };
function notifications() {
  const saves = [];
  const api = load('src/lib/stores/notifications.svelte.ts', {
    '@tauri-apps/api/core': { invoke: (command, args) => command === 'get_notification_settings' ? Promise.resolve({...baseline}) : new Promise((resolve,reject)=>saves.push({command,args,resolve,reject})) },
    '@tauri-apps/api/event': { listen: async () => () => {} },
    '../utils/logger': {logger}, './toast.svelte': {showError(){}}, '../utils/notificationSound': {playNotificationSound(){}},
  });
  return {api,saves};
}
test('notification preference changes merge immediately and save full snapshots in order', async () => {
  const {api,saves}=notifications(); await api.loadNotificationSettings();
  const first=api.updateNotificationSettings({sound_volume:70});
  const second=api.updateNotificationSettings({racing_enabled:false});
  assert.equal(api.getNotificationSettings().sound_volume,70);
  assert.equal(api.getNotificationSettings().racing_enabled,false);
  await settle(); assert.equal(saves.length,1,'later saves wait for the first write');
  saves[0].resolve(); await first; await settle();
  assert.equal(saves.length,2);
  assert.equal(saves[1].args.settings.sound_volume,70);
  assert.equal(saves[1].args.settings.racing_enabled,false);
  assert.equal(api.isNotificationSettingsSaving(),true);
  saves[1].resolve(); await second;
  assert.equal(api.isNotificationSettingsSaving(),false);
  assert.equal(api.getNotificationSettings().racing_enabled,false,'an old response cannot revert the newer selection');
});
test('notification preference failure is visible and retry saves the latest selections', async () => {
  const {api,saves}=notifications(); await api.loadNotificationSettings();
  const first=api.updateNotificationSettings({sound_volume:80}); await settle(); saves[0].reject(new Error('write failed'));
  assert.equal(await first,false); assert.match(api.getNotificationSettingsSaveError(),/not saved/);
  const retry=api.retryNotificationSettingsSave(); await settle(); assert.equal(saves[1].args.settings.sound_volume,80);
  saves[1].resolve(); assert.equal(await retry,true); assert.equal(api.getNotificationSettingsSaveError(),null);
});
test('notification preference loading failures remain visible and can be retried', async () => {
  let failing=true;
  const api=load('src/lib/stores/notifications.svelte.ts', {
    '@tauri-apps/api/core':{invoke:async()=>{if(failing)throw new Error('read failed');return {...baseline};}},
    '@tauri-apps/api/event':{listen:async()=>()=>{}}, '../utils/logger':{logger}, './toast.svelte':{showError(){}}, '../utils/notificationSound':{playNotificationSound(){}},
  });
  await api.loadNotificationSettings(); assert.match(api.getNotificationSettingsLoadError(),/read failed/); assert.equal(api.getNotificationSettings(),null);
  failing=false; await api.loadNotificationSettings(); assert.equal(api.getNotificationSettingsLoadError(),null); assert.equal(api.getNotificationSettings().enabled,true);
});
function theme(rows=[]) {
  const writes=[],styles=new Map(); let fail=false;
  const database={select:async()=>rows,execute:async(_sql,args)=>{writes.push(args);if(fail)throw new Error('disk unavailable');}};
  const api=load('src/lib/stores/theme.svelte.ts', {'../utils/database':{getDatabase:async()=>database},'../utils/logger':{logger}},
    {document:{documentElement:{style:{setProperty:(k,v)=>styles.set(k,v)},classList:{add(){},remove(){}}}}});
  return {api,writes,styles,setFail:value=>fail=value};
}
test('appearance normalizes legacy and custom short hex colours and rejects invalid values', async () => {
  const {api,writes,styles}=theme([{key:'theme_accentColor',value:'#AbC'}]); await api.loadThemeSettings();
  assert.equal(api.getThemeSettings().accentColor,'#aabbcc');
  assert.equal(await api.updateThemeSettings({accentColor:'not a colour'}),false);assert.equal(writes.length,0);
  assert.equal(api.getThemeSettings().accentColor,'#aabbcc');
  assert.equal(await api.updateThemeSettings({accentColor:'#fff'}),true);const shortHover=styles.get('--accent-hover');
  assert.equal(api.getThemeSettings().accentColor,'#ffffff');
  await api.updateThemeSettings({accentColor:'#ffffff'}); assert.equal(styles.get('--accent-hover'),shortHover);
  assert.ok(writes.some(([key,value])=>key==='theme_accentColor'&&value==='#ffffff'));
});
test('appearance write failure is visible and retry saves the current preferences', async () => {
  const {api,writes,setFail}=theme(); await api.loadThemeSettings();setFail(true);
  assert.equal(await api.updateThemeSettings({fontSize:1.25}),false);assert.match(api.getThemeSaveError(),/not saved/);
  setFail(false);assert.equal(await api.retryThemeSave(),true);assert.equal(api.getThemeSaveError(),null);
  assert.ok(writes.some(([key,value])=>key==='theme_fontSize'&&value==='1.25'));
});
test('appearance slider changes cannot finish writing older values after a newer selection', async () => {
  const persisted = new Map(); let unblockFirst; let blocked = false;
  const database = {
    select: async () => [],
    execute: async (_sql,[key,value]) => {
      if (!blocked) { blocked=true; await new Promise(resolve=>unblockFirst=resolve); }
      persisted.set(key,value);
    },
  };
  const api=load('src/lib/stores/theme.svelte.ts', {'../utils/database':{getDatabase:async()=>database},'../utils/logger':{logger}},
    {document:{documentElement:{style:{setProperty(){}},classList:{add(){},remove(){}}}}});
  await api.loadThemeSettings();
  const first=api.updateThemeSettings({fontSize:1.1});
  const newer=api.updateThemeSettings({fontSize:1.3});
  await settle(); assert.equal(api.getThemeSettings().fontSize,1.3);
  unblockFirst(); await Promise.all([first,newer]);
  assert.equal(persisted.get('theme_fontSize'),'1.3');
});
