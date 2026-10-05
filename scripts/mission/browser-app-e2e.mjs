// Installed app + unmodified installed extension, owned Medium Sandbox only.
import assert from 'node:assert/strict';
import {spawn,execFile,execFileSync} from 'node:child_process';
import {promisify} from 'node:util';
import {createServer} from 'node:http';
import {createRequire} from 'node:module';
import {readFile,writeFile,mkdir,readdir,rm} from 'node:fs/promises';
import {createHash} from 'node:crypto';
assert.equal(process.platform,'win32');assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
assert.notEqual(process.env.COMPUTERNAME,(await readFile('C:/GrammarPayload/host-machine.txt','utf8')).trim());
assert.match(execFileSync('whoami.exe',['/groups','/fo','csv'],{encoding:'utf8'}),/S-1-16-8192/);
const {chromium}=createRequire(import.meta.url)('C:/GrammarPayload/node_modules/playwright');
const execAsync=promisify(execFile),root='C:/GrammarTrial',profile='C:/GrammarBrowserAppProfile';
const manifest=JSON.parse(await readFile(root+'/MANIFEST.json','utf8'));
const extension=root+'/extension',ext=JSON.parse(await readFile(extension+'/manifest.json','utf8'));
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const extensionHashes=async()=>Object.fromEntries(await Promise.all((await readdir(extension)).map(async name=>[name,hash(await readFile(extension+'/'+name))])));
const originalExtension=await extensionHashes(),hashes={};
for(const name of ['codex-pencil.exe','target-apply.dll','grammar-chromium-host.exe']){
 hashes[name]=hash(await readFile(root+'/'+name));assert.equal(hashes[name],manifest.files.find(x=>x.path===name).sha256);
}
assert.equal(ext.host_permissions,undefined);assert.deepEqual(ext.permissions,['activeTab','scripting','nativeMessaging','storage']);
const title='Grammar desktop browser fixture',source='please seperate this.',corrected='please separate this.';
const html=`<!doctype html><meta charset=utf-8><title>${title}</title><label>Writing sample<textarea id=writing rows=4 cols=60></textarea></label><div id=editable contenteditable=true style="border:1px solid;width:500px;height:100px"></div><label>Password<textarea id=secret>synthetic only</textarea></label>`;
const server=createServer((req,res)=>{res.setHeader('Content-Type','text/html; charset=utf-8');res.end(html);});
await new Promise(resolve=>server.listen(18440,'127.0.0.1',resolve));
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(name,probe,ms=15000){const end=Date.now()+ms;while(Date.now()<end){const value=await probe();if(value)return value;await sleep(100);}throw Error('timeout: '+name);}
const report={status:'FAIL',classification:'INSTALLED_TAURI_BROWSER_GLOBAL_ACCEPT_SYNTHETIC_DOCUMENTS',sourceCommit:manifest.sourceCommit,hashes,checks:[],providerLive:'NOT_RUN'};
const pass=name=>{report.checks.push(name);console.log('PASS',name);};
let app,webview,widget,context,documentPage,appNatural=false;
try{
 app=spawn(root+'/codex-pencil.exe',[],{windowsHide:true,stdio:'ignore',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9481'}});
 await until('app endpoint',async()=>{try{return(await fetch('http://127.0.0.1:9481/json/version')).ok;}catch{return false;}},90000);
 webview=await chromium.connectOverCDP('http://127.0.0.1:9481');
 widget=await until('widget',()=>webview.contexts().flatMap(c=>c.pages()).find(p=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(p.url())),60000);
 await widget.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.invoke==='function',null,{timeout:60000});
 const settings=await widget.evaluate(()=>window.__TAURI_INTERNALS__.invoke('load_settings'));
 if(settings.onboardingVersion<1)await widget.getByRole('button',{name:'Continue',exact:true}).click();
 await widget.evaluate(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('load_settings');for(const key of ['cloudProcessingAcknowledgementVersion','claudeCloudAcknowledgementVersion','antigravityCloudAcknowledgementVersion'])if(s[key]!==0)throw Error('unexpected cloud grant');await window.__TAURI_INTERNALS__.invoke('save_settings',{settings:{...s,mode:'grammar',autoRewrite:false}});});
 await widget.reload();await widget.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.transformCallback==='function');
 await widget.evaluate(()=>{window.__captures=[];window.__errors=[];const t=window.__TAURI_INTERNALS__;return Promise.all(['selection-captured','capture-error'].map((event,i)=>t.invoke('plugin:event|listen',{event,target:{kind:'Any'},handler:t.transformCallback(e=>(i?window.__errors:window.__captures).push(e.payload))})));});
 const visible=()=>widget.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:window|is_visible',{label:'main'}));
 const hide=async()=>{if(await visible()){await widget.getByRole('button',{name:'Close',exact:true}).click();await until('widget hidden',async()=>!(await visible()));}};
 await hide();await mkdir(profile+'/Default',{recursive:true});
 await writeFile(profile+'/Default/Preferences',JSON.stringify({extensions:{pinned_extensions:[manifest.extensionId]}}));
 context=await chromium.launchPersistentContext(profile,{headless:false,chromiumSandbox:true,ignoreDefaultArgs:['--disable-extensions'],viewport:null,executablePath:'C:/GrammarPayload/chromium-1234/chrome-win64/chrome.exe',args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`,'--force-renderer-accessibility','--window-position=0,0','--window-size=1100,800']});
 documentPage=context.pages()[0];await documentPage.goto('http://127.0.0.1:18440/');
 const shortcut=async()=>{
  const result=await execAsync('C:/GrammarPayload/browser-hotkey-probe.exe',[],{windowsHide:true,timeout:5000});assert.equal(result.stdout.trim(),'SENT 6');
 };
 // No extension grant exists. The desktop must reject instead of native fallback.
 await documentPage.locator('#writing').fill(source);await documentPage.locator('#writing').focus();await documentPage.locator('#writing').evaluate(el=>el.select());
 await shortcut();await until('ungranted capture refusal',()=>widget.evaluate(()=>window.__errors.length===1));
 assert.equal(await widget.evaluate(()=>window.__captures.length),0);assert.equal(await documentPage.locator('#writing').inputValue(),source);pass('global capture without document opt-in refused with source unchanged');
 await hide();await documentPage.bringToFront();
 // Run the repository-owned helper as a command, like the existing guest
 // driver. Do not change machine/user execution policy or interpolate labels
 // as PowerShell code; pass the synthetic labels through the child environment.
 const gestureCommand="& ([ScriptBlock]::Create([IO.File]::ReadAllText('C:/GrammarPayload/browser-app-source/adapters/chromium/tests/ui-gesture.ps1'))) -Command Keyboard -WindowTitle $env:GRAMMAR_FIXTURE_TITLE -Name $env:GRAMMAR_FIXTURE_SEQUENCE -TimeoutMs 10000";
 const gesture=await execAsync('powershell.exe',['-NoProfile','-STA','-Command',gestureCommand],{windowsHide:true,timeout:100000,env:{...process.env,GRAMMAR_FIXTURE_TITLE:title,GRAMMAR_FIXTURE_SEQUENCE:`Extensions|prefix:${ext.name}|Enable this document`}});
 report.toolbar=JSON.parse(gesture.stdout.trim());assert.equal(report.toolbar.status,'clicked');
 await documentPage.bringToFront();await documentPage.locator('#grammar-local-assist').waitFor();pass('unmodified installed extension enabled through keyboard toolbar and popup');
 const draft=widget.getByLabel('Editable rewrite result text',{exact:true}),accept=widget.getByTestId('apply-result'),status=widget.locator('.status-pill');
 const text=selector=>documentPage.locator(selector).evaluate(el=>el.tagName==='TEXTAREA'?el.value:el.textContent);
 async function capture(selector='#writing'){
  await hide();await documentPage.bringToFront();await documentPage.locator(selector).fill('앞 😀 | '+source+' | 뒤');
  await documentPage.getByRole('button',{name:'Enable this field',exact:true}).click();
  await documentPage.getByRole('button',{name:/Suggestion 1:/}).waitFor();await documentPage.locator(selector).focus();
  await documentPage.locator(selector).evaluate((el,n)=>{if(el.tagName==='TEXTAREA')el.setSelectionRange(7,7+n);else{const r=document.createRange();r.setStart(el.firstChild,7);r.setEnd(el.firstChild,7+n);const s=getSelection();s.removeAllRanges();s.addRange(r);}},source.length);
  const count=await widget.evaluate(()=>window.__captures.length);await shortcut();
  const token=await until('global capture',()=>widget.evaluate(n=>window.__captures[n],count));
  assert.equal(token.selectedText,source);assert.equal(token.directApply,true);
  await until('Instant draft',async()=>await draft.inputValue().catch(()=>null)===corrected);await until('Accept enabled',()=>accept.isEnabled());return token;
 }
 for(const selector of ['#writing','#editable']){
  const token=await capture(selector);await draft.fill('검토한 문장 🙂');await accept.click();
  await until('explicit Accept',async()=>await status.textContent()==='Applied to the captured field');
  assert.equal(await text(selector),'앞 😀 | 검토한 문장 🙂 | 뒤');
  const replay=await widget.evaluate(t=>window.__TAURI_INTERNALS__.invoke('apply_replacement',{sessionId:t.sessionId,generation:t.generation,replacement:'replay',mode:'grammar',targetLanguage:null,autoReferenceLanguage:null,instantDraft:null,directApply:true}),token);
  assert.notEqual(replay.status,'applied');assert.equal(await text(selector),'앞 😀 | 검토한 문장 🙂 | 뒤');pass('global shortcut edited Accept and replay refusal '+selector);
 }
 await capture();await documentPage.locator('#writing').evaluate(el=>{el.value='application edit';el.dispatchEvent(new Event('input',{bubbles:true}));});await accept.click();
 await until('changed source refusal',async()=>await status.textContent()==='Apply unavailable - Copy is available');assert.equal(await text('#writing'),'application edit');pass('app-side source edit preserved through rejected desktop Accept');
 await capture();await documentPage.locator('#writing').evaluate(el=>el.setSelectionRange(0,1));await accept.click();
 await until('range refusal',async()=>await status.textContent()==='Apply unavailable - Copy is available');assert.equal(await text('#writing'),'앞 😀 | '+source+' | 뒤');pass('changed browser range refuses desktop Accept');
 const cancelled=await capture();await hide();
 const old=await widget.evaluate(t=>window.__TAURI_INTERNALS__.invoke('apply_replacement',{sessionId:t.sessionId,generation:t.generation,replacement:'cancelled',mode:'grammar',targetLanguage:null,autoReferenceLanguage:null,instantDraft:null,directApply:true}),cancelled);
 assert.notEqual(old.status,'applied');assert.equal(await text('#writing'),'앞 😀 | '+source+' | 뒤');pass('Dismiss invalidates browser capture token');
 // Re-capture after cancellation proves rearming without an injected arm command.
 await capture();await draft.fill('rearmed');await accept.click();await until('rearmed Accept',async()=>await status.textContent()==='Applied to the captured field');assert.equal(await text('#writing'),'앞 😀 | rearmed | 뒤');pass('normal field interaction rearms after cancellation');
 await hide();await documentPage.bringToFront();await documentPage.locator('#secret').focus();
 const errorCount=await widget.evaluate(()=>window.__errors.length),captureCount=await widget.evaluate(()=>window.__captures.length);await shortcut();
 await until('sensitive capture refused',()=>widget.evaluate(n=>window.__errors.length>n,errorCount));
 assert.equal(await widget.evaluate(()=>window.__captures.length),captureCount);assert.equal(await text('#secret'),'synthetic only');pass('sensitive focused field cannot fall back to previously enabled field');
 await widget.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 await until('natural app exit',()=>app.exitCode!==null,15000);assert.equal(app.exitCode,0);appNatural=true;
 await context.close();context=null;
 assert.deepEqual(await extensionHashes(),originalExtension);pass('installed extension bytes unchanged and app exited naturally');report.status='PASS';
}catch(error){report.error=String(error.stack??error);console.error(report.error);}
finally{
 report.captureErrors=widget?await widget.evaluate(()=>window.__errors).catch(()=>[]):[];
 report.captureCount=widget?await widget.evaluate(()=>window.__captures.length).catch(()=>null):null;
 report.lastStatus=widget?await widget.locator('.status-pill').textContent({timeout:1000}).catch(()=>null):null;
 if(widget&&app?.exitCode===null){await widget.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});await until('cleanup app',()=>app.exitCode!==null,15000).catch(()=>{});}
 if(context)await context.close().catch(()=>{});if(webview)await webview.close().catch(()=>{});
 if(app?.exitCode===null){app.kill();report.forcedApp=true;}
 const hostCount=()=>Number(execFileSync('powershell.exe',['-NoProfile','-Command',"@(Get-Process grammar-chromium-host -ErrorAction SilentlyContinue | Where-Object Path -eq 'C:\\GrammarTrial\\grammar-chromium-host.exe').Count"],{windowsHide:true,encoding:'utf8'}).trim());
 await until('owned native host exits',()=>hostCount()===0,15000).catch(error=>{report.cleanupError=String(error);report.status='FAIL';});
 // Fixed task-owned guest path; never recurse through user profile locations.
 await rm(profile,{recursive:true,force:true,maxRetries:3,retryDelay:100}).catch(error=>{report.cleanupError=String(error);report.status='FAIL';});
 report.appNatural=appNatural;await new Promise(resolve=>server.close(resolve));
 await writeFile('C:/GrammarOutput/browser-app-result.json',JSON.stringify(report,null,2));
}
process.exitCode=report.status==='PASS'?0:1;
