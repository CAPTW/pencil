import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
assert.equal(process.platform,'win32'); assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
assert.notEqual(process.env.COMPUTERNAME,(await readFile('C:/GrammarPayload/host-machine.txt','utf8')).trim());
assert.match(execFileSync('whoami.exe',['/groups','/fo','csv'],{encoding:'utf8'}),/S-1-16-8192/);
const {chromium}=createRequire(import.meta.url)('C:/GrammarPayload/node_modules/playwright');
const manifest=JSON.parse(await readFile('C:/GrammarTrial/MANIFEST.json','utf8'));
const hashes={};for(const name of ['codex-pencil.exe','target-apply.dll','grammar-gemma-broker.exe','Start-Grammar.ps1']){
 hashes[name]=createHash('sha256').update(await readFile('C:/GrammarTrial/'+name)).digest('hex');
 assert.equal(hashes[name],manifest.files.find(x=>x.path===name).sha256);
}
const editorKind=process.env.GRAMMAR_TEST_EDITOR_KIND??'richedit';assert.ok(['standard','multiline','richedit'].includes(editorKind));
const report={status:'FAIL',sourceCommit:manifest.sourceCommit,hashes,checks:[],providerLive:'NOT_RUN',classification:'INSTALLED_UNMODIFIED_APP_BROKER_SYNTHETIC_ASSET_CATALOG_UI_NOT_GPU_OR_APPLY'};
const pass=name=>{report.checks.push(name);console.log('PASS '+name);};
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function until(name,probe,ms=15000){const end=Date.now()+ms;while(Date.now()<end){const result=await probe();if(result)return result;await sleep(100);}throw new Error('timeout: '+name);}
let app,editor,browser,page,launcher;let editorNatural=false,appNatural=false;
try{
 pass('installed package hashes verified');
 launcher=spawn('C:/GrammarPayload/pwsh/pwsh.exe',['-NoProfile','-File','C:/GrammarTrial/Start-Grammar.ps1','-EnableDesktopGemma'],{windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9481'}});
 launcher.stdout.on('data',data=>process.stdout.write(data));launcher.stderr.on('data',data=>process.stderr.write(data));
 await until('WebView endpoint',async()=>{try{return (await fetch('http://127.0.0.1:9481/json/version')).ok;}catch{return false;}},90000);
 browser=await chromium.connectOverCDP('http://127.0.0.1:9481');
 page=await until('widget page',async()=>browser.contexts().flatMap(x=>x.pages()).find(x=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(x.url())),60000);
 await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.invoke==='function',null,{polling:100,timeout:60000});
 app={pid:Number(execFileSync('powershell.exe',['-NoProfile','-Command',"(Get-Process codex-pencil | Where-Object Path -eq 'C:\\GrammarTrial\\codex-pencil.exe').Id"],{encoding:'utf8',windowsHide:true}).trim())};
 assert.ok(Number.isInteger(app.pid)&&app.pid>0);report.appPid=app.pid;
 await until('preparing banner',()=>page.locator('.local-model-status').textContent().then(s=>s.includes('Preparing local Gemma')));
 pass('installed widget visible during model preparation');
 const settings=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('load_settings'));
 if(settings.onboardingVersion<1){await page.getByRole('button',{name:'Continue',exact:true}).click();}
 await page.evaluate(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('load_settings');for(const k of ['cloudProcessingAcknowledgementVersion','claudeCloudAcknowledgementVersion','antigravityCloudAcknowledgementVersion'])if(s[k]!==0)throw Error('unexpected cloud grant');await window.__TAURI_INTERNALS__.invoke('save_settings',{settings:{...s,mode:'grammar',autoRewrite:false}});});
 await page.reload();await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.transformCallback==='function',null,{polling:100});
 await page.evaluate(()=>{window.__captures=[];window.__errors=[];const t=window.__TAURI_INTERNALS__;return Promise.all(['selection-captured','capture-error'].map((event,i)=>t.invoke('plugin:event|listen',{event,target:{kind:'Any'},handler:t.transformCallback(e=>(i?window.__errors:window.__captures).push(e.payload))})));});
 editor=spawn('C:/GrammarPayload/target-editor.exe',[],{windowsHide:true,stdio:['pipe','pipe','pipe'],env:{...process.env,GRAMMAR_EXPECTED_APP_PID:String(app.pid)}});
 editor.on('error',error=>console.error('editor spawn error',error));editor.on('exit',(code,signal)=>console.log('editor exit',code,signal));
 editor.stderr.on('data',data=>process.stderr.write(data));const lines=[],waiters=[];
 createInterface({input:editor.stdout}).on('line',line=>{const waiter=waiters.shift();if(waiter)waiter(line);else lines.push(line);});
 const next=()=>lines.length?Promise.resolve(lines.shift()):new Promise((done,fail)=>{const timer=setTimeout(()=>fail(Error('editor reply timeout')),15000);waiters.push(line=>{clearTimeout(timer);done(line);});});
 const ready=await next();assert.match(ready,/^READY \d+$/);report.targetPid=Number(ready.split(' ')[1]);
 const command=async text=>{editor.stdin.write(text+'\n');const answer=await next();assert.ok(!answer.startsWith('ERR'),answer);return answer;};
 const text=async()=>Buffer.from((await command('READ')).slice(5),'base64').toString('utf8');
 const set=async value=>assert.equal(await command('SET '+Buffer.from(value).toString('base64')),'OK');
 const visible=()=>page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:window|is_visible',{label:'main'}));
 const hide=async()=>{if(await visible()){await page.getByRole('button',{name:'Close',exact:true}).click();await until('widget hidden',async()=>!(await visible()));}};
 const draft=page.getByLabel('Editable rewrite result text',{exact:true});const apply=page.getByTestId('apply-result');const status=page.locator('.status-pill');
 async function capture(prefix='',suffix=''){
  await hide();await command('PASSWORD 0');await set(prefix+'please seperate this.'+suffix);await command(`SELECT ${prefix.length} ${prefix.length+21}`);
  await until('owned target foreground',async()=>await command('FOCUS')==='FOCUS True');
  const count=await page.evaluate(()=>window.__captures.length);
  assert.equal(await command('HOTKEY'),'SENT 6');
  const token=await until('captured',()=>page.evaluate(i=>window.__captures[i],count));
  assert.equal(token.selectedText,'please seperate this.');assert.equal(token.directApply,true);
  await draft.waitFor({state:'visible'});return token;
 }

 const first=await capture();
 const reviewed='검토한 초안을 유지합니다. Keep this edited draft.';
 await draft.fill(reviewed);pass('synthetic capture and edited draft while preparing');
 await writeFile('C:/GrammarTrial/local/phase-control','fail');
 const retry=page.getByRole('button',{name:'Retry local model',exact:true});
 await retry.waitFor({state:'visible',timeout:20000});
 assert.equal(await draft.inputValue(),reviewed);
 const oldState=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('local_model_status'));
 assert.equal(oldState.phase,'failed');assert.ok(oldState.canRetry);pass('warmup failure leaves draft and offers explicit retry');
 await writeFile('C:/GrammarTrial/local/phase-control','ready');
 await retry.click();
 await until('ready banner',()=>page.locator('.local-model-status').textContent().then(s=>s.includes('Local Gemma ready')),20000);
 assert.equal(await draft.inputValue(),reviewed);
 const newState=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('local_model_status'));
 assert.equal(newState.phase,'ready');assert.equal(newState.canRetry,false);
 assert.equal(await text(),'please seperate this.');pass('retry becomes ready without changing draft or source');
 const userCount=await readFile('C:/GrammarTrial/local/user-requests','utf8').catch(()=> '0');
 assert.equal(userCount,'0');assert.equal(await readFile('C:/GrammarTrial/local/warmups','utf8'),'2');pass('only two warmups, no capture replay or user inference');
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 await until('launcher natural cleanup',async()=>launcher.exitCode!==null,20000);assert.equal(launcher.exitCode,0);appNatural=true;
 const cwd=await readFile('C:/GrammarTrial/local/runtime.cwd','utf8');
 const {existsSync,readdirSync}=await import('node:fs');assert.equal(existsSync(cwd),false);
 assert.equal(readdirSync('C:/GrammarTrial').filter(n=>/^gemma-session-.*\.json/.test(n)).length,0);
 pass('app exit completes runtime and session cleanup');
 assert.equal(await command('EXIT'),'EXIT 0');await until('editor natural exit',()=>editor.exitCode!==null);assert.equal(editor.exitCode,0);editorNatural=true;
 report.status='PASS';
}catch(error){report.error=String(error.stack??error);console.error(report.error);}
finally{
 if(page&&!appNatural){await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});}
 if(launcher?.exitCode===null){await until('cleanup launcher',()=>launcher.exitCode!==null,20000).catch(()=>{});}
 if(editor?.exitCode===null){editor.stdin.end();await until('cleanup editor',()=>editor.exitCode!==null,10000).catch(()=>{});}
 if(browser)await browser.close().catch(()=>{});
 if(launcher?.exitCode===null){launcher.kill();report.launcherForced=true;report.status='FAIL';}
 if(editor?.exitCode===null){editor.kill();report.editorForced=true;report.status='FAIL';}
 report.appNatural=appNatural;report.editorNatural=editorNatural;
 await writeFile('C:/GrammarOutput/app-result.json',JSON.stringify(report,null,2));
}
process.exitCode=report.status==='PASS'?0:1;
