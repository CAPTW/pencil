// Task-owned modified-extension fixture, real native stdio and resident GPU broker.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {mkdtemp,cp,readFile,writeFile,rm} from 'node:fs/promises';
import {join,resolve,sep} from 'node:path';
import {createHash,generateKeyPairSync} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {createInterface} from 'node:readline';
const require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/USER/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const evidence='D:/dev/grammar-evidence/grammar-gemma-20261007';
const binaryRoot=resolve(process.env.GRAMMAR_GEMMA_BIN_DIR || 'src-tauri/target/debug');
const run=await mkdtemp(join(evidence,'live-browser-'));
const hostDir=join(run,'host'),extension=join(run,'extension'),profile=join(run,'profile');
const hash=async path=>createHash('sha256').update(await readFile(path)).digest('hex');
const pwsh=(script,args)=>execFileSync('pwsh.exe',['-NoProfile','-File',resolve(script),...args],{encoding:'utf8',windowsHide:true,timeout:20000});
const report={scope:'MODIFIED_EXTENSION_OWNED_PROFILE_REAL_STDIO_GPU_NOT_INSTALLED_PACKAGE',rows:[],checks:[],cleanup:{}};
report.scriptSha256=await hash(new URL(import.meta.url));
let broker,context,registered=false,brokerExit;
const server=createServer((req,res)=>{res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Owned Gemma fixture</title><textarea id="writing"></textarea><div id="editable" contenteditable="true" style="min-height:40px;border:1px solid"></div><input id="password" type="password">');});
try {
 await cp('adapters/chromium/extension',extension,{recursive:true});
 const {publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
 const der=publicKey.export({type:'spki',format:'der'});
 const id=createHash('sha256').update(der).digest('hex').slice(0,32).replace(/[a-f0-9]/g,c=>String.fromCharCode(97+parseInt(c,16)));
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const url=`http://127.0.0.1:${server.address().port}/`;
 const manifest=JSON.parse(await readFile(join(extension,'manifest.json')));
 manifest.key=der.toString('base64');manifest.host_permissions=[url+'*'];
 await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest));
 const registration=JSON.parse(pwsh('adapters/chromium/host/Prepare-Host.ps1',['-Executable',join(binaryRoot,'grammar-chromium-host.exe'),'-ExtensionId',id,'-OutputDirectory',hostDir,'-Register']));registered=true;
 await writeFile(join(extension,'host-config.js'),`export const HOST=${JSON.stringify(registration.name)};\n`);
 const brokerPath=join(hostDir,'grammar-gemma-broker.exe'),hostPath=join(hostDir,'grammar-chromium-host.exe');
 await cp(join(binaryRoot,'grammar-gemma-broker.exe'),brokerPath);
 report.brokerSha256=await hash(brokerPath);report.hostSha256=await hash(hostPath);
 const start=performance.now();
 broker=spawn(brokerPath,[join(evidence,'runtime/llama-server.exe'),'a812655fed77b9f98d918e7aa0b1552403b76dacbeb9fa24c17dac9ed3d08ec1',join(evidence,'gemma-4-12b-it-qat-q4_0.gguf'),'93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b',hostPath,report.hostSha256],{windowsHide:true,stdio:['pipe','pipe','pipe']});
 broker.stdin.on('error',error=>{report.controlError=error.code;});
 brokerExit=new Promise(r=>broker.once('exit',(code,signal)=>r({code,signal})));
 let readyResolve,readyReject;const readyPromise=new Promise((r,j)=>{readyResolve=r;readyReject=j;});
 const timer=setTimeout(()=>readyReject(Error('broker startup timeout')),190000);
 const lines=createInterface({input:broker.stdout});
 lines.on('line',line=>{try{const value=JSON.parse(line);if(value.name)readyResolve(value);if(value.cleanup)report.cleanup.broker=value.cleanup;}catch{readyReject(Error('invalid broker metadata'));}});
 broker.on('error',readyReject);broker.once('exit',()=>readyReject(Error('broker startup exit')));
 broker.stderr.on('data',data=>{report.brokerError=(report.brokerError||'')+data.toString().slice(0,1024);});
 let ready;try{ready=await readyPromise;}finally{clearTimeout(timer);}
 report.startupMs=performance.now()-start;
 const configPath=join(hostDir,'grammar-chromium-host.origin.json');
 const config=JSON.parse(await readFile(configPath));config.gemma={pipe:ready.name,broker_sha256:report.brokerSha256};
 await writeFile(configPath,JSON.stringify(config));
 context=await chromium.launchPersistentContext(profile,{headless:true,chromiumSandbox:true,executablePath:'C:/Users/USER/AppData/Local/ms-playwright/chromium-1217/chrome-win64/chrome.exe',ignoreDefaultArgs:['--disable-extensions'],args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
 context.setDefaultTimeout(12000);
 const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');
 await worker.evaluate(()=>{globalThis.gemmaMetrics={requests:0,responses:0};chrome.runtime.onMessage.addListener(m=>{if(m.op==='analyze')gemmaMetrics.requests++;});const original=chrome.runtime.connectNative.bind(chrome.runtime);chrome.runtime.connectNative=(...args)=>{const port=original(...args);port.onMessage.addListener(m=>{if(Array.isArray(m.suggestions))gemmaMetrics.responses++;});return port;};});
 const page=await context.newPage();await page.goto(url);await page.bringToFront();
 const popup=await context.newPage();await page.bringToFront();await popup.goto(`chrome-extension://${id}/popup.html`);
 await popup.getByRole('button',{name:'Enable this document',exact:true}).click();
 await page.locator('#grammar-local-assist').waitFor();
 assert.equal((await worker.evaluate(()=>gemmaMetrics)).requests,0);report.checks.push('no analysis before field opt-in');
 for(const [field,source,expected] of [['writing','She have two books.','She has two books.'],['editable','He do not agree.','He does not agree.']]) {
   await page.locator('#'+field).click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
   await page.evaluate(({field,source})=>{
     const el=document.getElementById(field),shadow=document.getElementById('grammar-local-assist').shadowRoot;
     window.trial={};const observer=new MutationObserver(()=>{if(shadow.querySelector('[aria-label^="Suggestion 1:"]')){requestAnimationFrame(()=>{window.trial.recommendMs=performance.now()-window.trial.start;});observer.disconnect();}});
     observer.observe(shadow,{subtree:true,childList:true});
     if(el.tagName==='TEXTAREA')el.value=source;else el.textContent=source;
     window.trial.start=performance.now();el.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertText',data:source}));
   },{field,source});
   await page.waitForFunction(()=>Number.isFinite(window.trial?.recommendMs),null,{timeout:12000});
   const before=await worker.evaluate(()=>gemmaMetrics.requests);
   const suggestion=page.getByRole('button',{name:/Suggestion 1:/});await suggestion.hover();await suggestion.click();
   assert.equal(await page.getByRole('textbox',{name:'Edit replacement',exact:true}).inputValue(),expected);
   assert.equal((await worker.evaluate(()=>gemmaMetrics.requests)),before);report.checks.push(field+' cached card has no new analysis');
   await page.evaluate(({field,expected})=>{
     const shadow=document.getElementById('grammar-local-assist').shadowRoot;
     const accept=[...shadow.querySelectorAll('button')].find(b=>b.textContent==='Accept');
     accept.addEventListener('click',()=>{
       const start=performance.now();
       setTimeout(()=>{
         const el=document.getElementById(field),text=el.tagName==='TEXTAREA'?el.value:el.textContent;
         window.trial.row={field,recommendMs:window.trial.recommendMs,applyMs:performance.now()-start,text,correct:text===expected};
       });
     },{capture:true,once:true});
   },{field,expected});
   await page.getByRole('button',{name:'Accept',exact:true}).click();
   await page.getByRole('button',{name:'Pause field',exact:true}).click();
   await page.waitForFunction(()=>!!window.trial.row);
   const row=await page.evaluate(field=>({...window.trial.row,actualReread:document.getElementById(field).tagName==='TEXTAREA'?document.getElementById(field).value:document.getElementById(field).textContent}),field);report.rows.push(row);assert.ok(row.correct);assert.equal(row.actualReread,expected);
   assert.equal((await worker.evaluate(()=>gemmaMetrics.requests)),before);report.checks.push(field+' Accept reread correct; no inference during Accept');
 }
 await page.locator('#writing').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
 await page.locator('#writing').fill('She have two books.');
 await page.getByRole('button',{name:/Suggestion 1:/}).click();
 const boundaryCount=await worker.evaluate(()=>gemmaMetrics.requests);
 await page.evaluate(()=>{
   const shadow=document.getElementById('grammar-local-assist').shadowRoot;
   [...shadow.querySelectorAll('button')].find(b=>b.textContent==='Accept').addEventListener('mousedown',()=>{document.getElementById('writing').value='Owned page edit must survive.';},{once:true});
 });
 await page.getByRole('button',{name:'Accept',exact:true}).click();
 assert.equal(await page.locator('#writing').inputValue(),'Owned page edit must survive.');
 await page.getByRole('button',{name:'Pause field',exact:true}).click();
 assert.equal(await worker.evaluate(()=>gemmaMetrics.requests),boundaryCount);report.checks.push('source changed immediately before Accept is preserved; no inference during refused Accept');
 await page.locator('#password').click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
 const count=await worker.evaluate(()=>gemmaMetrics.requests);await page.waitForTimeout(400);
 assert.equal(await worker.evaluate(()=>gemmaMetrics.requests),count);report.checks.push('password refusal');
 await page.getByRole('button',{name:'Disable document',exact:true}).click();
 assert.equal(await page.locator('#grammar-local-assist').count(),0);report.checks.push('disable removes UI');
 report.functional='PASS';report.latencyPass=report.rows.every(r=>r.recommendMs<=1000&&r.applyMs<=500);
 const stats=key=>{const v=report.rows.map(r=>r[key]).sort((a,b)=>a-b);return {n:v.length,p50:v[Math.ceil(v.length*.5)-1],p95:v[Math.ceil(v.length*.95)-1],p99:v[Math.ceil(v.length*.99)-1],max:v.at(-1)};};
 report.latency={recommend:stats('recommendMs'),apply:stats('applyMs')};
 if(!report.latencyPass)process.exitCode=1;
} catch(error) {report.functional='FAIL';report.error=String(error);process.exitCode=1;}
finally {
 if(context){try{await context.close();report.cleanup.browser=true;}catch(error){report.cleanup.browser=false;report.cleanup.browserError=String(error);process.exitCode=1;}}
 if(broker){broker.stdin.end();let timer;const stopped=await Promise.race([brokerExit,new Promise(r=>{timer=setTimeout(()=>r(null),8000);})]);clearTimeout(timer);report.cleanup.exit=stopped;if(!stopped){broker.kill();report.cleanup.forced=true;process.exitCode=1;}}
 if(registered){try{pwsh('adapters/chromium/host/Remove-Registration.ps1',['-PackageDirectory',hostDir]);report.cleanup.registration=true;}catch(e){report.cleanup.registration=false;process.exitCode=1;}}
 await new Promise(r=>server.close(r));
 if(resolve(profile).startsWith(resolve(run)+sep)){try{await rm(profile,{recursive:true,force:true});report.cleanup.profile=true;}catch(error){report.cleanup.profile=false;report.cleanup.profileError=String(error);process.exitCode=1;}}
 report.cleanupPass=report.cleanup.browser===true&&report.cleanup.broker==='complete'&&report.cleanup.exit?.code===0&&report.cleanup.registration===true&&report.cleanup.profile===true;
 if(!report.cleanupPass)process.exitCode=1;
 await writeFile(join(run,'receipt.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({run,...report},null,2));
}
