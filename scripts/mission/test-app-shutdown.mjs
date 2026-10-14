// Windows installed-app regression: run inside owned_trial.py, with a fresh output directory.
// Arguments: package/install root, output directory, absolute Playwright module path.
// Overall 40 s observation covers the existing 8 s Provider budget plus the 30 s WebView budget.
// One bounded observer process polls handles; never spawn a process per sample.
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
const launched=Date.now();
const app=spawn(root+'/codex-pencil.exe',[],{windowsHide:true,stdio:'inherit',env:{...process.env,GRAMMAR_PROFILE_DIRECTORY:out+'/profile',WEBVIEW2_USER_DATA_FOLDER:out+'/profile/webview',WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9482'}});
let browser;
try{
 for(let i=0;i<100;i++){try{browser=await chromium.connectOverCDP('http://127.0.0.1:9482');break;}catch{await new Promise(r=>setTimeout(r,300));}}
 assert.ok(browser);
 const pages=browser.contexts().flatMap(c=>c.pages());
 await writeFile(out+'/pages-at-connect.json',JSON.stringify(pages.map(p=>p.url())));
 let page;
 for(let i=0;i<900;i++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(p.url()));if(page)break;await new Promise(r=>setTimeout(r,100));}
 assert.ok(page,'owned Tauri page unavailable');
 page.on('pageerror',error=>console.log('OWNED_PAGE_ERROR '+error.message));
 try{await page.waitForFunction(()=>window.__TAURI_INTERNALS__?.invoke);}
 catch(error){await writeFile(out+'/page-failure.json',JSON.stringify({url:page.url(),state:await page.evaluate(()=>({ready:document.readyState,internal:typeof window.__TAURI_INTERNALS__,scriptCount:document.scripts.length})).catch(()=>null)}));throw error;}

 await writeFile(out+'/startup.json',JSON.stringify({readyMs:Date.now()-launched,url:page.url()}));
 const rows=JSON.parse(execFileSync('python',['-c','import psutil,json,sys;p=psutil.Process(int(sys.argv[1]));print(json.dumps([dict(pid=x.pid,name=x.name(),created=x.create_time()) for x in [p]+p.children(recursive=True)]))',String(app.pid)],{windowsHide:true,encoding:'utf8'}));
 await writeFile(out+'/processes.json',JSON.stringify(rows));
 const started=Date.now();
 const quit=await page.evaluate(async()=>{try{await window.__TAURI_INTERNALS__.invoke('quit_app');return {accepted:true};}catch(e){return {accepted:false,error:String(e)};}}).catch(e=>({transportError:e.message}));
 await writeFile(out+'/quit.json',JSON.stringify(quit));
 assert.equal(quit.accepted,true,'Quit command acknowledgement required');
 const observerStarted=Date.now()-started;
 const timeline=JSON.parse(execFileSync('python',['-c', `import psutil,json,sys,time
rows=json.loads(sys.argv[1]); start=time.monotonic(); timeline=[]
while True:
 alive=[]
 for row in rows:
  try:
   if psutil.Process(row['pid']).create_time()==row['created']: alive.append(row)
  except psutil.NoSuchProcess: pass
 elapsed=round((time.monotonic()-start)*1000)
 timeline.append(dict(ms=elapsed,alive=alive))
 if not alive or elapsed>=40000: break
 time.sleep(.1)
print(json.dumps(timeline))`,JSON.stringify(rows)],{windowsHide:true,encoding:'utf8',timeout:45000,maxBuffer:1024*1024}));
 for(const row of timeline)row.ms+=observerStarted;
 // Deliver Node's child exit event after the single blocking observer completes.
 await new Promise(r=>setTimeout(r,100));
 const result={status:timeline.at(-1).alive.length?'FAIL':'PASS',appExit:app.exitCode,shutdownObservationMs:Date.now()-started,timeline};
 await writeFile(out+'/result.json',JSON.stringify(result,null,2));
 assert.equal(result.status,'PASS');
 assert.equal(app.exitCode,0,'app must confirm successful shutdown');
}finally{if(browser)await browser.close();}

