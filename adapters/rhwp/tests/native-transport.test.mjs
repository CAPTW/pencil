import test from 'node:test';
import assert from 'node:assert/strict';
import {createNativeTransport} from '../native-transport.mjs';
const id='a'.repeat(32),request={version:1,op:'analyze',id:'capture',epoch:'epoch',revision:1,text:'Synthetic',intent:{mode:'polite'}};
function fixture(extra={}){
 const event=()=>{const callbacks=new Set();return {addListener:f=>callbacks.add(f),removeListener:f=>callbacks.delete(f),emit:v=>{for(const f of [...callbacks])f(v);},get size(){return callbacks.size;}};};
 let calls=0,disconnects=0,sent;
 const port={onMessage:event(),onDisconnect:event(),disconnect(){disconnects++;},postMessage(x){sent=x;}};
 const runtime={id,connectNative(){calls++;return port;}};
 const transport=createNativeTransport({runtime,location:{protocol:'chrome-extension:',host:id},host:'com.pencil.owned',allowed:()=>true,...extra});
 return {transport,port,get calls(){return calls;},get disconnects(){return disconnects;},get sent(){return sent;}};
}
test('web page and mismatched extension contexts are refused before connection',()=>{
 assert.throws(()=>fixture({location:{protocol:'https:',host:id}}),/CONTEXT/);
 assert.throws(()=>fixture({location:{protocol:'chrome-extension:',host:'b'.repeat(32)}}),/CONTEXT/);
});
test('local envelope binds reply and closes its actual port',async()=>{
 const f=fixture();const pending=f.transport.analyze({...request,provider:'cloud'},new AbortController().signal);
 assert.deepEqual(f.sent,request);f.port.onMessage.emit({...request,suggestions:[]});await pending;
 assert.equal(f.calls,1);assert.equal(f.disconnects,1);assert.equal(f.port.onMessage.size,0);
});
test('Deep and invalid intent cannot open a native port',async()=>{
 const f=fixture();await assert.rejects(f.transport.analyze({...request,op:'deep'}),/REQUEST/);
 await assert.rejects(f.transport.analyze({...request,intent:{mode:'translate',target:'xx'}}),/INTENT/);assert.equal(f.calls,0);
});
test('admission denial and exceptions send nothing',async()=>{
 for(const allowed of [()=>false,()=>{throw Error('unknown');}]){
  const f=fixture({allowed});await assert.rejects(f.transport.analyze(request),/NOT_ALLOWED/);assert.equal(f.calls,0);
 }
});
test('abort disconnects transport, releases slot and cannot publish late result',async()=>{
 const f=fixture(),c=new AbortController();const pending=f.transport.analyze(request,c.signal);c.abort();
 await assert.rejects(pending,/CANCELLED/);f.port.onMessage.emit(request);assert.equal(f.disconnects,1);
 const next=f.transport.analyze(request);f.port.onMessage.emit(request);await next;assert.equal(f.calls,2);
});
test('revoked permission refuses already computed response',async()=>{
 let allowed=true;const f=fixture({allowed:()=>allowed});const pending=f.transport.analyze(request);allowed=false;
 f.port.onMessage.emit(request);await assert.rejects(pending,/NOT_ALLOWED/);assert.equal(f.disconnects,1);
});
test('wrong capture response and disconnected host fail without replay',async()=>{
 const f=fixture();let pending=f.transport.analyze(request);f.port.onMessage.emit({...request,id:'other'});
 await assert.rejects(pending,/RESPONSE/);pending=f.transport.analyze(request);f.port.onDisconnect.emit();
 await assert.rejects(pending,/DISCONNECTED/);assert.equal(f.calls,2);
});
test('one request only and terminal close cancels pending work',async()=>{
 const f=fixture();const pending=f.transport.analyze(request);await assert.rejects(f.transport.analyze(request),/BUSY/);
 f.transport.close();await assert.rejects(pending,/CANCELLED/);await assert.rejects(f.transport.analyze(request),/NOT_ALLOWED/);
 assert.equal(f.calls,1);
});
test('deadline closes native connection without automatic retry',async()=>{
 const f=fixture({timeoutMs:5});await assert.rejects(f.transport.analyze(request),/TIMEOUT/);assert.equal(f.calls,1);assert.equal(f.disconnects,1);
});

test('internal alignment requires explicit mode and a bounded alignment envelope',async()=>{const f=fixture();await assert.rejects(f.transport.analyze({...request,intent:{mode:'style_align'}}),/INTENT/);assert.equal(f.calls,0);const r={...request,text:'<pencil-align>\n<original>source</original><result>fixed</result></pencil-align>',intent:{mode:'style_align'}};const pending=f.transport.analyze(r);assert.deepEqual(f.sent,r);f.port.onMessage.emit({...r,suggestions:[]});await pending;assert.equal(f.calls,1);});
