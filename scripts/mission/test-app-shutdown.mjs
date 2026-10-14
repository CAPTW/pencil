// Windows installed-app regression: run inside owned_trial.py, with a fresh output directory.
// Arguments: package/install root, output directory, absolute Playwright module path.
// Overall 40 s observation covers the existing 8 s Provider budget plus the 30 s WebView budget.
// psutil observes only descendants recorded from this app; PID creation times reject reuse.
import {spawn,execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {mkdir,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createServer} from 'node:net';
const {chromium}=createRequire(import.meta.url)(process.argv[4]);
const root=process.argv[2], out=process.argv[3];
await mkdir(out,{recursive:true});
await mkdir(out+'/profile');
await new Promise((resolve,reject)=>{const server=createServer();server.once('error',reject);server.listen(9482,'127.0.0.1',()=>server.close(resolve));});
const app=spawn(root+'/codex-pencil.exe',[],{windowsHide:true,stdio:'inherit',env:{...process.env,GRAMMAR_PROFILE_DIRECTORY:out+'/profile',WEBVIEW2_USER_DATA_FOLDER:out+'/profile/webview',WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9482'}});
let browser;
try{
 for(let i=0;i<100;i++){try{browser=await chromium.connectOverCDP('http://127.0.0.1:9482');break;}catch{await new Promise(r=>setTimeout(r,300));}}
 assert.ok(browser);
 const page=browser.contexts()[0].pages()[0];await page.waitForFunction(()=>window.__TAURI_INTERNALS__?.invoke);
 const rows=JSON.parse(execFileSync('python',['-c','import psutil,json,sys;p=psutil.Process(int(sys.argv[1]));print(json.dumps([dict(pid=x.pid,name=x.name(),created=x.create_time()) for x in [p]+p.children(recursive=True)]))',String(app.pid)],{windowsHide:true,encoding:'utf8'}));
 await writeFile(out+'/processes.json',JSON.stringify(rows));
 const started=Date.now();
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app')).catch(()=>{});
 await browser.close();browser=null;
 const timeline=[];
 for(let i=0;i<400;i++){
  const alive=JSON.parse(execFileSync('python',['-c','import psutil,json,sys;rows=json.loads(sys.argv[1]);print(json.dumps([r for r in rows if psutil.pid_exists(r["pid"]) and psutil.Process(r["pid"]).create_time()==r["created"]]))',JSON.stringify(rows)],{windowsHide:true,encoding:'utf8'}));
  timeline.push({ms:Date.now()-started,alive});if(!alive.length)break;
  if(Date.now()-started>40000)break;await new Promise(r=>setTimeout(r,100));
 }
 const result={status:timeline.at(-1).alive.length?'FAIL':'PASS',appExit:app.exitCode,timeline};
 await writeFile(out+'/result.json',JSON.stringify(result,null,2));
 assert.equal(result.status,'PASS');
 assert.equal(app.exitCode,0,'app must confirm successful shutdown');
}finally{if(browser)await browser.close();}

