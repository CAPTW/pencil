// Explicit CPU diagnostic. Raw output is NOT product acceptance or safe Apply evidence.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createServer} from 'node:net';
assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
assert.notEqual(process.env.COMPUTERNAME,(await readFile('C:/GrammarPayload/host-machine.txt','utf8')).trim());
const rows=JSON.parse(await readFile('C:/GrammarPayload/real-prompts.json','utf8'));
const report={scope:'RAW_REAL_GEMMA_CPU_DIAGNOSTIC_NOT_INSTALLED_ACCEPTANCE',productDeadlineMs:15000,diagnosticDeadlineMs:120000,rows:[]};
const out='C:/GrammarOutput/raw-quality.json';const save=()=>writeFile(out,JSON.stringify(report,null,2));
const delay=ms=>new Promise(r=>setTimeout(r,ms));
const socket=createServer();await new Promise(r=>socket.listen(0,'127.0.0.1',r));const port=socket.address().port;await new Promise(r=>socket.close(r));
const token=randomBytes(32).toString('hex');const env={...process.env};for(const k of Object.keys(env))if(k.startsWith('LLAMA_'))delete env[k];env.LLAMA_API_KEY=token;
await mkdir('C:/GrammarDiagnostic',{recursive:true});
const start=performance.now();
const child=spawn('C:/GrammarAssets/runtime/llama-server.exe',['-m','C:/GrammarAssets/model.gguf','--host','127.0.0.1','--port',String(port),'--offline','--no-webui','-c','4096','-ngl','99','-np','1','--reasoning','off','--cache-ram','0'],{env,windowsHide:true,cwd:'C:/GrammarDiagnostic',stdio:'ignore'});
let exited=false;child.on('exit',()=>{exited=true;});
const url=`http://127.0.0.1:${port}`;
async function post(body){const r=await fetch(url+'/v1/chat/completions',{method:'POST',headers:{authorization:'Bearer '+token,'content-type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(120000)});assert.ok(r.ok);const s=await r.text();assert.ok(Buffer.byteLength(s)<=32768);return JSON.parse(s);}
try{
 let ready=false;while(performance.now()-start<180000){if(child.exitCode!==null)throw Error('runtime exited '+child.exitCode);try{ready=(await fetch(url+'/health',{headers:{authorization:'Bearer '+token},signal:AbortSignal.timeout(1000)})).ok;}catch{}if(ready)break;await delay(200);}assert.ok(ready,'startup deadline');
 await post({messages:[{role:'user',content:'Reply with OK.'}],temperature:0,seed:42,max_tokens:1,stream:false,cache_prompt:false});report.readyMs=performance.now()-start;await save();
 for(const row of rows){const record={...row};report.rows.push(record);const before=performance.now();
  try{const response=await post({messages:[{role:'user',content:row.prompt}],temperature:0,seed:42,max_tokens:256,stream:false,cache_prompt:false,response_format:{type:'json_object',schema:{type:'object',properties:{replacement:{type:['string','null']}},required:['replacement'],additionalProperties:false}}});record.ms=performance.now()-before;record.content=response.choices?.[0]?.message?.content;record.finishReason=response.choices?.[0]?.finish_reason;record.usage=response.usage;record.timings=response.timings;record.status='RESPONSE';}catch(e){record.ms=performance.now()-before;record.status='FAIL';record.error=e.name;await save();break;}await save();
 }
}catch(e){report.error=String(e);}finally{
 child.kill();for(let i=0;i<100&&!exited;i++)await delay(100);report.runtimeExited=exited;report.runtimeExitCode=child.exitCode;report.runtimeSignal=child.signalCode;await save();
}
