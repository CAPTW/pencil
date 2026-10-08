import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import net from 'node:net';
assert.equal(process.platform,'win32');assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
assert.notEqual(process.env.COMPUTERNAME,(await readFile('C:/GrammarPayload/host-machine.txt','utf8')).trim());
assert.match(execFileSync('whoami.exe',['/groups','/fo','csv'],{encoding:'utf8'}),/S-1-16-8192/);
const {chromium}=createRequire(import.meta.url)('C:/GrammarPayload/node_modules/playwright');
const manifest=JSON.parse(await readFile('C:/GrammarTrial/MANIFEST.json','utf8'));
const hash=createHash('sha256').update(await readFile('C:/GrammarTrial/codex-pencil.exe')).digest('hex');
assert.equal(hash,manifest.files.find(x=>x.path==='codex-pencil.exe').sha256);
const targetHash=createHash('sha256').update(await readFile('C:/GrammarTrial/target-apply.dll')).digest('hex');assert.equal(targetHash,manifest.files.find(x=>x.path==='target-apply.dll').sha256);
const report={targetSha256:targetHash,fixtureOverrides:manifest.fixtureOverrides??null,status:'FAIL',classification:manifest.fixtureOverrides?'MODIFIED_TARGET_DLL_FIXTURE_API_CANCEL_NOT_SUCCESSFUL_API':'INSTALLED_UNMODIFIED_APP_API_SETUP_AND_LOCAL_TLS_CANCEL_NOT_SUCCESSFUL_API',sourceCommit:manifest.sourceCommit,appSha256:hash,checks:[],appExitCodes:[],liveProvider:'NOT_RUN',successfulApiReply:'NOT_RUN'};
const pass=s=>{report.checks.push(s);console.log('PASS '+s);};
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function until(name,probe,ms=15000){const end=Date.now()+ms;while(Date.now()<end){const value=await probe();if(value)return value;await sleep(100);}throw Error('timeout: '+name);}
let app,editor,browser,page;const sockets=new Set();let connections=0;
const server=net.createServer(socket=>{connections++;sockets.add(socket);socket.on('close',()=>sockets.delete(socket));socket.on('error',()=>{});socket.on('data',()=>{});});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const endpoint=`https://127.0.0.1:${server.address().port}/v1/chat/completions`;
async function launch(){
 app=spawn('C:/GrammarTrial/codex-pencil.exe',[],{windowsHide:true,stdio:'ignore',env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9481'}});
 await until('WebView',async()=>{if(app.exitCode!==null)throw Error('app exited');try{return(await fetch('http://127.0.0.1:9481/json/version')).ok;}catch{return false;}},60000);
 browser=await chromium.connectOverCDP('http://127.0.0.1:9481');
 await until('page',async()=>{page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(p.url()));return !!page;});
 await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.invoke==='function');
 await page.evaluate(()=>{window.__errors=[];const t=window.__TAURI_INTERNALS__;return t.invoke('plugin:event|listen',{event:'capture-error',target:{kind:'Any'},handler:t.transformCallback(e=>window.__errors.push(e.payload))});});
}
try{
 await launch();pass('installed app hash and real IPC');
 const settings=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('load_settings'));
 if(settings.onboardingVersion<1)await page.getByRole('button',{name:'Continue',exact:true}).click();
 await page.evaluate(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('load_settings');await window.__TAURI_INTERNALS__.invoke('save_settings',{settings:{...s,mode:'grammar',autoRewrite:false}});});
 await page.reload();await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.transformCallback==='function');
 await page.evaluate(()=>{window.__errors=[];const t=window.__TAURI_INTERNALS__;return t.invoke('plugin:event|listen',{event:'capture-error',target:{kind:'Any'},handler:t.transformCallback(e=>window.__errors.push(e.payload))});});
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 const setup=page.getByRole('region',{name:'Optional API setup'});
 await setup.getByLabel('HTTPS chat/completions endpoint').fill(endpoint);
 await setup.getByLabel('Model',{exact:true}).fill('synthetic-no-inference');
 await setup.getByLabel('API key (session only)').fill('owned-dummy-api-key');
 await setup.getByRole('checkbox').check();await setup.getByRole('button',{name:'Keep setup for this session'}).click();
 await setup.getByText('Setup kept for this app session.',{exact:false}).waitFor();
 assert.equal(await setup.getByLabel('API key (session only)').inputValue(),'');
 const api=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('api_session_status'));assert.equal(api.requestEnabled,true);assert.equal(api.endpoint,endpoint);assert.equal('key' in api,false);assert.equal(connections,0);pass('actual settings IPC saves memory setup without connection or key status');
 await setup.getByRole('button',{name:'Remove API key and permission'}).click();
 await setup.getByText('API key and permission removed from this session.').waitFor();
 assert.equal((await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('api_session_status'))).requestEnabled,false);pass('actual removal revokes API setup');
 await setup.getByLabel('API key (session only)').fill('owned-dummy-api-key');await setup.getByRole('checkbox').check();await setup.getByRole('button',{name:'Keep setup for this session'}).click();
 await setup.getByText('Setup kept for this app session.',{exact:false}).waitFor();
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 await page.evaluate(()=>{window.__captures=[];const t=window.__TAURI_INTERNALS__;return t.invoke('plugin:event|listen',{event:'selection-captured',target:{kind:'Any'},handler:t.transformCallback(e=>window.__captures.push(e.payload))});});
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
  await command('PASSWORD 0');await set(prefix+'please seperate this.'+suffix);await command(`SELECT ${prefix.length} ${prefix.length+21}`);
  console.log(execFileSync('C:/GrammarPayload/managed-target-focus.exe',[String(report.targetPid)],{windowsHide:true,encoding:'utf8',timeout:10000}));
  await until('owned target foreground',async()=>await command('FOCUS')==='FOCUS True');
  await hide();
  await until('owned target remains foreground',async()=>await command('FOCUS')==='FOCUS True');
  const count=await page.evaluate(()=>window.__captures.length);
  assert.equal(await command('HOTKEY'),'SENT 6');
  const token=await until('captured',()=>page.evaluate(i=>window.__captures[i],count));
  assert.equal(token.selectedText,'please seperate this.');assert.equal(token.directApply,true);
  await draft.waitFor({state:'visible'});return token;
 }

 await capture();pass('real shortcut captures owned synthetic source');
 const reviewed='Reviewed synthetic draft.';await draft.fill(reviewed);
 page.once('dialog',d=>d.dismiss());await page.getByTestId('run-api').click();await sleep(150);assert.equal(connections,0);assert.equal(await draft.inputValue(),reviewed);pass('declining per-request confirmation sends nothing and retains draft');
 page.once('dialog',d=>d.accept());await page.getByTestId('run-api').click();
 await until('owned TLS connection',()=>connections===1);assert.equal(await draft.inputValue(),reviewed);
 await page.getByTestId('cancel-rewrite').click();await until('cancel settles',()=>page.getByTestId('run-api').isEnabled());
 assert.equal(await draft.inputValue(),reviewed);assert.equal(await text(),'please seperate this.');
 await until('socket closed',()=>sockets.size===0);pass('explicit API request cancellation closes local connection and preserves draft/source');
 await page.getByTestId('copy-result').click();await until('copy feedback',async()=>/Copied/.test(await status.textContent()));assert.equal(await text(),'please seperate this.');assert.equal(Buffer.from((await command('CLIPBOARD')).slice(5),'base64').toString('utf8'),reviewed);pass('retained edited draft Copy exact clipboard reread without source mutation');
 assert.equal(await command('EXIT'),'EXIT 0');await until('editor exit',()=>editor.exitCode!==null);
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});await until('app exit',()=>app.exitCode!==null);assert.equal(app.exitCode,0);report.appExitCodes.push(app.exitCode);await browser.close();browser=null;
 await launch();assert.equal((await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('api_session_status'))).configured,false);pass('API setup absent after actual process restart');
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});await until('app exit',()=>app.exitCode!==null);assert.equal(app.exitCode,0);report.appExitCodes.push(app.exitCode);report.status='PASS';
}catch(error){report.error=String(error.stack??error);report.errorEvents=await page?.evaluate(()=>window.__errors).catch(()=>[]);console.error(report.error,report.errorEvents);}
finally{
 if(page&&app?.exitCode===null)await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 if(editor?.exitCode===null)editor.stdin.end();
 if(app?.exitCode===null)await until('app cleanup',()=>app.exitCode!==null,15000).catch(()=>{});
 if(editor?.exitCode===null)await until('editor cleanup',()=>editor.exitCode!==null,10000).catch(()=>{});
 if(browser)await browser.close().catch(()=>{});
 if(app?.exitCode===null){app.kill();report.forcedApp=true;report.status='FAIL';await until('forced owned app exit',()=>app.exitCode!==null||app.signalCode!==null,5000).catch(()=>{});}
 if(editor?.exitCode===null){editor.kill();report.forcedEditor=true;report.status='FAIL';}
 for(const socket of sockets)socket.destroy();await new Promise(r=>server.close(r));
 report.connections=connections;report.appExited=!!app&&(app.exitCode!==null||app.signalCode!==null);report.editorExited=!!editor&&(editor.exitCode!==null||editor.signalCode!==null);
 await writeFile('C:/GrammarOutput/app-result.json',JSON.stringify(report,null,2));
}
process.exitCode=report.status==='PASS'?0:1;
