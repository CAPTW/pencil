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
function fixture() {
  const sent=[],ports=[],timers=new Map();let nextTimer=0,epoch=0,storageGate=null,injectionGate=null,writeGate=null,stored={deniedOrigins:[]};
  const injectionGates=new Map(),writes=[];
  const onMessage=event(),onUpdated=event(),onRemoved=event();
  const chrome={
    storage:{local:{async get(){const gate=storageGate;storageGate=null;return gate?gate.promise:structuredClone(stored);},
      async set(value){writes.push(structuredClone(value));const gate=writeGate;writeGate=null;if(gate)await gate.promise;stored=structuredClone(value);}}},
    runtime:{id:'extension',getURL:path=>'chrome-extension://extension/'+path,onMessage,
      connectNative(name){assert.equal(name,'org.grammar.test');const port={onMessage:event(),onDisconnect:event(),posted:[],closed:0,
        postMessage(message){this.posted.push(message);},disconnect(){this.closed++;this.onDisconnect.emit();}};ports.push(port);return port;}},
    tabs:{onUpdated,onRemoved,async get(tabId){return {url:'https://example.test/document/'+tabId};},
      async sendMessage(tabId,message,options){sent.push({tabId,message,options});return {ready:true};}},
    scripting:{async executeScript({target}){const gate=injectionGates.get(target.tabId)||injectionGate;injectionGates.delete(target.tabId);injectionGate=null;return gate?gate.promise:[{frameId:0,documentId:'doc-'+target.tabId}];}},
  };
  const context=vm.createContext({chrome,URL,crypto:{randomUUID:()=>`epoch-${++epoch}`},
    setTimeout:fn=>{const id=++nextTimer;timers.set(id,fn);return id;},clearTimeout:id=>timers.delete(id)});
  new vm.Script(source,{filename:'production-worker.js'}).runInContext(context);
  const popup={id:'extension',url:'chrome-extension://extension/popup.html'};
  const dispatch=(message,sender)=>new Promise(resolve=>onMessage.emit(message,sender,resolve));
  const command=(op,tabId=1)=>dispatch({op,tabId},popup);
  const sender=(tabId=1)=>({id:'extension',url:'https://example.test/document/'+tabId,tab:{id:tabId},frameId:0,documentId:'doc-'+tabId});
  const currentEpoch=(tabId=1)=>sent.filter(x=>x.tabId===tabId&&x.message.op==='enable').at(-1)?.message.epoch;
  const request=(id='r1',tabId=1)=>({version:1,op:'analyze',id,epoch:currentEpoch(tabId),revision:1,text:'seperate'});
  return {sent,ports,timers,writes,command,dispatch,sender,request,currentEpoch,onUpdated,onRemoved,
    delayStorage(){return storageGate=deferred();},delayWrite(){return writeGate=deferred();},
    delayInjection(tabId){if(tabId!==undefined){const gate=deferred();injectionGates.set(tabId,gate);return gate;}return injectionGate=deferred();},
    fireTimer(){const [id,fn]=timers.entries().next().value;timers.delete(id);fn();}};
}

test('revoke during awaited denylist lookup sends zero native messages',async()=>{
  const f=fixture();await f.command('enable');const gate=f.delayStorage();
  const pending=f.dispatch(f.request(),f.sender());await tick();await f.command('disable');
  gate.resolve({deniedOrigins:[]});assert.equal((await pending).error,'permission_revoked');assert.equal(f.ports.length,0);
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
  for(const patch of [{text:'x'.repeat(8193)},{id:'x'.repeat(129)},{revision:0},{revision:1.5},{revision:Number.MAX_SAFE_INTEGER+1},{version:2},{op:'deep'}]){
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
