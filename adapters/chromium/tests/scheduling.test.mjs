import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

// Execute the production closure with only DOM/transport dependencies stubbed.
// Scheduling and reconciliation bodies are not duplicated by the test.
function fixture() {
 let now=0,next=1;const timers=new Map();
 const context=vm.createContext({GrammarCore:{},chrome:{runtime:{onMessage:{addListener(){}}}},document:{hidden:false},
  setTimeout(fn,ms){const id=next++;timers.set(id,{fn,at:now+ms});return id;},clearTimeout(id){timers.delete(id);}});
 const source=readFileSync(new URL('../extension/content.js',import.meta.url),'utf8');
 const seam=`
  let invalidations=0;
  session={text:'old'};read=()=> 'new';cancelDeepUI=()=>{};invalidate=()=>{invalidations++;};
  globalThis.port={schedule,reconcileChanged,compositionStart,compositionEnd,
   composing(value){composing=value;},pending:()=>timer,invalidations:()=>invalidations,
   fireWithoutSession(){session=null;analyzeChanged();},setSession(){session={text:'old'};}};
 `;
 assert.ok(source.trimEnd().endsWith('})();'));
 vm.runInContext(source.replace(/\}\)\(\);\s*$/,seam+'})();'),context);
 return {port:context.port,timers,at(value){now=value;}};
}
test('heartbeat reconciliation does not postpone an already scheduled input',()=>{
 const f=fixture();f.port.schedule();f.at(100);f.port.reconcileChanged();
 assert.equal(f.timers.size,1);assert.equal([...f.timers.values()][0].at,250);
 assert.equal(f.port.invalidations(),1);
});
test('new input still debounces from the most recent input',()=>{
 const f=fixture();f.port.schedule();f.at(100);f.port.schedule();
 assert.equal(f.timers.size,1);assert.equal([...f.timers.values()][0].at,350);
});
test('reconciliation schedules programmatic changes when no input is pending',()=>{
 const f=fixture();f.at(100);f.port.reconcileChanged();
 assert.equal([...f.timers.values()][0].at,350);
});
test('expired timer ownership clears even if the session was removed',()=>{
 const f=fixture();f.port.schedule();f.port.fireWithoutSession();assert.equal(f.port.pending(),null);
});
test('composition cannot schedule analysis',()=>{
 const f=fixture();f.port.composing(true);f.port.schedule();f.port.reconcileChanged();assert.equal(f.timers.size,0);
});
test('composition revokes the pending timer and its ownership',()=>{
 const f=fixture();f.port.schedule();f.port.compositionStart({target:null});
 assert.equal(f.timers.size,0);assert.equal(f.port.pending(),null);
 f.at(100);f.port.compositionEnd({target:null});assert.equal([...f.timers.values()][0].at,350);
});


function pipeline() {
 const sent=[],waiting=[],timers=new Map();let id=0;
 const context=vm.createContext({document:{hidden:false},
  chrome:{runtime:{onMessage:{addListener(){}},sendMessage(message){
   sent.push(message);if(message.op==='cancel-local')return Promise.resolve({ok:true});
   return new Promise(resolve=>waiting.push({message,resolve}));
  }}},setTimeout(fn){timers.set(++id,fn);return id;},clearTimeout(key){timers.delete(key);}});
 vm.runInContext(readFileSync(new URL('../extension/core.js',import.meta.url),'utf8'),context);
 const source=readFileSync(new URL('../extension/content.js',import.meta.url),'utf8');
 const seam=`
  let value='head\\none\\ntwo\\ntail',renders=0;
  epoch='doc';session=new DocumentSession(epoch);editor={};status={textContent:''};
  read=()=>value;invalidate=()=>{};render=()=>{renders++;};
  globalThis.pipeline={flush:analyzeChanged,
   edit(next){value=next;input({target:editor,isTrusted:true});},
   cache:()=>session.cache,lookup:i=>session.suggestion(i),renders:()=>renders,
   revision:()=>session.revision};
 `;
 vm.runInContext(source.replace(/\}\)\(\);\s*$/,seam+'})();'),context);
 return {p:context.pipeline,sent,waiting};
}
const settle=()=>new Promise(resolve=>setImmediate(resolve));
test('rapid edits coalesce unpublished lines; stale response never publishes; cache lookup sends nothing',async()=>{
 const f=pipeline();f.p.flush();
 let req=f.waiting.shift();req.resolve({...req.message,suggestions:[]});await settle();
 f.p.edit('head\nONE\ntwo\ntail');f.p.flush();const old=f.waiting.shift();
 f.p.edit('head\nONE\nTWO\ntail');f.p.flush();
 assert.equal(f.waiting.length,0,'only one physical request at a time');
 old.resolve({...old.message,suggestions:[{start:0,end:3,replacement:'OLD',rule:'bad'}]});await settle();
 assert.equal(f.p.cache().length,0);req=f.waiting.shift();
 assert.equal(req.message.text,'ONE\nTWO');
 req.resolve({...req.message,suggestions:[{start:4,end:7,replacement:'two',rule:'grammar'}]});await settle();
 assert.equal(f.p.cache().length,1);assert.equal(f.p.lookup(0).start,9);
 const count=f.sent.length;f.p.lookup(0);f.p.lookup(0);assert.equal(f.sent.length,count);
 assert.equal(f.sent.filter(x=>x.op==='analyze').length,3);
});
