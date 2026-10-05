// Parent is the owned LibreOffice Python fixture; its JSON commands operate
// only its newly created synthetic document. No product-private Apply shortcut.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile, writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
assert.equal(process.platform,'win32');assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
const {chromium}=createRequire(import.meta.url)('C:/GrammarPayload/node_modules/playwright');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function until(name,fn,ms=15000){const end=Date.now()+ms;while(Date.now()<end){const value=await fn();if(value)return value;await sleep(50);}throw Error('timeout: '+name);}
let reply;
createInterface({input:process.stdin}).on('line',line=>{const done=reply;reply=null;if(done)done(JSON.parse(line));});
async function command(action){assert.ok(reply==null);return new Promise((resolve,reject)=>{
 const timer=setTimeout(()=>reject(Error('Writer command timeout: '+action)),10000);
 reply=value=>{clearTimeout(timer);value.error?reject(Error(value.error)):resolve(value.value);};
 process.stdout.write(JSON.stringify({action})+'\n');
});}
let app,browser,page;
const result={status:'FAIL',classification:'INSTALLED_WRITER_WIDGET_SYNTHETIC',checks:[],providerLive:'NOT_RUN'};
const pass=name=>result.checks.push(name);
try{
 const manifest=JSON.parse(await readFile('C:/GrammarTrial/MANIFEST.json','utf8'));
 result.sourceCommit=manifest.sourceCommit;
 result.hashes={};
 for(const name of ['codex-pencil.exe','target-apply.dll']){
  const digest=createHash('sha256').update(await readFile('C:/GrammarTrial/'+name)).digest('hex');
  assert.equal(digest,manifest.files.find(x=>x.path===name).sha256);result.hashes[name]=digest;
 }
 pass('installed app and native library match manifest');
 app=spawn('C:/GrammarTrial/codex-pencil.exe',[],{windowsHide:true,stdio:'ignore',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9481'}});
 await until('WebView',async()=>{try{return (await fetch('http://127.0.0.1:9481/json/version')).ok;}catch{return false;}},60000);
 browser=await chromium.connectOverCDP('http://127.0.0.1:9481');
 page=await until('widget',()=>browser.contexts().flatMap(c=>c.pages()).find(p=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(p.url())));
 await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.invoke==='function');
 const settings=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('load_settings'));
 if(settings.onboardingVersion<1)await page.getByRole('button',{name:'Continue',exact:true}).click();
 await page.evaluate(async()=>{const t=window.__TAURI_INTERNALS__;const s=await t.invoke('load_settings');for(const k of ['cloudProcessingAcknowledgementVersion','claudeCloudAcknowledgementVersion','antigravityCloudAcknowledgementVersion'])if(s[k]!==0)throw Error('unexpected consent');await t.invoke('save_settings',{settings:{...s,mode:'grammar',autoRewrite:false}});});
 await page.reload();await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.transformCallback==='function');
 await page.evaluate(()=>{window.__captures=[];window.__errors=[];const t=window.__TAURI_INTERNALS__;return Promise.all(['selection-captured','capture-error'].map((event,i)=>t.invoke('plugin:event|listen',{event,target:{kind:'Any'},handler:t.transformCallback(e=>(i?window.__errors:window.__captures).push(e.payload))})));});
 const draft=page.getByLabel('Editable rewrite result text',{exact:true});
 const apply=page.getByTestId('apply-result');const status=page.locator('.status-pill');
 const visible=()=>page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:window|is_visible',{label:'main'}));
 async function capture(){
  if(await visible()){await page.getByRole('button',{name:'Close',exact:true}).click();await until('hidden',async()=>!(await visible()));}
  await command('prepare');await command('enable');await command('focus');
  const count=await page.evaluate(()=>window.__captures.length);await command('hotkey');
  const token=await until('Writer captured',()=>page.evaluate(i=>window.__captures[i],count));
  assert.equal(token.selectedText,'please seperate this.');assert.equal(token.directApply,true);
  await until('Instant',async()=>await draft.inputValue().catch(()=>null)==='please separate this.');await until('Accept enabled',()=>apply.isEnabled());
  return token;
 }
 await capture();await draft.fill('검토한 교체 문장 🙂');await apply.click();
 await until('Accept result',async()=>await status.textContent()==='Applied to the captured field');
 assert.equal(await command('read'),'Before | 검토한 교체 문장 🙂 | After');
 pass('actual hotkey Instant edited draft Accept exact Writer reread');
 await command('undo');assert.equal(await command('read'),'Before | please seperate this. | After');
 pass('Writer Undo restores original and surrounding text');await command('settled');
 await capture();await command('change');await apply.click();
 await until('changed source refused',async()=>await status.textContent()==='Apply unavailable - Copy is available');
 assert.equal(await command('read'),'USER SYNTHETIC CHANGE');pass('concurrent source edit is preserved without rollback');await command('settled');
 await capture();await command('disable');await apply.click();
 await until('revoked permission refused',async()=>['Apply unavailable - Copy is available','Apply outcome unconfirmed'].includes(await status.textContent()));
 result.revokedStatus=await status.textContent();
 assert.equal(await command('read'),'Before | please seperate this. | After');pass('Disable after capture prevents mutation');
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app'));
 await until('app natural exit',()=>app.exitCode!==null);assert.equal(app.exitCode,0);pass('app exits naturally');result.status='PASS';
}catch(error){result.error=String(error.stack??error);}
finally{
 if(page){result.captureErrors=await page.evaluate(()=>window.__errors).catch(()=>[]);}
 if(page&&app?.exitCode===null){await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});await until('cleanup app',()=>app.exitCode!==null).catch(()=>{});}
 if(browser)await browser.close().catch(()=>{});
 result.appNatural=app?.exitCode===0;
 if(app?.exitCode===null){app.kill();result.forced=true;result.status='FAIL';}
 await writeFile('C:/GrammarOutput/app-writer-result.json',JSON.stringify(result,null,2));
}
process.exit(result.status==='PASS'?0:1);
