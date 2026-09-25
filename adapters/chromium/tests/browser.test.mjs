import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {mkdtemp,cp,readFile,writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {generateKeyPairSync,createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.GRAMMAR_PLAYWRIGHT || 'C:/Users/USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const evidence=resolve(process.env.GRAMMAR_EVIDENCE || 'D:/dev/grammar-evidence/grammar-autonomous-r1');
const run=await mkdtemp(join(evidence,'browser-'));
const extension=join(run,'extension');await cp(resolve('adapters/chromium/extension'),extension,{recursive:true});
const {publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
const der=publicKey.export({type:'spki',format:'der'});
const id=createHash('sha256').update(der).digest('hex').slice(0,32).replace(/[0-9a-f]/g,c=>String.fromCharCode(97+parseInt(c,16)));
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
manifest.key=der.toString('base64');
// Only the isolated synthetic profile receives this test permission. Production uses activeTab.
manifest.host_permissions=['http://127.0.0.1:18437/*'];
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
 const prep=execFileSync('pwsh.exe',['-NoProfile','-File',resolve('adapters/chromium/host/Prepare-Host.ps1'),'-Executable',resolve('src-tauri/target/debug/grammar-chromium-host.exe'),'-ExtensionId',id,'-OutputDirectory',hostDir,'-Register'],{encoding:'utf8'});
const receipt=JSON.parse(prep);await writeFile(join(extension,'host-config.js'),`export const HOST = ${JSON.stringify(receipt.name)};\n`);

 context=await chromium.launchPersistentContext(join(run,'profile'),{headless:true,channel:'chromium',executablePath:process.env.GRAMMAR_CHROMIUM || 'C:/Users/USER/AppData/Local/ms-playwright/chromium-1217/chrome-win64/chrome.exe',args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
 let worker=context.serviceWorkers()[0] || await context.waitForEvent('serviceworker');
 await worker.evaluate(()=>{globalThis.fixtureMetrics={analyses:0,units:[]};chrome.runtime.onMessage.addListener(m=>{if(m.op==='analyze'){fixtureMetrics.analyses++;fixtureMetrics.units.push(m.text.length)}})});
 context.setDefaultTimeout(10000);
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto('http://127.0.0.1:18437/');
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
 await page.locator('#writing').fill('seperate');await suggestion.waitFor();await suggestion.click();await page.getByRole('button',{name:'Dismiss',exact:true}).click();
 assert.equal(await page.locator('#writing').inputValue(),'seperate');pass('Dismiss no mutation');
 await page.locator('#writing').fill('seperate!');await suggestion.waitFor();await suggestion.click();await page.getByRole('button',{name:'Ignore',exact:true}).click();
 await page.locator('#writing').fill('seperate!!');await page.waitForTimeout(700);assert.equal(await suggestion.count(),0);pass('Ignore suppresses recurring rule in session');
 await page.locator('#editable').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();await suggestion.waitFor();await suggestion.click();await page.getByRole('button',{name:'Accept',exact:true}).click();
 assert.equal(await page.locator('#editable').textContent(),'separate');pass('simple contenteditable explicit Accept reread');
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
 await page.reload();assert.equal(await page.locator('#grammar-local-assist').count(),0);pass('navigation revokes opt-in');
 assert.deepEqual(errors,[]);
 await writeFile(join(run,'result.json'),JSON.stringify({status:'PASS',classification:'SYNTHETIC_CHROMIUM_NATIVE_HOST',checks,browser:context.browser().version(),metrics:await worker.evaluate(()=>fixtureMetrics),productionActiveTabGesture:'NOT_RUN',providerLive:'NOT_RUN',sourceHead:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),sourceFiles:Object.fromEntries(await Promise.all(['core.js','content.js','worker.js','popup.js','manifest.json','host-config.js'].map(async f=>[f,createHash('sha256').update(await readFile(resolve('adapters/chromium/extension',f))).digest('hex')]))),extensionManifestSha256:createHash('sha256').update(await readFile(join(extension,'manifest.json'))).digest('hex')},null,2));
 console.log(JSON.stringify({status:'PASS',checks,evidence:run}));
} catch(error) {await writeFile(join(run,'failure.json'),JSON.stringify({checks,error:String(error),errors},null,2));console.error({evidence:run,checks,error});process.exitCode=1;}
finally {await context?.close();await new Promise(r=>server.close(r));if(existsSync(join(hostDir,'registration.json')))execFileSync('pwsh.exe',['-NoProfile','-File',resolve('adapters/chromium/host/Remove-Registration.ps1'),'-PackageDirectory',hostDir]);}

