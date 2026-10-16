// Actual installed launcher/broker/Gemma + installed rHWP extension. Guest only.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {requestAppQuit} from './request-app-quit.mjs';
assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
assert.notEqual(process.env.COMPUTERNAME,(await readFile('C:/GrammarPayload/host-machine.txt','utf8')).trim());
assert.equal(process.env.GRAMMAR_PROFILE_DIRECTORY,'C:\\PencilTrialScope\\profile');
const root='C:/PencilTrialScope',install=root+'/install';
const {chromium}=createRequire(import.meta.url)('C:/GrammarPayload/node_modules/playwright');
const manifest=JSON.parse(await readFile(install+'/MANIFEST.json','utf8'));
const assets=JSON.parse(await readFile(install+'/Gemma-Assets.json','utf8'));
const installation=JSON.parse(await readFile(install+'/install-receipt.json','utf8'));
assert.match(installation.nativeHost.name,/^org\.grammar\.personal\.t[0-9a-f]{32}$/);
const extensionHashes=new Map();
for(const entry of manifest.files.filter(f=>f.path.startsWith('extension/'))){
 const bytes=await readFile(install+'/'+entry.path);
 const hash=createHash('sha256').update(bytes).digest('hex');
 if(entry.path==='extension/host-config.js')assert.equal(bytes.toString('utf8'),'export const HOST = '+JSON.stringify(installation.nativeHost.name)+';\n');
 else assert.equal(hash,entry.sha256,entry.path+' packaged bytes');
 extensionHashes.set(entry.path,hash);
}

const report={status:'FAIL',nativeSource:manifest.sourceCommit,extensionId:manifest.extensionId,model:assets.model,headless:false,classification:'OFFLINE_SANDBOX_ACTUAL_GEMMA_INSTALLED_NATIVE_MESSAGING',providerLive:'NOT_RUN'};
const until=async(label,fn,ms)=>{const end=Date.now()+ms;while(Date.now()<end){if(await fn())return;await new Promise(r=>setTimeout(r,200));}throw Error('Timeout: '+label);};
let launcher,browser,page,probe;
try{
 const start=performance.now();
 launcher=spawn('C:/GrammarPayload/pwsh/pwsh.exe',['-NoProfile','-File',install+'/Start-Grammar.ps1','-ProfileDirectory',process.env.GRAMMAR_PROFILE_DIRECTORY,'-RuntimeDirectory',install+'/local/runtime','-ModelPath',install+'/local/'+assets.model.name],{windowsHide:true,stdio:'inherit',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9481'}});
 await until('WebView',async()=>{assert.equal(launcher.exitCode,null);try{return(await fetch('http://127.0.0.1:9481/json/version')).ok;}catch{return false;}},60000);
 browser=await chromium.connectOverCDP('http://127.0.0.1:9481');
 await until('app page',()=>{page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(p.url()));return !!page;},30000);
 await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__);
 await until('real model ready',async()=>{report.lastModelStatus=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('local_model_status'));if(report.lastModelStatus.phase==='failed')throw Error('Local model failed: '+JSON.stringify(report.lastModelStatus));return report.lastModelStatus.phase==='ready';},300000);
 report.readinessMs=performance.now()-start;
 await writeFile('C:/GrammarOutput/model-ready.json',JSON.stringify(report,null,2));
 probe=spawn('C:/GrammarPayload/node.exe',['C:/GrammarPayload/test-rhwp-installed.mjs','C:/GrammarPayload/rhwp-config.json'],{windowsHide:true,stdio:'inherit',env:{...process.env,GRAMMAR_INSTALL_ROOT:install}});
 await until('rHWP actual model flow',()=>probe.exitCode!==null,240000);assert.equal(probe.exitCode,0);
 report.probe=JSON.parse(await readFile(root+'/installed-receipt.json','utf8'));
 assert.equal(report.probe.status,'PASS');const config=JSON.parse(await readFile('C:/GrammarPayload/rhwp-config.json','utf8'));assert.equal(report.probe.samples.length,config.expectedSamples??6);
 for(const [path,hash] of extensionHashes){assert.equal(createHash('sha256').update(await readFile(install+'/'+path)).digest('hex'),hash,path+' changed after installation');}
 report.extensionFilesUnchanged=true;report.extensionFileCount=extensionHashes.size;report.registeredHost=installation.nativeHost.name;report.status='PASS';
}catch(e){report.error=String(e.stack??e);}
finally{
 if(probe?.exitCode===null){probe.kill();report.forcedProbe=true;report.status='FAIL';await until('probe exit',()=>probe.exitCode!==null,10000).catch(()=>{});}
 if(page)report.quit=await requestAppQuit(page).catch(e=>({error:String(e)}));
 if(launcher)await until('natural app exit',()=>launcher.exitCode!==null,45000).catch(()=>{});
 report.launcherExitCode=launcher?.exitCode;report.appNatural=launcher?.exitCode===0;if(!report.appNatural)report.status='FAIL';
 if(browser)await browser.close().catch(()=>{});
 await writeFile('C:/GrammarOutput/app-result.json',JSON.stringify(report,null,2));
}
process.exitCode=report.status==='PASS'?0:1;
