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
