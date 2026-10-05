// Run only through the guarded restricted-Medium Sandbox runner.
import {spawn} from 'node:child_process';
import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const root='C:/GrammarPayload/browser-host-relay',output='C:/GrammarOutput';
assert.equal(process.env.USERNAME,'WDAGUtilityAccount');
const origin='chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/';
const id='11111111-1111-4111-8111-111111111111',ticket='22222222-2222-4222-8222-222222222222';
const hostExe=root+'/grammar-chromium-host.exe',clientExe=root+'/codex-pencil.exe';
const rows=[];
const send=(child,value)=>{const body=Buffer.from(JSON.stringify(value));const header=Buffer.alloc(4);header.writeUInt32LE(body.length);child.stdin.write(Buffer.concat([header,body]));};
for(let scenario=0;scenario<6;scenario++) {
 const host=spawn(hostExe,[origin],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe']});
 let client,buffer=Buffer.alloc(0),commands=0,clientCode,hostCode,clientLog='',errors=[];
 host.stdin.on('error',()=>{});host.stderr.on('data',()=>{errors.push('unexpected_host_stderr');});
 const hostDone=new Promise(resolve=>host.on('exit',code=>{hostCode=code;resolve();}));
 let clientDone=Promise.resolve(),delayed=Promise.resolve();
 const handle=message=>{
  assert.equal(message.version,2);assert.equal(message.id,id);
  if(message.op==='review-ready') {
   assert.equal(message.sequence,0);assert.equal(commands,0);
   client=spawn(clientExe,[String(host.pid),hostExe,String(scenario)],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
   client.stdout.on('data',data=>{clientLog+=data;if(clientLog.length>4096)errors.push('client_output_limit');});client.stderr.on('data',()=>errors.push('unexpected_client_stderr'));
   clientDone=new Promise(resolve=>client.on('exit',code=>{clientCode=code;resolve();}));return;
  }
  assert.ok(Number.isFinite(message.deadline));
  assert.ok(message.deadline<=performance.timeOrigin+performance.now()+3000);
  ++commands;
  if(message.op==='review-capture') {
   assert.equal(message.sequence,1);assert.equal(message.ticket,null);assert.equal(message.text,null);
   if(scenario===3){host.stdin.end();return;}
   if(scenario===5){
    const original=message.deadline;
    delayed=new Promise((resolve,reject)=>setTimeout(()=>{
     try{assert.ok(original<=performance.timeOrigin+performance.now(),'host expiry must remain expired after delivery delay');resolve();}catch(error){reject(error);}
    },3100));
    return; // Expired command is never acted on or answered.
   }
   const reply={version:2,op:'review-result',id,sequence:scenario===1?99:1,status:2,text:'선택 😀',ticket};
   if(scenario===4)reply.provider='codex'; // Forbidden extra payload, never dispatch Provider.
   send(host,reply);
  }else if(message.op==='review-apply') {
   assert.equal(message.sequence,2);assert.equal(message.ticket,ticket);assert.equal(message.text,'검토 🙂');
   send(host,{version:2,op:'review-result',id,sequence:2,status:3,text:'',ticket:scenario===2?id:ticket});
  }else throw Error('unexpected_review_operation');
 };
 host.stdout.on('data',chunk=>{
  try {buffer=Buffer.concat([buffer,chunk]);assert.ok(buffer.length<=262148);while(buffer.length>=4){const n=buffer.readUInt32LE();assert.ok(n<=262144);if(buffer.length<n+4)break;const msg=JSON.parse(buffer.subarray(4,n+4));buffer=buffer.subarray(n+4);handle(msg);}}
  catch(error){errors.push(String(error));host.stdin.end();}
 });
 send(host,{version:2,op:'review-listen',id,epoch:'desktop-review',revision:1,text:'',clock_anchor:performance.timeOrigin+performance.now()});
 let timeout;
 try {
  await Promise.race([(async()=>{await hostDone;await clientDone;await delayed;})(),new Promise((_,reject)=>{timeout=setTimeout(()=>reject(Error('owned relay deadline')),12000);})]);
  await clientDone;console.log(JSON.stringify({scenario,commands,hostCode,clientCode,errors}));assert.deepEqual(errors,[]);assert.equal(clientCode,0);assert.equal(hostCode,scenario===0||scenario===3||scenario===5?0:1);assert.equal(buffer.length,0);
  assert.equal(commands,scenario===0||scenario===2?2:1);
  rows.push({scenario,status:'PASS',commands,hostExit:hostCode,clientExit:clientCode});
 } finally {clearTimeout(timeout);host.stdin.end();}
}
await writeFile(output+'/relay-result.json',JSON.stringify({status:'PASS',classification:'ACTUAL_NATIVE_HOST_AND_DLL_SYNTHETIC_MV3_AND_DESKTOP_PEER',providerLive:'NOT_RUN',rows,hostSha256:createHash('sha256').update(await readFile(hostExe)).digest('hex'),dllSha256:createHash('sha256').update(await readFile(root+'/target-apply.dll')).digest('hex')},null,2));
console.log(JSON.stringify({status:'PASS',cases:rows.length}));
