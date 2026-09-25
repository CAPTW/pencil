import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {mkdtemp,mkdir,cp,readFile,writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {generateKeyPairSync,createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.GRAMMAR_PLAYWRIGHT || 'C:/Users/USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const evidence=resolve(process.env.GRAMMAR_EVIDENCE || 'D:/dev/grammar-evidence/grammar-autonomous-r1');
const hostExecutable=resolve(process.env.GRAMMAR_HOST_EXE || 'src-tauri/target/debug/grammar-chromium-host.exe');
const extensionSource=resolve(process.env.GRAMMAR_EXTENSION_DIR || 'adapters/chromium/extension');
const syntheticDeep=process.env.GRAMMAR_SYNTHETIC_DEEP==='1';
const activeTabFixture=process.env.GRAMMAR_ACTIVE_TAB==='1';
if(syntheticDeep) {
 assert.equal(resolve(process.env.CODEX_PENCIL_CLAUDE_BIN || ''),resolve(evidence,'runtime/child.exe'),'Deep test requires the owned synthetic executable');
 assert.equal(process.env.P01_FIXTURE_MODE,'success');
 const caseDirectory=resolve(process.env.P01_CASE_DIR || '').replaceAll('\\','/');
 assert.ok(caseDirectory.startsWith(evidence.replaceAll('\\','/')+'/resume-r') && /\/resume-r[0-9]+\/[^/]+$/.test(caseDirectory),'fixture output must be task-owned');
 await mkdir(caseDirectory,{recursive:false}); // Child writes require a fresh existing output directory.
}
const run=await mkdtemp(join(evidence,'browser-'));
const extension=join(run,'extension');await cp(extensionSource,extension,{recursive:true});
const {publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
const der=publicKey.export({type:'spki',format:'der'});
const id=createHash('sha256').update(der).digest('hex').slice(0,32).replace(/[0-9a-f]/g,c=>String.fromCharCode(97+parseInt(c,16)));
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
manifest.key=der.toString('base64');
// Only the isolated synthetic profile receives this test permission. Production uses activeTab.
if(!activeTabFixture)manifest.host_permissions=['http://127.0.0.1:18437/*'];
else assert.equal(manifest.host_permissions,undefined,'production permissions must remain unchanged');
await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest,null,2));
const hostDir=join(run,'host');
const html=`<!doctype html><html lang=en><meta charset=utf-8><title>Grammar synthetic fixture</title><body>
<label for=writing>Writing sample</label><textarea id=writing rows=4 cols=50>seperate</textarea>
<div id=editable contenteditable=true style="border:1px solid;width:380px;padding:10px">seperate</div>
<label for=secret>Password</label><textarea id=secret>synthetic only</textarea>
<div id=rich contenteditable=true><b>seperate</b></div></body></html>`;
const server=createServer((req,res)=>{res.setHeader('Content-Type','text/html');res.end(html)});
await new Promise(r=>server.listen(18437,'127.0.0.1',r));
let context;let checks=[];let errors=[];
const pass=name=>{checks.push(name);console.log('PASS',name)};
try {
 const prep=execFileSync('pwsh.exe',['-NoProfile','-File',resolve('adapters/chromium/host/Prepare-Host.ps1'),'-Executable',hostExecutable,'-ExtensionId',id,'-OutputDirectory',hostDir,'-Register'],{encoding:'utf8'});
const receipt=JSON.parse(prep);await writeFile(join(extension,'host-config.js'),`export const HOST = ${JSON.stringify(receipt.name)};\n`);

 const launch=()=>chromium.launchPersistentContext(join(run,'profile'),{headless:true,channel:'chromium',executablePath:process.env.GRAMMAR_CHROMIUM || 'C:/Users/USER/AppData/Local/ms-playwright/chromium-1217/chrome-win64/chrome.exe',args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
 context=await launch();
 let worker=context.serviceWorkers()[0] || await context.waitForEvent('serviceworker');
 // Exercise a real worker restart after a native reservation reply was lost.
 // The fixture intentionally retains only the durable pre-reservation intent.
 const recoveryToken=await worker.evaluate(async host=>{
   const token=crypto.randomUUID();
   await chrome.storage.local.set({deepCleanupPending:[{token,phase:'reserving'}]});
   const reply=await new Promise((resolve,reject)=>{
     const port=chrome.runtime.connectNative(host),id=crypto.randomUUID();
     port.onMessage.addListener(value=>{port.disconnect();resolve(value);});
     port.onDisconnect.addListener(()=>{if(chrome.runtime.lastError)reject(Error('fixture reserve disconnected'));});
     port.postMessage({version:1,op:'cleanup-reserve',id,epoch:'cleanup-fixture',revision:1,text:'',cleanup_token:token});
   });
   if(!reply.cleanup_ticket || reply.error)throw Error('fixture reservation failed');
   return token;
 },receipt.name);
 // Restart only this task-owned browser/profile. runtime.reload can unload a
 // command-line-loaded extension; a clean profile restart exercises real storage.
 await context.close();context=await launch();
 worker=context.serviceWorkers()[0] || await context.waitForEvent('serviceworker');
 const recoveryPopup=await context.newPage();
 await recoveryPopup.goto(`chrome-extension://${id}/popup.html`);
 await recoveryPopup.getByRole('button',{name:'Check completed cleanup',exact:true}).click();
 await recoveryPopup.getByRole('status').filter({hasText:'Cleanup confirmed'}).waitFor();
 assert.deepEqual(await worker.evaluate(async()=> (await chrome.storage.local.get('deepCleanupPending')).deepCleanupPending),[]);
 const recoveredLedger=JSON.parse(await readFile(join(hostDir,'grammar-cleanup.json'),'utf8'));
 assert.ok(recoveredLedger.slots.every(slot=>slot.state==='FREE'));
 assert.ok(recoveredLedger.slots.some(slot=>slot.token===recoveryToken));
 await recoveryPopup.close();pass('real worker restart recovers exact reserved cleanup without inference');
 await worker.evaluate(()=>{globalThis.fixtureMetrics={analyses:0,units:[],deep:0};chrome.runtime.onMessage.addListener(m=>{if(m.op==='deep')fixtureMetrics.deep++;if(m.op==='analyze'){fixtureMetrics.analyses++;fixtureMetrics.units.push(m.text.length)}})});
 context.setDefaultTimeout(10000);
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto('http://127.0.0.1:18437/');
 if(activeTabFixture) {
   await page.bringToFront();
   const before=await worker.evaluate(async()=>{const [tab]=await chrome.tabs.query({active:true,currentWindow:true});if(!Number.isInteger(tab?.id))throw Error('fixture tab missing');try{await chrome.scripting.executeScript({target:{tabId:tab.id},func:()=>true});return 'unexpected_permission';}catch(error){return String(error);}});
   assert.match(before,/Cannot access|permission/i);
   const action=await context.newCDPSession(page);const {targetInfo}=await action.send('Target.getTargetInfo');
   await action.send('Extensions.triggerAction',{id,targetId:targetInfo.targetId});await action.detach();
   pass('production activeTab denied before and granted by protocol extension action');
 }
 const popup=await context.newPage();await page.bringToFront();await popup.goto(`chrome-extension://${id}/popup.html`);
 await popup.getByRole('button',{name:'Enable this document',exact:true}).click();
 
 await page.bringToFront();
 await page.locator('#grammar-local-assist').waitFor({timeout:10000});pass('explicit document enable');
 assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,0);pass('no read/analysis before field opt-in');
 await page.locator('#writing').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
 const suggestion=page.getByRole('button',{name:/Suggestion 1:/});await suggestion.waitFor({timeout:10000});pass('native Instant transport produced annotation');
 await page.locator('#writing').fill('seperate\nseperate');
 await page.getByRole('button',{name:/Suggestion 2:/}).waitFor();
 await page.locator('#writing').fill('seperate\nseperate!');await page.waitForTimeout(700);
 assert.equal((await worker.evaluate(()=>fixtureMetrics)).units.at(-1),9);
 assert.equal(await page.getByRole('button',{name:/Suggestion [12]:/}).count(),2);pass('incremental changed line only; untouched annotation retained');
 await suggestion.click();await page.getByRole('textbox',{name:'Edit replacement',exact:true}).fill('my draft');
 await page.getByRole('button',{name:/Suggestion 2:/}).hover();
 assert.equal(await page.getByRole('textbox',{name:'Edit replacement',exact:true}).inputValue(),'my draft');pass('passive hover preserves edited draft');
 await page.locator('#writing').fill('seperate');await suggestion.waitFor();
 const before=(await worker.evaluate(()=>fixtureMetrics)).analyses;await suggestion.hover();await suggestion.click();await page.waitForTimeout(400);
 assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,before);pass('hover and click use cache; inference count unchanged');
 await page.getByRole('button',{name:'Accept',exact:true}).click();assert.equal(await page.locator('#writing').inputValue(),'separate');pass('textarea explicit Accept reread');
 await page.locator('#writing').fill('seperate');await suggestion.waitFor();await suggestion.click();
 await page.getByRole('textbox',{name:'Edit replacement',exact:true}).fill('distinct');await page.getByRole('button',{name:'Apply edit',exact:true}).click();
 assert.equal(await page.locator('#writing').inputValue(),'distinct');pass('edited replacement reread');
 await page.locator('#writing').fill('seperate');await suggestion.waitFor();await suggestion.click();
 await page.evaluate(()=>{document.querySelector('#writing').value='user changed';});
 await page.getByRole('button',{name:'Accept',exact:true}).click();assert.equal(await page.locator('#writing').inputValue(),'user changed');pass('programmatic stale source rejects mutation');
 await page.locator('#writing').fill('seperate');await suggestion.waitFor();await suggestion.click();
 await page.locator('#writing').evaluate(el=>el.addEventListener('beforeinput',()=>{el.value='handler changed';},{once:true}));
 await page.getByRole('button',{name:'Accept',exact:true}).click();assert.equal(await page.locator('#writing').inputValue(),'handler changed');pass('synchronous beforeinput edit rejects replacement');
 await page.locator('#writing').fill('seperate');await suggestion.waitFor();await suggestion.click();await page.getByRole('button',{name:'Dismiss',exact:true}).click();
 assert.equal(await page.locator('#writing').inputValue(),'seperate');pass('Dismiss no mutation');
 await page.locator('#writing').fill('seperate!');await suggestion.waitFor();await suggestion.click();await page.getByRole('button',{name:'Ignore',exact:true}).click();
 await page.locator('#writing').fill('seperate!!');await page.waitForTimeout(700);assert.equal(await suggestion.count(),0);pass('Ignore suppresses recurring rule in session');
 await page.locator('#editable').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();await suggestion.waitFor();await suggestion.click();await page.getByRole('button',{name:'Accept',exact:true}).click();
 assert.equal(await page.locator('#editable').textContent(),'separate');pass('simple contenteditable explicit Accept reread');
 // Buffer one actual native Instant response in the isolated content world to
 // deterministically exercise a late reply during Chromium composition.
 const gateInstant=hold=>worker.evaluate(async hold=>{
   const [tab]=await chrome.tabs.query({url:'http://127.0.0.1:18437/*'});
   return (await chrome.scripting.executeScript({target:{tabId:tab.id},args:[hold],func:hold=>{
     if(!globalThis.fixtureInstantWrapped) {
       globalThis.fixtureInstantWrapped=true;const send=chrome.runtime.sendMessage.bind(chrome.runtime);
       chrome.runtime.sendMessage=async(...args)=>{
         const result=await send(...args);
         if(args[0]?.op==='analyze' && globalThis.fixtureHoldInstant)
           return new Promise(resolve=>{globalThis.fixtureReleaseInstant=()=>resolve(result);});
         return result;
       };
     }
     if(hold!==null)globalThis.fixtureHoldInstant=hold;
     if(hold===false){globalThis.fixtureReleaseInstant?.();globalThis.fixtureReleaseInstant=null;}
     return !!globalThis.fixtureReleaseInstant;
   }}))[0].result;
 },hold);
 const inputProtocol=await context.newCDPSession(page);
 for(const selector of ['#writing','#editable']) {
   await page.locator(selector).click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
   await page.locator(selector).fill('');await page.waitForTimeout(700);
   const beforeComposition=(await worker.evaluate(()=>fixtureMetrics)).analyses;
   await inputProtocol.send('Input.imeSetComposition',{text:'seperate',selectionStart:8,selectionEnd:8});
   await page.waitForTimeout(700);
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,beforeComposition);
   assert.equal(await suggestion.count(),0);
   const composingText=selector==='#writing' ? await page.locator(selector).inputValue() : await page.locator(selector).textContent();
   assert.equal(composingText,'seperate');
   await inputProtocol.send('Input.insertText',{text:'seperate'});
   await suggestion.waitFor();
   const cachedComposition=(await worker.evaluate(()=>fixtureMetrics)).analyses;
   await inputProtocol.send('Input.imeSetComposition',{text:'seperate',selectionStart:8,selectionEnd:8,replacementStart:0,replacementEnd:8});
   await page.waitForTimeout(700);assert.equal(await suggestion.count(),0);
   await inputProtocol.send('Input.insertText',{text:'seperate'});await suggestion.waitFor();
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,cachedComposition);
   await suggestion.click();await page.getByRole('button',{name:'Accept',exact:true}).press('Enter');
   const committedText=selector==='#writing' ? await page.locator(selector).inputValue() : await page.locator(selector).textContent();
   assert.equal(committedText,'separate');pass(`${selector} Chromium protocol composition defers analysis and commits safely`);
   await gateInstant(true);await page.locator(selector).fill('seperate!');
   for(let n=0;n<100 && !await gateInstant(null);n++)await page.waitForTimeout(20);
   assert.equal(await gateInstant(null),true,'actual Instant response held');
   await inputProtocol.send('Input.imeSetComposition',{text:'seperate!',selectionStart:9,selectionEnd:9,replacementStart:0,replacementEnd:9});
   await gateInstant(false);const deferredCount=(await worker.evaluate(()=>fixtureMetrics)).analyses;
   await page.waitForTimeout(350);assert.equal(await suggestion.count(),0);
   await inputProtocol.send('Input.insertText',{text:'seperate!'});await suggestion.waitFor();
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,deferredCount);
   await suggestion.click();await page.getByRole('button',{name:'Accept',exact:true}).click();
   assert.equal(selector==='#writing' ? await page.locator(selector).inputValue() : await page.locator(selector).textContent(),'separate!');
   pass(`${selector} late Instant reply waits for committed source with no repeated inference`);

 }
 await inputProtocol.detach();
 if(syntheticDeep) {
   await popup.locator('#provider').selectOption('claude');await popup.locator('#consent').check();
   await popup.getByRole('button',{name:'Allow Deep for this document',exact:true}).click();
   await page.bringToFront();await page.locator('#writing').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
   await page.locator('#writing').fill('seperate');await suggestion.waitFor();
   const deepButton=page.getByRole('button',{name:'Send current field to claude for Deep review',exact:true});
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).deep,0);
   await suggestion.click();await page.getByRole('textbox',{name:'Edit replacement',exact:true}).fill('my draft');await deepButton.click();
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).deep,0);assert.equal(await page.getByRole('textbox',{name:'Edit replacement',exact:true}).inputValue(),'my draft');pass('dirty review draft blocks Deep submission');
   await page.waitForTimeout(61000);
   assert.equal(await page.getByRole('textbox',{name:'Edit replacement',exact:true}).inputValue(),'my draft');
   assert.equal(await page.getByRole('button',{name:'Apply edit',exact:true}).isDisabled(),true);pass('expired suggestion retains edited Copy draft but no Apply authority');
   await page.getByRole('button',{name:'Dismiss',exact:true}).click();
   await deepButton.click();await page.getByRole('button',{name:'Suggestion 1: Deep claude',exact:true}).waitFor();
   assert.equal(await page.locator('#writing').inputValue(),'seperate');pass('consented selected-provider synthetic Deep produces review without mutation');assert.equal((await worker.evaluate(()=>fixtureMetrics)).deep,1);
   await page.getByRole('button',{name:'Accept',exact:true}).click();assert.equal(await page.locator('#writing').inputValue(),'Synthetic.');pass('Deep explicit Apply rereads exact synthetic result');
   await page.locator('#editable').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
   await page.locator('#editable').fill('seperate');await suggestion.waitFor();await deepButton.click();
   await page.getByRole('button',{name:'Suggestion 1: Deep claude',exact:true}).waitFor();
   await page.getByRole('button',{name:'Accept',exact:true}).press('Enter');assert.equal(await page.locator('#editable').textContent(),'Synthetic.');pass('simple contenteditable Deep keyboard Accept reread');
   await page.locator('#writing').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
   await page.locator('#writing').fill('seperate');await suggestion.waitFor();await deepButton.click();
   for(let n=0;n<100 && (await worker.evaluate(()=>fixtureMetrics)).deep<3;n++)await page.waitForTimeout(20);
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).deep,3,'third explicit request observed before edit');
   await page.locator('#writing').fill('user changed');await page.waitForTimeout(1200);
   assert.equal(await page.locator('#writing').inputValue(),'user changed');assert.equal(await page.getByRole('button',{name:'Suggestion 1: Deep claude',exact:true}).count(),0);pass('user edit cancels or discards Deep across its lifecycle');
   await popup.getByRole('button',{name:'Revoke Deep',exact:true}).click();await page.bringToFront();
   assert.equal(await page.getByRole('button',{name:'Deep requires document permission in the popup',exact:true}).isDisabled(),true);pass('Deep permission revoked');assert.equal((await worker.evaluate(()=>fixtureMetrics)).deep,3);
 }
 await page.getByRole('button',{name:'Pause field',exact:true}).click();const paused=(await worker.evaluate(()=>fixtureMetrics)).analyses;
 await page.locator('#writing').fill('seperate');await page.waitForTimeout(1200);assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,paused);pass('pause clears cache and prevents inference');
 // Poison value getter proves the associated sensitive label is checked before reading.
 await worker.evaluate(async()=>{
   const [tab]=await chrome.tabs.query({url:'http://127.0.0.1:18437/*'});
   await chrome.scripting.executeScript({target:{tabId:tab.id},func:()=>{
     globalThis.fixtureSensitiveReads=0;
     Object.defineProperty(document.querySelector('#secret'),'value',{get(){globalThis.fixtureSensitiveReads++;return 'synthetic';},configurable:true});
   }});
 });
 await page.locator('#secret').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();await page.waitForTimeout(500);
 assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,paused);const sensitiveReads=await worker.evaluate(async()=>{const [tab]=await chrome.tabs.query({url:'http://127.0.0.1:18437/*'});return (await chrome.scripting.executeScript({target:{tabId:tab.id},func:()=>globalThis.fixtureSensitiveReads}))[0].result;});
 assert.equal(sensitiveReads,0);pass('sensitive label rejected before isolated-world value read');
 await page.locator('#writing').evaluate(el=>el.setAttribute('data-sensitive','true'));
 await page.locator('#writing').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();await page.waitForTimeout(500);
 assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,paused);pass('explicit sensitive metadata denied');
 await page.locator('#secret').press('Tab');
 assert.equal(await page.evaluate(()=>document.activeElement.id),'rich');await page.getByRole('button',{name:'Enable this field',exact:true}).click();await page.waitForTimeout(400);assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,paused);pass('rich editor unsupported');
 for(const html of ['seperate<br>','<br><br>','<br class="custom">']) {
   await page.locator('#rich').evaluate((el,html)=>{el.innerHTML=html;},html);
   await page.locator('#secret').press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.id),'rich');
   await page.getByRole('button',{name:'Enable this field',exact:true}).click();await page.waitForTimeout(400);
   assert.equal((await worker.evaluate(()=>fixtureMetrics)).analyses,paused);
 }
 pass('empty-placeholder exception does not admit mixed or attributed BR editors');
 await page.reload();assert.equal(await page.locator('#grammar-local-assist').count(),0);pass('navigation revokes opt-in');
 assert.deepEqual(errors,[]);
 await writeFile(join(run,'result.json'),JSON.stringify({status:'PASS',classification:'SYNTHETIC_CHROMIUM_NATIVE_HOST',checks,browser:context.browser().version(),metrics:await worker.evaluate(()=>fixtureMetrics),productionActiveTabGesture:activeTabFixture?'SYNTHETIC_PROTOCOL_ACTION':'NOT_RUN',providerLive:'NOT_RUN',syntheticDeep,hostBinarySha256:createHash('sha256').update(await readFile(hostExecutable)).digest('hex'),sourceDirty:execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim().length>0,sourceHead:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),sourceFiles:Object.fromEntries(await Promise.all(['core.js','content.js','worker.js','popup.js','manifest.json','host-config.js'].map(async f=>[f,createHash('sha256').update(await readFile(join(extensionSource,f))).digest('hex')]))),extensionManifestSha256:createHash('sha256').update(await readFile(join(extension,'manifest.json'))).digest('hex')},null,2));
 console.log(JSON.stringify({status:'PASS',checks,evidence:run}));
} catch(error) {await writeFile(join(run,'failure.json'),JSON.stringify({checks,error:String(error),errors},null,2));console.error({evidence:run,checks,error});process.exitCode=1;}
finally {await context?.close();await new Promise(r=>server.close(r));if(existsSync(join(hostDir,'registration.json')))execFileSync('pwsh.exe',['-NoProfile','-File',resolve('adapters/chromium/host/Remove-Registration.ps1'),'-PackageDirectory',hostDir]);}

