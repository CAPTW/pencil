import test from 'node:test';
import assert from 'node:assert/strict';
import '../../chromium/extension/core.js';
import {createInstantDraft} from '../instant-draft.mjs';
const capture={id:'owned-capture',source:'😀 seperate'};
const response=r=>({...r,suggestions:[{start:3,end:11,replacement:'separate',rule:'spelling'}]});
const signal=()=>new AbortController().signal;
test('selected UTF16 input and validated cached draft',async()=>{
 let calls=0;
 const run=createInstantDraft(async r=>{calls++;assert.equal(r.op,'analyze');assert.equal(r.text,capture.source);return response(r);},GrammarCore);
 assert.equal(await run(capture,signal()),'😀 separate');assert.equal(calls,1);
});
test('wrong capture identity refuses a late reply',async()=>{
 const run=createInstantDraft(async r=>({...response(r),id:'another'}),GrammarCore);
 await assert.rejects(run(capture,signal()),/INSTANT_REPLY_REFUSED/);
});
test('split surrogate suggestion cannot corrupt draft',async()=>{
 const run=createInstantDraft(async r=>({...response(r),suggestions:[{start:1,end:2,replacement:'x',rule:'bad'}]}),GrammarCore);
 assert.equal(await run(capture,signal()),capture.source);
});
test('one in-flight operation and cancelled result never publishes',async()=>{
 let finish,calls=0;
 const run=createInstantDraft(r=>{calls++;return new Promise(ok=>finish=()=>ok(response(r)));},GrammarCore);
 const cancel=new AbortController(),pending=run(capture,cancel.signal);
 await assert.rejects(run(capture,signal()),/INSTANT_BUSY/);cancel.abort();finish();
 await assert.rejects(pending,/INSTANT_CANCELLED/);assert.equal(calls,1);
});
test('failure is not retried and slot can be reused',async()=>{
 let calls=0;const run=createInstantDraft(async r=>{if(++calls===1)throw new Error('offline');return response(r);},GrammarCore);
 await assert.rejects(run(capture,signal()),/offline/);assert.equal(calls,1);
 assert.equal(await run(capture,signal()),'😀 separate');assert.equal(calls,2);
});
test('already cancelled capture sends no request',async()=>{
 let calls=0;const run=createInstantDraft(async r=>{calls++;return response(r);},GrammarCore);
 const c=new AbortController();c.abort();await assert.rejects(run(capture,c.signal),/INSTANT_CANCELLED/);assert.equal(calls,0);
});

test('writing mode requires explicit capability and exact response intent',async()=>{
 let calls=0;
 const analyze=async r=>{calls++;return response(r);};
 await assert.rejects(createInstantDraft(analyze,GrammarCore)(capture,signal(),{mode:'translate',target:'ko'}),/INTENT_REFUSED/);
 assert.equal(calls,0);
 const run=createInstantDraft(analyze,GrammarCore,{writing:true});
 assert.equal(await run(capture,signal(),{mode:'translate',target:'ko'}),'😀 separate');
 const wrong=createInstantDraft(async r=>({...response(r),intent:{mode:'translate',target:'en'}}),GrammarCore,{writing:true});
 await assert.rejects(wrong(capture,signal(),{mode:'translate',target:'ko'}),/INTENT_REPLY_REFUSED/);
});

test('unsupported modes and extra fields never dispatch',async()=>{
 let calls=0;const run=createInstantDraft(async r=>{calls++;return response(r);},GrammarCore,{writing:true});
 for(const intent of [{mode:'provider'},{mode:'translate',target:'unknown'},{mode:'polite',provider:'codex'},[]])await assert.rejects(run(capture,signal(),intent),/INTENT_REFUSED/);
 assert.equal(calls,0);
});

test('caller mutation cannot change in-flight mode binding',async()=>{
 let finish;const run=createInstantDraft(r=>new Promise(ok=>{finish=()=>ok(response(r));}),GrammarCore,{writing:true});
 const intent={mode:'translate',target:'ko'},pending=run(capture,signal(),intent);
 intent.target='en';finish();assert.equal(await pending,'😀 separate');
});


test('rich writing uses plain context then exact style alignment, never outside text',async()=>{
 const rich={id:'rich',source:'<p0><r0>re</r0><r1>check</r1></p0>',richParagraphs:[{text:'private outside text',runs:[{id:0,text:'re'},{id:1,text:'check'}]}]};let calls=0;
 const run=createInstantDraft(async r=>{calls++;assert.equal(JSON.stringify(r).includes('private outside'),false);if(calls===1)assert.equal(r.text,'recheck');else assert.match(r.text,/\"fixed\":\"check again\"/);return {...r,suggestions:[{start:0,end:r.text.length,replacement:calls===1?'check again':'<p0><r0>again</r0></p0>',rule:'rewrite'}]};},GrammarCore,{writing:true});
 assert.deepEqual(await run(rich,signal(),{mode:'paraphrase'}),{text:'check \nagain',runOrder:[[1,0]]});assert.equal(calls,2);
});
test('style alignment cannot change the fluent result or silently retry',async()=>{
 const c={id:'rich',source:'<p0><r0>source</r0><r1>x</r1></p0>',richParagraphs:[{runs:[{id:0,text:'source'},{id:1,text:'x'}]}]};let calls=0;
 const run=createInstantDraft(async r=>({...r,suggestions:[{start:0,end:r.text.length,replacement:++calls===1?'result':'<p0><r1>different</r1></p0>',rule:'rewrite'}]}),GrammarCore,{writing:true});
 await assert.rejects(run(c,signal(),{mode:'polite'}),/RICH_ALIGNMENT_CHANGED_TEXT/);assert.equal(calls,2);
});

test('cancelled rich writing never dispatches alignment or replays the first stage',async()=>{const c={id:'rich',source:'<p0><r0>source</r0></p0>',richParagraphs:[{runs:[{id:0,text:'source'}]}]},abort=new AbortController();let calls=0;const run=createInstantDraft(async r=>{calls++;abort.abort();return {...r,suggestions:[]};},GrammarCore,{writing:true});await assert.rejects(run(c,abort.signal,{mode:'polite'}),/CANCELLED/);assert.equal(calls,1);});


test('repeated styled source refuses before inference instead of marking the wrong occurrence',async()=>{
 for(const runs of [
  [{id:0,text:'Do not check; '},{id:1,text:'check'},{id:0,text:' after approval.'}],
  [{id:1,text:'check'},{id:0,text:' before approval; do not check later.'}],
 ]) {
  const c={id:'repeat',source:'unused bound envelope',richParagraphs:[{runs}]};let calls=0;
  const run=createInstantDraft(async()=>{calls++;throw new Error('must not infer');},GrammarCore,{writing:true});
  await assert.rejects(run(c,signal(),{mode:'polite'}),/RICH_REPEATED_STYLE_UNQUALIFIED/);
  assert.equal(calls,0);
 }
});
