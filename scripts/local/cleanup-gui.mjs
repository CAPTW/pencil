// Owned offline Sandbox only. Uses the installed app, never host user profiles.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
assert.notEqual(process.env.COMPUTERNAME,(await readFile('C:/GrammarPayload/host-machine.txt','utf8')).trim());
assert.equal(process.env.GRAMMAR_PROFILE_DIRECTORY,'C:\\PencilTrialScope\\profile');
assert.match(execFileSync('whoami.exe',['/groups','/fo','csv'],{encoding:'utf8',windowsHide:true}),/S-1-16-8192/);
const {chromium}=createRequire(import.meta.url)('C:/GrammarPayload/node_modules/playwright');
const root='C:/PencilTrialScope/install';
const manifest=JSON.parse(await readFile(root+'/MANIFEST.json','utf8'));
const sha=createHash('sha256').update(await readFile(root+'/codex-pencil.exe')).digest('hex');
assert.equal(sha,manifest.files.find(x=>x.path==='codex-pencil.exe').sha256);
const report={status:'FAIL',nativeSource:manifest.sourceCommit,appSha256:sha,checks:[],modelInference:'NOT_RUN'};
const wait=async(label,fn,ms=60000)=>{const end=Date.now()+ms;while(Date.now()<end){if(await fn())return;await new Promise(r=>setTimeout(r,200));}throw Error('Timeout: '+label);};
let launcher,browser,page;
try{
 launcher=spawn('C:/GrammarPayload/pwsh/pwsh.exe',['-NoProfile','-File',root+'/Start-Grammar.ps1','-WithoutLocalModel','-ProfileDirectory',process.env.GRAMMAR_PROFILE_DIRECTORY],{windowsHide:true,stdio:'inherit',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9482'}});
 await wait('installed WebView',async()=>{if(launcher.exitCode!==null)throw Error('Launcher exited '+launcher.exitCode);try{return (await fetch('http://127.0.0.1:9482/json/version')).ok;}catch{return false;}});
 browser=await chromium.connectOverCDP('http://127.0.0.1:9482');
 await wait('product page',()=>{page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(p.url()));return !!page;});
 await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.invoke==='function');
 assert.ok((await page.locator('body').innerText()).trim().length>0);
 report.checks.push('installed GUI rendered through real WebView');
 const settings=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('load_settings'));
 await page.evaluate(s=>window.__TAURI_INTERNALS__.invoke('save_settings',{settings:{...s,autoRewrite:false}}),settings);
 const saved=JSON.parse(await readFile('C:/PencilTrialScope/profile/settings.json','utf8'));
 assert.equal(saved.autoRewrite,false);
 report.checks.push('settings persisted only to explicit owned profile');
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 await wait('natural launcher exit',()=>launcher.exitCode!==null,45000);
 assert.equal(launcher.exitCode,0);
 report.checks.push('product quit and launcher natural cleanup');
 report.status='PASS';
}catch(e){report.error=String(e.stack??e);}
finally{
 if(page&&launcher?.exitCode===null)await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 if(launcher?.exitCode===null)await wait('cleanup',()=>launcher.exitCode!==null,45000).catch(()=>{});
 if(browser)await browser.close().catch(()=>{});
 if(launcher?.exitCode===null){launcher.kill();report.forced=true;report.status='FAIL';}
 await writeFile('C:/GrammarOutput/gui-result.json',JSON.stringify(report,null,2));
}
process.exitCode=report.status==='PASS'?0:1;
