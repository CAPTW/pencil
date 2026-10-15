import test from 'node:test';
import assert from 'node:assert/strict';
import {mountReviewPanel} from '../review-panel.mjs';

function fixture(analyze) {
 const elements=[];let confirmed=false,disabled=0;
 const doc={defaultView:{confirm:()=>confirmed},createElement(tag){
  const e={tag,value:'',children:[],events:{},attrs:{},append(...x){this.children.push(...x);},setAttribute(k,v){this.attrs[k]=v;},addEventListener(k,v){this.events[k]=v;},remove(){}};
  elements.push(e);return e;
 }};
 const session={enable(){},disable(){disabled++;},capture(){return {id:'capture',source:'Original'};},valid(){return true;},close(){},accept(){throw Error('unexpected mutation');}};
 analyze.supportsWriting=true;
 mountReviewPanel({ownerDocument:doc,append(){}},session,analyze);
 return {get:label=>elements.find(e=>e.textContent===label||e.attrs['aria-label']===label),confirm:()=>{confirmed=true;},disabled:()=>disabled};
}

test('mode change preserves declined draft and invalidates accepted draft without inference',async()=>{
 let calls=0;const f=fixture(async()=>{calls++;return 'Suggested';});
 await f.get('Capture selection').events.click();
 const mode=f.get('Local writing mode'),draft=f.get('Reviewed replacement');
 draft.value='My edit';draft.events.input();mode.value='ko';mode.events.change();
 assert.equal(mode.value,'grammar');assert.equal(draft.value,'My edit');assert.equal(f.disabled(),0);
 f.confirm();mode.value='ko';mode.events.change();
 assert.equal(draft.value,'');assert.equal(f.get('Accept').disabled,true);assert.equal(f.disabled(),1);assert.equal(calls,1);
});

test('changing mode aborts publication of pending old mode result',async()=>{
 let finish,signal;const f=fixture(async(_capture,s)=>{signal=s;return await new Promise(r=>{finish=r;});});
 const pending=f.get('Capture selection').events.click();
 const mode=f.get('Local writing mode');mode.value='polite';mode.events.change();
 assert.equal(signal.aborted,true);finish('Old result');await pending;
 assert.equal(f.get('Reviewed replacement').value,'');assert.equal(f.get('Accept').disabled,true);
});
