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
