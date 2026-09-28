import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

// Only the configuration import is substituted. All routing, async guards,
// permissions, framing decisions and port callbacks execute production worker.js.
const original = readFileSync(new URL('../extension/worker.js', import.meta.url), 'utf8');
const source = original.replace("import {HOST} from './host-config.js';", "const HOST = 'org.grammar.test';");
assert.notEqual(source, original, 'mock loader must replace exactly the host config import');
const deferred = () => { let resolve,reject; const promise = new Promise((r,j) => {resolve=r;reject=j;}); return {promise,resolve,reject}; };
const tick = () => new Promise(resolve => setImmediate(resolve));
function event() { const listeners=[];return {addListener(fn){listeners.push(fn);},emit(...args){for(const fn of listeners)fn(...args);}}; }
function fixture(initial={deniedOrigins:[]}) {
  const sent=[],ports=[],controlPorts=[],timers=new Map();let nextTimer=0,epoch=0,storageGate=null,injectionGate=null,writeGate=null,stored=structuredClone(initial);
  const injectionGates=new Map(),writes=[];let controlAuto=true, enableReplyGate=null;
  const ticket=token=>({installation:'a'.repeat(32),slot:0,generation:1,token});
  const onMessage=event(),onUpdated=event(),onRemoved=event();
  const chrome={
    storage:{local:{async get(){const gate=storageGate;storageGate=null;return gate?gate.promise:structuredClone(stored);},
      async set(value){writes.push(structuredClone(value));const gate=writeGate;writeGate=null;if(gate)await gate.promise;stored={...stored,...structuredClone(value)};}}},
    runtime:{id:'extension',getURL:path=>'chrome-extension://extension/'+path,onMessage,
      connectNative(name){assert.equal(name,'org.grammar.test');const port={onMessage:event(),onDisconnect:event(),posted:[],closed:0,
        postMessage(message){this.posted.push(message);
          if(message.op.startsWith('cleanup-')) {
            if(ports.includes(this))ports.splice(ports.indexOf(this),1);
            if(!controlPorts.includes(this))controlPorts.push(this);
            if(controlAuto)queueMicrotask(()=>this.onMessage.emit({...message,
              cleanup_ticket:message.cleanup_ticket || ticket(message.cleanup_token),
              ...(message.op==='cleanup-reserve'?{}:{cleanup_complete:true})}));
          }
        },disconnect(){this.closed++;this.onDisconnect.emit();}};ports.push(port);return port;}},
    tabs:{onUpdated,onRemoved,async get(tabId){return {url:'https://example.test/document/'+tabId};},
      async sendMessage(tabId,message,options){sent.push({tabId,message,options});if(message.op==='enable' && enableReplyGate){const gate=enableReplyGate;enableReplyGate=null;return gate.promise;}return {ready:true};}},
    scripting:{async executeScript({target}){const gate=injectionGates.get(target.tabId)||injectionGate;injectionGates.delete(target.tabId);injectionGate=null;return gate?gate.promise:[{frameId:0,documentId:'doc-'+target.tabId}];}},
  };
  const context=vm.createContext({chrome,URL,crypto:{randomUUID:()=>`00000000-0000-4000-8000-${String(++epoch).padStart(12,'0')}`},
    setTimeout:fn=>{const id=++nextTimer;timers.set(id,fn);return id;},clearTimeout:id=>timers.delete(id)});
  new vm.Script(source,{filename:'production-worker.js'}).runInContext(context);
  const popup={id:'extension',url:'chrome-extension://extension/popup.html'};
  const dispatch=(message,sender)=>new Promise(resolve=>onMessage.emit(message,sender,resolve));
  const command=(op,tabId=1,extra={})=>dispatch({op,tabId,...extra},popup);
  const sender=(tabId=1)=>({id:'extension',url:'https://example.test/document/'+tabId,tab:{id:tabId},frameId:0,documentId:'doc-'+tabId});
  const currentEpoch=(tabId=1)=>sent.filter(x=>x.tabId===tabId&&x.message.op==='enable').at(-1)?.message.epoch;
  const request=(id='r1',tabId=1)=>({version:1,op:'analyze',id,epoch:currentEpoch(tabId),revision:1,text:'seperate'});
  return {sent,ports,controlPorts,ticket,controlAuto(value){controlAuto=value;},stored(){return structuredClone(stored);},timers,writes,command,dispatch,sender,request,currentEpoch,onUpdated,onRemoved,
    delayEnableReply(){return enableReplyGate=deferred();},
    delayStorage(){return storageGate=deferred();},delayWrite(){return writeGate=deferred();},
    delayInjection(tabId){if(tabId!==undefined){const gate=deferred();injectionGates.set(tabId,gate);return gate;}return injectionGate=deferred();},
    fireTimer(){const [id,fn]=timers.entries().next().value;timers.delete(id);fn();}};
}

test('revoke during awaited denylist lookup sends zero native messages',async()=>{
  const f=fixture();await f.command('enable');const gate=f.delayStorage();
  const pending=f.dispatch(f.request(),f.sender());await tick();await f.command('disable');
  gate.resolve({deniedOrigins:[]});assert.equal((await pending).error,'permission_revoked');assert.equal(f.ports.length,0);
});

for(const revoke of ['cancel','regrant'])test(`${revoke} during pre-admission storage cannot revive Deep`,async()=>{
  const f=fixture();await f.command('enable');await f.command('deep-consent',1,{consent:true,provider:'claude'});
  const gate=f.delayStorage(),request={...f.request(),op:'deep',provider:'claude'};
  const pending=f.dispatch(request,f.sender());await tick();
  if(revoke==='cancel')await f.dispatch({op:'cancel-deep',epoch:request.epoch,id:request.id},f.sender());
  else await f.command('deep-consent',1,{consent:true,provider:'claude'});
  gate.resolve({deniedOrigins:[]});assert.equal((await pending).error,'deep_permission_denied');
  assert.equal(f.ports.length,0);assert.equal(f.writes.length,0);
});

for(const revoke of ['disable','stop'])test(`late enable after ${revoke} cannot activate document`,async()=>{
  const f=fixture(),gate=f.delayInjection();const pending=f.command('enable');await tick();
  await f.command(revoke);gate.resolve([{frameId:0,documentId:'doc-1'}]);
  assert.equal((await pending).status,'Enable cancelled');
  assert.equal(f.sent.filter(x=>x.message.op==='enable').length,0);assert.equal(f.ports.length,0);
});

test('navigation revokes an enable waiting on storage',async()=>{
  const f=fixture(),gate=f.delayStorage();const pending=f.command('enable');await tick();
  f.onUpdated.emit(1,{status:'loading'});gate.resolve({deniedOrigins:[]});
  assert.equal((await pending).status,'Enable cancelled');assert.equal(f.currentEpoch(),undefined);
});

test('sender, frame, origin, epoch and missing document identity fail before native request',async()=>{
  const f=fixture();await f.command('enable');
  for(const patch of [{id:'other'},{frameId:1},{url:'https://other.test/'},{documentId:''},{documentId:undefined}]){
    assert.equal((await f.dispatch(f.request(),{...f.sender(),...patch})).error,'permission_denied');
  }
  assert.equal((await f.dispatch({...f.request(),epoch:'wrong'},f.sender())).error,'permission_denied');
  assert.equal((await f.dispatch(f.request(),{...f.sender(),documentId:'new-document'})).error,'document_changed');
  assert.equal((await f.dispatch(f.request(),f.sender())).error,'permission_denied');assert.equal(f.ports.length,0);
});

test('oversized text, id and invalid revision are rejected; exactly one request may be in flight',async()=>{
  const f=fixture();await f.command('enable');
  for(const patch of [{text:'x'.repeat(8193)},{id:'x'.repeat(129)},{revision:0},{revision:1.5},{revision:Number.MAX_SAFE_INTEGER+1},{version:2},{op:'unsupported'}]){
    assert.equal((await f.dispatch({...f.request(),...patch},f.sender())).error,'invalid_request');
  }
  assert.equal(f.ports.length,0);
  const pending=f.dispatch({...f.request(),text:'x'.repeat(8192)},f.sender());await tick();
  assert.equal(f.ports.length,1);assert.equal(f.ports[0].posted.length,1);
  assert.equal((await f.dispatch(f.request('second'),f.sender())).error,'busy');
  await f.command('disable');assert.equal((await pending).error,'disabled');assert.equal(f.timers.size,0);
});

test('old native port callbacks cannot complete or disconnect a newer request',async()=>{
  const f=fixture();await f.command('enable');const first=f.dispatch(f.request('first'),f.sender());await tick();
  const old=f.ports[0];f.fireTimer();assert.equal((await first).error,'native_timeout');
  let settled=false;const second=f.dispatch(f.request('second'),f.sender()).then(value=>{settled=true;return value;});await tick();
  const current=f.ports[1];assert.ok(current);
  old.onMessage.emit({id:'second',epoch:f.currentEpoch(),revision:1,suggestions:[]});old.onDisconnect.emit();await tick();
  assert.equal(settled,false);assert.equal(current.closed,0);
  current.onMessage.emit({id:'second',epoch:f.currentEpoch(),revision:1,suggestions:[]});
  assert.equal((await second).id,'second');assert.equal(f.timers.size,0);
});

test('wrong native response closes the port and pending request fails closed',async()=>{
  const f=fixture();await f.command('enable');const pending=f.dispatch(f.request(),f.sender());await tick();
  f.ports[0].onMessage.emit({id:'forged',epoch:f.currentEpoch()});
  assert.equal((await pending).error,'invalid_native_response');assert.equal(f.ports[0].closed,1);
});

test('document cap and emergency teardown bound native ownership',async()=>{
  const f=fixture();for(let id=1;id<=4;id++)assert.match((await f.command('enable',id)).status,/Document enabled/);
  assert.match((await f.command('enable',5)).status,/limit 4/);
  const pending=f.dispatch(f.request(),f.sender());await tick();await f.command('stop');
  assert.equal((await pending).error,'disabled');assert.equal(f.ports[0].closed,1);
  for(let id=1;id<=4;id++)assert.equal((await f.dispatch(f.request('after',id),f.sender(id))).error,'permission_denied');
});


test('domain deny invalidates an enable already waiting on injection',async()=>{
  const f=fixture(),gate=f.delayInjection(1);const enable=f.command('enable');await tick();
  assert.match((await f.command('deny')).status,/^Domain blocked/);
  gate.resolve([{frameId:0,documentId:'doc-1'}]);
  assert.equal((await enable).status,'Enable cancelled');
  assert.equal(f.sent.filter(x=>x.message.op==='enable').length,0);assert.equal(f.ports.length,0);
  assert.match((await f.command('enable')).status,/blocked/);
});

test('new enable while policy storage write waits is rejected before document activation',async()=>{
  const f=fixture(),gate=f.delayWrite();const deny=f.command('deny');await tick();
  assert.equal(f.writes.length,1);
  const enable=await f.command('enable',2);
  assert.match(enable.status,/busy|progress|changing/i);
  assert.equal(f.sent.filter(x=>x.message.op==='enable').length,0);
  gate.resolve();assert.match((await deny).status,/^Domain blocked/);assert.equal(f.ports.length,0);
});

test('concurrent domain policy commands fail busy instead of losing stored deny entries',async()=>{
  const f=fixture(),gate=f.delayWrite();const deny=f.command('deny');await tick();
  assert.match((await f.command('allow')).status,/busy|progress|changing/i);
  assert.match((await f.command('deny',2)).status,/busy|progress|changing/i);
  assert.equal(f.writes.length,1);gate.resolve();await deny;
  assert.match((await f.command('enable')).status,/blocked/);
  assert.match((await f.command('allow')).status,/removed/);
  assert.equal(f.writes.length,2);assert.deepEqual(f.writes[1].deniedOrigins,[]);
  assert.match((await f.command('enable')).status,/Document enabled/);
});

test('concurrent enables recheck the four-document cap after asynchronous injection',async()=>{
  const f=fixture(),gates=[],pending=[];
  for(let id=1;id<=5;id++){gates.push(f.delayInjection(id));pending.push(f.command('enable',id));}
  await tick();
  for(let id=1;id<=5;id++)gates[id-1].resolve([{frameId:0,documentId:'doc-'+id}]);
  const results=await Promise.all(pending);
  assert.equal(results.filter(value=>value.status.startsWith('Document enabled')).length,4);
  assert.equal(results.filter(value=>value.status.includes('limit 4')).length,1);
  assert.equal(f.sent.filter(x=>x.message.op==='enable').length,4);assert.equal(f.ports.length,0);
});

test('failed policy write releases busy latch and keeps all prior sessions revoked',async()=>{
  const f=fixture();await f.command('enable');const gate=f.delayWrite();const deny=f.command('deny');await tick();
  assert.equal((await f.dispatch(f.request(),f.sender())).error,'permission_denied');
  gate.reject(Error('synthetic storage failure'));assert.equal((await deny).error,'unavailable');
  assert.match((await f.command('enable')).status,/Document enabled/);assert.equal(f.ports.length,0);
});


test('Deep requires popup consent and selected provider; no native call before consent',async()=>{
 const f=fixture();await f.command('enable');const deep={...f.request(),op:'deep',provider:'claude'};
 assert.equal((await f.dispatch(deep,f.sender())).error,'deep_permission_denied');assert.equal(f.ports.length,0);
 await f.command('deep-consent',1,{provider:'claude',consent:true});
 assert.equal((await f.dispatch({...deep,provider:'codex'},f.sender())).error,'deep_permission_denied');
 const pending=f.dispatch(deep,f.sender());await tick();assert.equal(f.ports[0].posted.length,1);assert.equal(f.ports[0].posted[0].provider,'claude');assert.equal(f.ports[0].posted[0].consent,true);
 f.ports[0].onMessage.emit({id:deep.id,epoch:deep.epoch,revision:1,cleanup_complete:true,provider:'claude',replacement:'synthetic'});assert.equal((await pending).replacement,'synthetic');
});
test('disable Deep sends cancel and holds native port until actual cleanup receipt',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const deep={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(deep,f.sender());await tick();const port=f.ports[0];
 await f.command('disable');assert.equal((await pending).error,'disabled');assert.equal(port.closed,0);assert.equal(port.posted.at(-1).op,'cancel');
 port.onMessage.emit({id:deep.id,epoch:deep.epoch,revision:1,error:'cancelled',cleanup_complete:true});await tick();assert.equal(port.closed,1);assert.deepEqual(f.writes.at(-1).deepCleanupPending,[]);
});
test('missing cleanup receipt latches Deep admission and does not kill host as cancellation',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const deep={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(deep,f.sender());await tick();const port=f.ports[0];
 f.fireTimer();assert.equal(port.posted.at(-1).op,'cancel');assert.equal(port.closed,0);
 f.fireTimer();assert.equal((await pending).error,'cleanup_unconfirmed');assert.equal(port.closed,0);
 assert.match((await f.command('deep-consent',1,{provider:'codex',consent:true})).status,/unavailable/);
 port.onMessage.emit({id:deep.id,epoch:deep.epoch,revision:1,error:'cancelled',cleanup_complete:true});
});
test('Deep revoke cancels in flight and cannot silently replay',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'antigravity',consent:true});
 const deep={...f.request(),op:'deep',provider:'antigravity'},pending=f.dispatch(deep,f.sender());await tick();
 await f.command('deep-revoke');const port=f.ports[0];assert.equal(port.posted.length,2);assert.equal(port.posted[1].op,'cancel');
 port.onMessage.emit({id:deep.id,epoch:deep.epoch,revision:1,error:'cancelled',cleanup_complete:true});assert.equal((await pending).error,'cancelled');
 assert.equal((await f.dispatch(deep,f.sender())).error,'deep_permission_denied');assert.equal(port.posted.length,2);
});


test('persisted unfinished Deep blocks new admission after worker restart without document text',async()=>{
 const f=fixture({deniedOrigins:[],deepCleanupPending:['opaque-owned-marker']});await f.command('enable');
 assert.match((await f.command('deep-consent',1,{provider:'claude',consent:true})).status,/unavailable/);
 assert.equal((await f.dispatch({...f.request(),op:'deep',provider:'claude'},f.sender())).error,'deep_permission_denied');assert.equal(f.ports.length,0);
});
test('marker is persisted before send; wrong revision cleanup cannot erase ownership',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const deep={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(deep,f.sender());await tick();const port=f.ports[0];
 assert.equal(f.writes.at(-1).deepCleanupPending.length,1);assert.equal(JSON.stringify(f.writes).includes('seperate'),false);
 port.onMessage.emit({id:deep.id,epoch:deep.epoch,revision:2,error:'cancelled',cleanup_complete:true});await tick();
 assert.equal((await pending).error,'cleanup_unconfirmed');assert.equal(f.writes.at(-1).deepCleanupPending.length,1);assert.equal(port.closed,0);
 port.onMessage.emit({id:deep.id,epoch:deep.epoch,revision:1,error:'cancelled',cleanup_complete:true});await tick();assert.deepEqual(f.writes.at(-1).deepCleanupPending,[]);
});


test('field cancellation during marker persistence prevents all Provider work',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const gate=f.delayWrite(),deep={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(deep,f.sender());await tick();
 await f.dispatch({op:'cancel-deep',id:deep.id,epoch:deep.epoch},f.sender());gate.resolve();
 assert.equal((await pending).error,'permission_revoked');assert.equal(f.ports.length,0);assert.deepEqual(f.writes.at(-1).deepCleanupPending,[]);
});
test('same-provider regrant cannot revive old asynchronous admission',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const gate=f.delayWrite(),deep={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(deep,f.sender());await tick();
 await f.command('deep-revoke');await f.command('deep-consent',1,{provider:'claude',consent:true});gate.resolve();
 assert.equal((await pending).error,'permission_revoked');assert.equal(f.ports.length,0);
});
test('disabled admissions retain document capacity until marker cleanup completes',async()=>{
 const f=fixture(),gate=f.delayWrite(),pending=[];
 for(let id=1;id<=4;id++) {
   await f.command('enable',id);await f.command('deep-consent',id,{provider:'claude',consent:true});
   pending.push(f.dispatch({...f.request('d'+id,id),op:'deep',provider:'claude'},f.sender(id)));await tick();await f.command('disable',id);
 }
 assert.match((await f.command('enable',5)).status,/limit 4/);gate.resolve();
 for(const result of await Promise.all(pending))assert.equal(result.error,'permission_revoked');
 assert.equal(f.ports.length,0);assert.deepEqual(f.writes.at(-1).deepCleanupPending,[]);
 assert.match((await f.command('enable',5)).status,/Document enabled/);
});


test('idle Instant port disconnect cannot release a stalled Deep admission slot',async()=>{
 const f=fixture(),gate=f.delayWrite(),pending=[];
 for(let id=1;id<=4;id++) {
   await f.command('enable',id);const instant=f.request('i'+id,id),local=f.dispatch(instant,f.sender(id));await tick();
   f.ports.at(-1).onMessage.emit({id:instant.id,epoch:instant.epoch,revision:1,suggestions:[]});await local;
   await f.command('deep-consent',id,{provider:'claude',consent:true});
   pending.push(f.dispatch({...f.request('d'+id,id),op:'deep',provider:'claude'},f.sender(id)));await tick();await f.command('disable',id);
 }
 assert.match((await f.command('enable',5)).status,/limit 4/);gate.resolve();await Promise.all(pending);
 assert.ok(f.ports.every(port=>port.posted.length===1 && port.posted[0].op==='analyze'));
 assert.match((await f.command('enable',5)).status,/Document enabled/);
});


const recoveredTicket={installation:'a'.repeat(32),slot:2,generation:7,token:'10000000-0000-4000-8000-000000000001'};
function recoveryFixture(phase='pending') {return fixture({deepCleanupPending:[{ticket:recoveredTicket,phase}]});}
function replyControl(f,patch={}) {const port=f.controlPorts.at(-1);port.onMessage.emit({...port.posted[0],cleanup_complete:true,...patch});}

test('restart recovery is explicit, content-free, and grants no document consent',async()=>{
 const f=recoveryFixture();await tick();assert.equal(f.controlPorts.length,0);
 assert.match((await f.command('cleanup-recover')).status,/Cleanup confirmed/);
 assert.deepEqual(f.controlPorts.map(p=>p.posted[0].op),['cleanup-query','cleanup-ack']);
 for(const port of f.controlPorts) {
   assert.equal(port.posted[0].text,'');assert.equal(port.posted[0].provider,undefined);
   assert.equal(port.posted[0].epoch,'cleanup-control');assert.equal(port.closed,1);
 }
 assert.deepEqual(f.stored().deepCleanupPending,[]);assert.equal(f.ports.length,0);
 await f.command('enable');assert.equal((await f.dispatch({...f.request(),op:'deep',provider:'claude'},f.sender())).error,'deep_permission_denied');
 assert.match((await f.command('deep-consent',1,{provider:'claude',consent:true})).status,/Deep allowed/);
});

for(const patch of [{cleanup_complete:false,error:'cleanup_unknown'},{cleanup_complete:undefined},
 {cleanup_ticket:{...recoveredTicket,generation:8}},{revision:2},{id:'old-id'}])test(`unconfirmed or mismatched receipt retains restart ownership ${JSON.stringify(patch)}`,async()=>{
 const f=recoveryFixture();f.controlAuto(false);const recovery=f.command('cleanup-recover');await tick();replyControl(f,patch);
 assert.match((await recovery).status,/unconfirmed/);assert.equal(f.controlPorts.length,1);
 assert.equal(f.stored().deepCleanupPending[0].phase,'pending');await f.command('enable');
 assert.match((await f.command('deep-consent',1,{provider:'claude',consent:true})).status,/unavailable/);
});

test('ack phase is durable before native ack and survives lost ack response',async()=>{
 const f=recoveryFixture();f.controlAuto(false);const recovery=f.command('cleanup-recover');await tick();replyControl(f);await tick();
 assert.equal(f.stored().deepCleanupPending[0].phase,'ack');assert.equal(f.controlPorts.at(-1).posted[0].op,'cleanup-ack');
 f.fireTimer();assert.match((await recovery).status,/unconfirmed/);
 const restarted=fixture(f.stored());assert.match((await restarted.command('cleanup-recover')).status,/Cleanup confirmed/);
 assert.deepEqual(restarted.controlPorts.map(p=>p.posted[0].op),['cleanup-ack']);assert.deepEqual(restarted.stored().deepCleanupPending,[]);
});

test('failed ack-phase persistence cannot release native receipt or erase marker',async()=>{
 const f=recoveryFixture();f.controlAuto(false);const recovery=f.command('cleanup-recover');await tick();const gate=f.delayWrite();
 replyControl(f);await tick();gate.reject(Error('synthetic storage failure'));
 assert.match((await recovery).status,/failed/);assert.equal(f.controlPorts.length,1);assert.equal(f.stored().deepCleanupPending[0].phase,'pending');
 assert.match((await f.command('cleanup-recover')).status,/records unavailable/);
});

test('legacy or corrupt ownership is never inferred complete',async()=>{
 for(const entries of [['opaque-old-marker'],[{ticket:{...recoveredTicket,generation:0},phase:'pending'}],Array(5).fill({ticket:recoveredTicket,phase:'pending'})]) {
  const f=fixture({deepCleanupPending:entries});assert.match((await f.command('cleanup-recover')).status,/blocked/);
  assert.equal(f.controlPorts.length,0);assert.deepEqual(f.stored().deepCleanupPending,entries);
 }
});

test('concurrent recovery cannot exceed one bounded native control operation',async()=>{
 const f=recoveryFixture();f.controlAuto(false);const first=f.command('cleanup-recover');await tick();
 assert.match((await f.command('cleanup-recover')).status,/busy/);assert.equal(f.controlPorts.length,1);
 replyControl(f);await tick();replyControl(f);assert.match((await first).status,/confirmed/);
});

test('cancelled reservation is retired before any text is sent',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 f.controlAuto(false);const request={...f.request(),op:'deep',provider:'claude'};
 const pending=f.dispatch(request,f.sender());await tick();const reserve=f.controlPorts[0].posted[0];
 await f.command('disable');replyControl(f,{cleanup_ticket:f.ticket(reserve.cleanup_token)});await tick();
 assert.equal(f.controlPorts.at(-1).posted[0].op,'cleanup-query');replyControl(f);await tick();replyControl(f);
 assert.equal((await pending).error,'permission_revoked');assert.equal(f.ports.length,0);assert.deepEqual(f.stored().deepCleanupPending,[]);
});


test('lost reservation reply remains discoverable from durable intent after restart',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});f.controlAuto(false);
 const pending=f.dispatch({...f.request(),op:'deep',provider:'claude'},f.sender());await tick();
 const token=f.controlPorts[0].posted[0].cleanup_token;
 assert.deepEqual(f.stored().deepCleanupPending,[{token,phase:'reserving'}]);f.fireTimer();assert.equal((await pending).error,'cleanup_unconfirmed');
 const restarted=fixture(f.stored());assert.match((await restarted.command('cleanup-recover')).status,/confirmed/);
 assert.deepEqual(restarted.controlPorts.map(p=>p.posted[0].op),['cleanup-find','cleanup-query','cleanup-ack']);assert.deepEqual(restarted.stored().deepCleanupPending,[]);
});

test('intent with no exact native record remains blocked, never absent-means-complete',async()=>{
 const f=fixture({deepCleanupPending:[{token:recoveredTicket.token,phase:'reserving'}]});f.controlAuto(false);
 const recovery=f.command('cleanup-recover');await tick();replyControl(f,{error:'cleanup_unknown',cleanup_ticket:undefined});
 assert.match((await recovery).status,/unconfirmed/);assert.equal(f.stored().deepCleanupPending.length,1);
});

test('slot cannot be reused until completed ack marker removal is durable',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const deep={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(deep,f.sender());await tick();f.controlAuto(false);
 f.ports[0].onMessage.emit({id:deep.id,epoch:deep.epoch,revision:1,provider:'claude',cleanup_complete:true});await tick();
 const gate=f.delayWrite();replyControl(f);await tick(); // ack succeeded, browser erase deliberately stalled
 await f.command('enable',2);await f.command('deep-consent',2,{provider:'claude',consent:true});
 const second=f.dispatch({...f.request('second',2),op:'deep',provider:'claude'},f.sender(2));await tick();
 assert.equal(f.controlPorts.filter(p=>p.posted[0].op==='cleanup-reserve').length,1);
 gate.resolve();await pending;await tick();assert.equal(f.controlPorts.at(-1).posted[0].op,'cleanup-reserve');
 await f.command('disable',2);const reserve=f.controlPorts.at(-1).posted[0];replyControl(f,{cleanup_ticket:f.ticket(reserve.cleanup_token)});await tick();replyControl(f);await tick();replyControl(f);await second;
});

test('closed failed admission releases document capacity only after exact recovery',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});f.controlAuto(false);
 const pending=f.dispatch({...f.request(),op:'deep',provider:'claude'},f.sender());await tick();await f.command('disable');f.fireTimer();await pending;
 for(let id=2;id<=4;id++)await f.command('enable',id);
 assert.match((await f.command('enable',5)).status,/limit 4/);
 f.controlAuto(true);assert.match((await f.command('cleanup-recover')).status,/confirmed/);
 assert.match((await f.command('enable',5)).status,/Document enabled/);
});


test('closed queued admission rejected before reservation leaves no markerless retired owner',async()=>{
 const f=fixture();for(let id=1;id<=2;id++){await f.command('enable',id);await f.command('deep-consent',id,{provider:'claude',consent:true});}
 const first={...f.request(),op:'deep',provider:'claude'},one=f.dispatch(first,f.sender());await tick();f.controlAuto(false);
 f.ports[0].onMessage.emit({id:first.id,epoch:first.epoch,revision:1,provider:'claude',cleanup_complete:true});await tick();
 const two=f.dispatch({...f.request('second',2),op:'deep',provider:'claude'},f.sender(2));await tick();await f.command('disable',2);
 replyControl(f,{error:'cleanup_busy',cleanup_complete:false});await one;await two;
 assert.equal(f.stored().deepCleanupPending.length,1);f.controlAuto(true);await f.command('cleanup-recover');
 for(let id=3;id<=5;id++)assert.match((await f.command('enable',id)).status,/Document enabled/);
});


test('native ticket extra content is rejected before durable receipt persistence or inference',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});f.controlAuto(false);
 const pending=f.dispatch({...f.request(),op:'deep',provider:'claude'},f.sender());await tick();const token=f.controlPorts[0].posted[0].cleanup_token;
 replyControl(f,{cleanup_ticket:{...f.ticket(token),source:'must never be persisted'}});
 assert.equal((await pending).error,'cleanup_unconfirmed');assert.equal(JSON.stringify(f.writes).includes('must never'),false);assert.equal(f.ports.length,0);
});


test('disable during receipt acknowledgement leaves no stale cancellation timer',async()=>{
 const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
 const request={...f.request(),op:'deep',provider:'claude'},pending=f.dispatch(request,f.sender());await tick();f.controlAuto(false);
 f.ports[0].onMessage.emit({id:request.id,epoch:request.epoch,revision:1,provider:'claude',cleanup_complete:true});await tick();
 await f.command('disable');assert.equal((await pending).error,'disabled');replyControl(f);await tick();
 assert.equal(f.timers.size,0);assert.deepEqual(f.stored().deepCleanupPending,[]);assert.equal(f.ports[0].closed,1);
});


test('failed enable delivery releases capacity and content permission',async()=>{
  const f=fixture();
  for(let id=1;id<=4;id++) {
    const gate=f.delayEnableReply(),pending=f.command('enable',id);await tick();
    gate.reject(Error('document receiver gone'));await pending;
    assert.equal((await f.dispatch({op:'heartbeat',epoch:f.currentEpoch(id)},f.sender(id))).error,'permission_denied');
  }
  assert.match((await f.command('enable',5)).status,/Document enabled/);
});

test('late rejected enable cannot revoke a newer session',async()=>{
  const f=fixture(),gate=f.delayEnableReply(),pending=f.command('enable');await tick();
  const old=f.currentEpoch();await f.command('enable');const current=f.currentEpoch();assert.notEqual(current,old);
  gate.reject(Error('old receiver gone'));await pending;
  assert.equal((await f.dispatch({op:'heartbeat',epoch:current},f.sender())).ok,true);
  const disable=f.sent.find(x=>x.message.op==='disable' && x.message.epoch===old);
  assert.ok(disable);assert.equal(disable.options.documentId,'doc-1');
});

// Pre-use review: after "Check completed cleanup" says a new grant is needed,
// the grant from before the failure must no longer admit Deep.
test('cleanup recovery revokes every Deep grant from before the failure',async()=>{
  const f=fixture();await f.command('enable');
  assert.match((await f.command('deep-consent',1,{provider:'claude',consent:true})).status,/Deep allowed/);
  const deep={...f.request(),op:'deep',provider:'claude'};
  const pending=f.dispatch(deep,f.sender());await tick();await tick();
  const port=f.ports[0];assert.equal(port.posted.at(-1).op,'deep');
  port.onDisconnect.emit();await tick();await pending;
  assert.equal((await f.dispatch({...deep,id:'blocked'},f.sender())).error,'deep_permission_denied');
  assert.match((await f.command('cleanup-recover')).status,/Grant Deep permission again to send/);
  assert.ok(f.sent.some(x=>x.tabId===1 && x.message.op==='deep-policy' && x.message.provider===null),'the page is told the grant is gone');
  const before=f.ports.length;
  assert.equal((await f.dispatch({...deep,id:'after-recovery'},f.sender())).error,'deep_permission_denied');
  assert.equal(f.ports.slice(before).some(p=>p.posted.some(m=>m.op==='deep')),false,'no Deep request without a new grant');
  assert.match((await f.command('deep-consent',1,{provider:'claude',consent:true})).status,/Deep allowed/);
});

// Pre-use review: cleanup_busy on reserve means the host recorded nothing
// (another host process held the ledger lock). It must not block Deep until a
// manual recovery that can never succeed.
test('cleanup_busy on reserve records nothing and leaves Deep usable',async()=>{
  const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
  f.controlAuto(false);
  const deep={...f.request(),op:'deep',provider:'claude'};
  const pending=f.dispatch(deep,f.sender());
  for(let i=0;i<10 && !f.controlPorts.length;i++)await tick();
  const reserve=f.controlPorts.at(-1);assert.equal(reserve.posted[0].op,'cleanup-reserve');
  reserve.onMessage.emit({...reserve.posted[0],error:'cleanup_busy',cleanup_complete:false});
  assert.equal((await pending).error,'busy');
  assert.equal((f.stored().deepCleanupPending ?? []).length,0,'no reservation intent is left behind');
  f.controlAuto(true);
  const before=f.controlPorts.length;
  void f.dispatch({...deep,id:'r2'},f.sender());
  for(let i=0;i<20 && !f.ports.some(p=>p.posted.some(m=>m.op==='deep' && m.id==='r2'));i++)await tick();
  assert.ok(f.controlPorts.length>before,'the next explicit send reserves again');
  assert.ok(f.ports.some(p=>p.posted.some(m=>m.op==='deep' && m.id==='r2')),'and is admitted');
});

// Review finding: a document closed while its reserve was answered
// cleanup_busy stayed in the retiring set, so it used one of the four document
// slots until the worker restarted.
test('a document closed during a busy reserve frees its slot',async()=>{
  const f=fixture();await f.command('enable');await f.command('deep-consent',1,{provider:'claude',consent:true});
  f.controlAuto(false);
  const pending=f.dispatch({...f.request(),op:'deep',provider:'claude'},f.sender());
  for(let i=0;i<10 && !f.controlPorts.length;i++)await tick();
  const reserve=f.controlPorts.at(-1);assert.equal(reserve.posted[0].op,'cleanup-reserve');
  await f.command('disable');
  reserve.onMessage.emit({...reserve.posted[0],error:'cleanup_busy',cleanup_complete:false});
  assert.equal((await pending).error,'busy');
  f.controlAuto(true);
  for (const tab of [2,3,4,5]) assert.doesNotMatch((await f.command('enable',tab)).status,/limit/i,`tab ${tab} can be enabled`);
});
