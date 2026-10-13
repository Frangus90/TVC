import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

function load(invoke) {
  const exports = {};
  const code = ts.transpileModule(readFileSync('src/lib/stores/backups.svelte.ts','utf8'), { compilerOptions:{ target:ts.ScriptTarget.ES2021,module:ts.ModuleKind.CommonJS } }).outputText;
  vm.runInNewContext(code, { exports,$state:value=>value,require:name=>{
    if(name==='@tauri-apps/api/core')return {invoke};throw new Error(`Unexpected dependency: ${name}`);
  }});
  return exports;
}
const baseline = () => ({ preferences:{frequency:'off',folder:null,retention:5,include_settings:false},running:false,last_success_at:null,last_success_path:null,last_attempt_at:null,last_error:null,next_due_at:null });
const settle = async () => { for(let index=0;index<8;index++)await Promise.resolve(); };

test('backup preference saves serialize partial edits and old polling cannot undo success', async () => {
  const pending=[];let initial=true;
  const api=load((command,args)=>{
    if(command==='get_backup_status'&&initial){initial=false;return Promise.resolve(baseline());}
    return new Promise((resolve,reject)=>pending.push({command,args,resolve,reject}));
  });
  await api.loadBackupStatus();
  const poll=api.loadBackupStatus();
  const first=api.updateBackupPreferences({frequency:'weekly'});
  const second=api.updateBackupPreferences({include_settings:true});
  await settle();assert.equal(pending.length,2,'old poll plus first save only');
  pending[1].resolve({...baseline(),preferences:{...baseline().preferences,frequency:'weekly'}});
  await first;await settle();
  assert.equal(pending[2].args.preferences.frequency,'weekly');
  pending[0].resolve(baseline());await poll;
  assert.equal(api.getBackupStatus().preferences.frequency,'weekly','old poll ignored');
  pending[2].resolve({...baseline(),preferences:{...baseline().preferences,frequency:'weekly',include_settings:true}});
  await second;assert.equal(api.getBackupStatus().preferences.include_settings,true);assert.equal(api.isBackupSaving(),false);
});

test('backup settings failures preserve persisted preferences and the next save can retry', async () => {
  let fail=true;
  const api=load(async(command,args)=>{
    if(command==='get_backup_status')return baseline();
    if(fail)throw new Error('folder unavailable');return {...baseline(),preferences:args.preferences};
  });
  await api.loadBackupStatus();
  await assert.rejects(api.updateBackupPreferences({frequency:'daily'}));
  assert.equal(api.getBackupStatus().preferences.frequency,'off');assert.match(api.getBackupError(),/not saved/);
  fail=false;await api.updateBackupPreferences({frequency:'daily'});
  assert.equal(api.getBackupStatus().preferences.frequency,'daily');assert.equal(api.getBackupError(),null);
});

test('restore gates navigation and drains pending saves before native replacement', async () => {
  let release,imports=0;
  const api=load(async(command)=>{assert.equal(command,'import_database');imports++;return {restart_required:true,settings_restored:true};});
  const restore=api.restoreBackup({},()=>new Promise(resolve=>release=resolve));
  assert.equal(api.isBackupRestoring(),true);assert.equal(imports,0);assert.equal(api.isBackupRestartRequired(),false);
  release();await restore;
  assert.equal(imports,1);assert.equal(api.isBackupRestoring(),false);assert.equal(api.isBackupRestartRequired(),true);
  await assert.rejects(api.restoreBackup({},async()=>{}),/restarting/);
});

test('failed restore preparation or native import releases gate without requiring restart', async () => {
  let imports=0;
  const api=load(async()=>{imports++;throw new Error('invalid backup');});
  await assert.rejects(api.restoreBackup({},async()=>{throw new Error('save failed');}),/save failed/);
  assert.equal(imports,0);assert.equal(api.isBackupRestoring(),false);assert.equal(api.isBackupRestartRequired(),false);
  await assert.rejects(api.restoreBackup({},async()=>{}),/invalid backup/);
  assert.equal(imports,1);assert.equal(api.isBackupRestoring(),false);assert.equal(api.isBackupRestartRequired(),false);
});
