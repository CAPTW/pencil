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
const manifest=JSON.parse(await readFile('C:/PencilTrialScope/install/MANIFEST.json','utf8'));
const hashes={};for(const name of ['codex-pencil.exe','target-apply.dll']){
 hashes[name]=createHash('sha256').update(await readFile('C:/PencilTrialScope/install/'+name)).digest('hex');
 assert.equal(hashes[name],manifest.files.find(x=>x.path===name).sha256);
}
const editorKind=process.env.GRAMMAR_TEST_EDITOR_KIND??'richedit';assert.ok(['standard','multiline','richedit'].includes(editorKind));
const report={status:'FAIL',sourceCommit:manifest.sourceCommit,hashes,checks:[],providerLive:'NOT_RUN',classification:editorKind==='multiline'?'INSTALLED_APP_TARGET_SIDE_ACCEPT_OWNED_MULTILINE_AUTOVSCROLL_EDIT_ONLY':editorKind==='standard'?'INSTALLED_APP_TARGET_SIDE_ACCEPT_OWNED_SINGLELINE_EDIT_ONLY':'INSTALLED_APP_TARGET_SIDE_APPLY_OWNED_RICHEDIT_ONLY'};
const pass=name=>{report.checks.push(name);console.log('PASS '+name);};
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function until(name,probe,ms=15000){const end=Date.now()+ms;while(Date.now()<end){const result=await probe();if(result)return result;await sleep(100);}throw new Error('timeout: '+name);}
let app,editor,browser,page;let editorNatural=false,appNatural=false;
try{
 pass('installed package hashes verified');
 app=spawn('C:/PencilTrialScope/install/codex-pencil.exe',[],{windowsHide:true,stdio:'ignore',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9481'}});
 await until('WebView endpoint',async()=>{try{return (await fetch('http://127.0.0.1:9481/json/version')).ok;}catch{return false;}},90000);
 browser=await chromium.connectOverCDP('http://127.0.0.1:9481');
 page=await until('widget page',async()=>browser.contexts().flatMap(x=>x.pages()).find(x=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(x.url())),60000);
 await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.invoke==='function',null,{polling:100,timeout:60000});
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
  await until('Instant draft',async()=>await draft.inputValue().catch(()=>null)==='please separate this.');
  await until('Apply enabled',()=>apply.isEnabled());return token;
 }
 pass('installed app and owned target initialized without cloud consent');
 const first=await capture();await draft.fill('Please separate these items.');await apply.click();
 await until('Apply result',async()=>await status.textContent()==='Applied to the captured field');
 assert.equal(await text(),'Please separate these items.');pass('actual widget edited Instant Apply reread matches');
 const stale=await page.evaluate(token=>window.__TAURI_INTERNALS__.invoke('apply_replacement',{...token,replacement:'must not replace',mode:'grammar',targetLanguage:null,autoReferenceLanguage:null,instantDraft:null,directApply:true}),{sessionId:first.sessionId,generation:first.generation});
 assert.notEqual(stale.status,'applied');assert.equal(await text(),'Please separate these items.');pass('consumed token cannot replay mutation');
 await capture('앞부분 😀 | ',' | 뒤쪽');await draft.fill('검토한 교체 문장 🙂');await apply.click();
 await until('Unicode partial Accept',async()=>await status.textContent()==='Applied to the captured field');
 assert.equal(await text(),'앞부분 😀 | 검토한 교체 문장 🙂 | 뒤쪽');pass('reviewed Korean emoji replacement preserves exact unselected prefix and suffix');
 if(editorKind==='multiline'){
  await capture('첫 줄\r\n','\r\n마지막 줄');await draft.fill('검토한 첫 문장\n두 번째 문장 🙂');await apply.click();
  await until('multiline Accept',async()=>await status.textContent()==='Applied to the captured field');
  assert.equal(await text(),'첫 줄\r\n검토한 첫 문장\n두 번째 문장 🙂\r\n마지막 줄');pass('multiline reviewed replacement preserves exact surrounding lines');
  assert.equal(await command('UNDO'),'OK');assert.equal(await text(),'첫 줄\r\nplease seperate this.\r\n마지막 줄');pass('native multiline Undo restores full original source after Accept');
 }
 await capture();await set('application changed its own text');await apply.click();
 await until('source rejection',async()=>await status.textContent()==='Apply unavailable - Copy is available');
 assert.equal(await text(),'application changed its own text');pass('application source change rejected without restore');
 await page.getByTestId('copy-result').click();
 await until('rejected Apply fallback Copy',async()=>await status.textContent()==='Copied — paste it into the field');
 assert.equal(await text(),'application changed its own text');
 assert.equal(Buffer.from((await command('CLIPBOARD')).slice(5),'base64').toString('utf8'),'please separate this.');
 pass('explicit Copy after rejected Apply preserves application edit');
 await capture();await command('SELECT 0 6');await apply.click();
 await until('range rejection',async()=>await status.textContent()==='Apply unavailable - Copy is available');
 assert.equal(await text(),'please seperate this.');pass('moved selection rejected without mutation');
 await capture();await command('PASSWORD 42');await apply.click();
 await until('privacy rejection',async()=>await status.textContent()==='Apply unavailable - Copy is available');
 await command('PASSWORD 0');assert.equal(await text(),'please seperate this.');pass('privacy revocation prevents Apply');
 const dismissed=await capture();await hide();
 const revoked=await page.evaluate(token=>window.__TAURI_INTERNALS__.invoke('apply_replacement',{sessionId:token.sessionId,generation:token.generation,replacement:'must not apply after Dismiss',mode:'grammar',targetLanguage:null,autoReferenceLanguage:null,instantDraft:null,directApply:true}),dismissed);
 assert.notEqual(revoked.status,'applied');assert.equal(await text(),'please seperate this.');pass('Dismiss rejects old Apply request without mutation');
 await capture();await draft.fill('Copy only draft.');await page.getByTestId('copy-result').click();
 await until('Copy outcome',async()=>await status.textContent()==='Copied — paste it into the field');
 assert.equal(await text(),'please seperate this.');assert.equal(Buffer.from((await command('CLIPBOARD')).slice(5),'base64').toString('utf8'),'Copy only draft.');pass('explicit Copy writes clipboard and leaves target unchanged');
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 await until('app natural exit',async()=>app.exitCode!==null,15000);assert.equal(app.exitCode,0);appNatural=true;
 await until('target service unloaded before editor exit',async()=>await command('MODULES')==='MODULES 0',10000);
 pass('target DLL unloaded while target process remains alive');
 assert.equal(await command('EXIT'),'EXIT 0');await until('editor controller natural exit',async()=>editor.exitCode!==null,15000);assert.equal(editor.exitCode,0);editorNatural=true;
 pass('app target cleanup and fixture natural exit');report.status='PASS';
}catch(error){report.error=String(error.stack??error);console.error(report.error);}
finally{
 report.captureErrors=page?await page.evaluate(()=>window.__errors).catch(()=>[]):[];
 report.lastStatus=page?await page.locator('.status-pill').textContent({timeout:1000}).catch(()=>null):null;
 if(page&&app?.exitCode===null){await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});await until('cleanup app',async()=>app.exitCode!==null,15000).catch(()=>{});}
 if(editor?.exitCode===null){editor.stdin.end();await until('cleanup editor',async()=>editor.exitCode!==null,15000).catch(()=>{});}
 if(browser)await browser.close().catch(()=>{});
 if(app&&app.exitCode===null){app.kill();report.appForced=true;}
 if(editor&&editor.exitCode===null){editor.kill();report.editorForced=true;}
 report.appNatural=appNatural;report.editorNatural=editorNatural;
 await writeFile('C:/GrammarOutput/app-target-result.json',JSON.stringify(report,null,2));
}
process.exitCode=report.status==='PASS'?0:1;
