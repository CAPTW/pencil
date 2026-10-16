import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {mkdirSync,writeFileSync,readFileSync,existsSync,symlinkSync,linkSync,realpathSync} from 'node:fs';
const script=process.env.PENCIL_UNINSTALL_SCRIPT||new URL('./package/Uninstall-Grammar.ps1',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1');
const windows={skip:process.platform!=='win32'};
function fixture(){
 const parent=join(process.env.PENCIL_CLEANUP_TEST_ROOT||tmpdir(),'case-'+randomUUID());
 const root=join(parent,'owned'),install=join(root,'install'),data=join(root,'profile'),outside=join(parent,'outside');
 for(const p of [install,data,outside])mkdirSync(p,{recursive:true});
 const marker={schema:'pencil-owned-trial/v1',root,id:randomUUID().replaceAll('-','')};
 writeFileSync(join(root,'.pencil-trial-owner.json'),JSON.stringify(marker));
 const receipt=JSON.stringify({schema:'grammar-personal-install/v1',installRoot:install,sourceCommit:'synthetic',nativeHost:null});
 writeFileSync(join(install,'install-receipt.json'),receipt);writeFileSync(join(install,'Uninstall-Grammar.ps1'),'synthetic');
 for(const p of [data,outside])writeFileSync(join(p,'sentinel'),'preserve');
 return {parent,root,install,data,outside,receipt};
}
function run(f,args,trial=true){return spawnSync('pwsh',['-NoProfile','-File',script,'-InstallRoot',f.install,...args],{encoding:'utf8',windowsHide:true,timeout:15000,env:{...process.env,GRAMMAR_HOST_TRIAL:trial?'owned-cleanup-regression':'',GRAMMAR_TRIAL_ROOT:''}});}
function intact(f){assert.equal(readFileSync(join(f.outside,'sentinel'),'utf8'),'preserve');assert.equal(readFileSync(join(f.data,'sentinel'),'utf8'),'preserve');assert.equal(readFileSync(join(f.install,'install-receipt.json'),'utf8'),f.receipt);assert.equal(existsSync(join(f.install,'gemma-session.lock')),false);}
for(const trial of [true,false])test('implicit shared data refuses before mutation trial='+trial,windows,()=>{const f=fixture(),r=run(f,['-RemoveUserData'],trial);assert.equal(r.status,1);assert.match(r.stderr,/REQUIRES_EXPLICIT_USER_DATA_DIRECTORY/);intact(f);});
test('explicit data without trial root refuses',windows,()=>{const f=fixture(),r=run(f,['-RemoveUserData','-UserDataDirectory',f.data]);assert.match(r.stderr,/TRIAL_ROOT_REQUIRED/);intact(f);});
for(const kind of ['outside','prefix','traversal','root','junction','nested-junction','bad-marker'])test('preflight rejects '+kind,windows,()=>{
 const f=fixture();let target=f.data;
 if(kind==='outside')target=f.outside;
 if(kind==='prefix')target=f.root+'-other';
 if(kind==='traversal')target=f.root+'\\..\\outside';
 if(kind==='root')target=f.root;
 if(kind==='junction'){target=join(f.root,'link');symlinkSync(f.outside,target,'junction');}
 if(kind==='nested-junction')symlinkSync(f.outside,join(f.data,'link'),'junction');
 if(kind==='bad-marker')writeFileSync(join(f.root,'.pencil-trial-owner.json'),JSON.stringify({schema:'pencil-owned-trial/v1',root:f.outside,id:'0'.repeat(32)}));
 const r=run(f,['-TrialRoot',f.root,'-RemoveUserData','-UserDataDirectory',target]);assert.equal(r.status,1,r.stdout+r.stderr);intact(f);
});
test('install outside scope refuses without mutation',windows,()=>{const f=fixture(),r=run(f,['-TrialRoot',f.data]);assert.equal(r.status,1);intact(f);});
test('scoped cleanup removes only owned installation and profile',windows,()=>{
 const f=fixture();mkdirSync(join(f.data,'nested'));writeFileSync(join(f.data,'nested','data'),'synthetic');
 const r=run(f,['-TrialRoot',f.root,'-RemoveUserData','-UserDataDirectory',f.data]);assert.equal(r.status,0,r.stdout+r.stderr);
 const result=JSON.parse(r.stdout);assert.equal(result.complete,true);assert.equal(existsSync(f.install),false);assert.equal(existsSync(f.data),false);assert.equal(readFileSync(join(f.outside,'sentinel'),'utf8'),'preserve');assert.equal(existsSync(join(f.root,'.pencil-trial-owner.json')),true);
});

test('hard-linked receipt cannot modify outside bytes',windows,()=>{
 const f=fixture(),backup=join(f.outside,'receipt-link');linkSync(join(f.install,'install-receipt.json'),backup);
 const r=run(f,['-TrialRoot',f.root,'-RemoveUserData','-UserDataDirectory',f.data]);assert.equal(r.status,0,r.stdout+r.stderr);assert.equal(readFileSync(backup,'utf8'),f.receipt);
});
test('foreign native registration rejected before mutation',windows,()=>{
 const f=fixture();const receipt=JSON.stringify({schema:'grammar-personal-install/v1',installRoot:f.install,sourceCommit:'synthetic',nativeHost:{name:'org.grammar.personal.t'+'0'.repeat(32),registryKey:'HKCU:\\Software\\Foreign',manifest:join(f.outside,'host.json')}});writeFileSync(join(f.install,'install-receipt.json'),receipt);f.receipt=receipt;
 const r=run(f,['-TrialRoot',f.root]);assert.equal(r.status,1);intact(f);
});

const scopeScript=process.env.PENCIL_SCOPE_SCRIPT||new URL('./package/New-GrammarTrialScope.ps1',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1');
function initScope(root){return spawnSync('pwsh',['-NoProfile','-File',scopeScript,'-Root',root],{encoding:'utf8',windowsHide:true,timeout:15000,env:{...process.env,GRAMMAR_TRIAL_ROOT:''}});}
test('scope initializer refuses adopting existing non-owned directory',windows,()=>{const f=fixture();const r=initScope(f.outside);assert.equal(r.status,1);assert.match(r.stderr,/EXISTING_ROOT_NOT_OWNED/);assert.equal(existsSync(join(f.outside,'.pencil-trial-owner.json')),false);intact(f);});
test('scope initializer refuses junction without target writes',windows,()=>{const f=fixture();const link=join(f.parent,'linked-scope');symlinkSync(f.outside,link,'junction');const r=initScope(link);assert.equal(r.status,1);assert.match(r.stderr,/TRIAL_ROOT_REPARSE/);intact(f);});
test('fresh scope initializer creates exact owned profile and marker',windows,()=>{const f=fixture();const root=join(f.parent,'new-scope');const r=initScope(root);assert.equal(r.status,0,r.stderr);const owner=JSON.parse(readFileSync(join(root,'.pencil-trial-owner.json'),'utf8'));assert.equal(owner.root,realpathSync.native(root));assert.equal(owner.schema,'pencil-owned-trial/v1');assert.equal(existsSync(join(root,'profile')),true);intact(f);});
