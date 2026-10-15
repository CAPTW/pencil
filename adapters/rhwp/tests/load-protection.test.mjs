import test from 'node:test';
import assert from 'node:assert/strict';
import {inspectDocument} from '../load-protection.mjs';
function hwp(flags=1){
 const b=new Uint8Array(2560),v=new DataView(b.buffer),u16=(o,n)=>v.setUint16(o,n,true),u32=(o,n)=>v.setUint32(o,n,true);
 b.set([208,207,17,224,161,177,26,225]);u16(26,3);u16(28,65534);u16(30,9);u16(32,6);u32(44,1);u32(48,1);u32(56,4096);u32(60,2);u32(64,1);u32(68,0xfffffffe);
 for(let o=76;o<512;o+=4)u32(o,0xffffffff);u32(76,0);
 for(let o=512;o<1024;o+=4)u32(o,0xffffffff);u32(512,0xfffffffd);for(let i=1;i<=3;i++)u32(512+i*4,0xfffffffe);
 const entry=(o,name,type,start,length)=>{for(let i=0;i<name.length;i++)u16(o+i*2,name.charCodeAt(i));u16(o+64,(name.length+1)*2);b[o+66]=type;u32(o+68,0xffffffff);u32(o+72,0xffffffff);u32(o+76,0xffffffff);u32(o+116,start);u32(o+120,length);};
 entry(1024,'Root Entry',5,3,256);u32(1024+76,1);entry(1152,'FileHeader',2,0,256);
 for(let i=0;i<4;i++)u32(1536+i*4,i===3?0xfffffffe:i+1);
 b.set(new TextEncoder().encode('HWP Document File'),2048);b[2048+35]=5;u32(2048+36,flags);return b;
}
test('plain compressed and uncompressed HWP5 accepted from original header',async()=>{
 for(const flags of [0,1])assert.deepEqual(await inspectDocument(hwp(flags)),{format:'hwp',protection:'plain'});
});
test('all unknown/protected bits refuse including password/distribution/DRM',async()=>{
 for(let bit=1;bit<32;bit++)await assert.rejects(inspectDocument(hwp((2**bit)>>>0)));
});
test('missing header, corrupt mini chain, old version and cycles fail closed',async()=>{
 for(const mutate of [b=>b[1152]=0,b=>new DataView(b.buffer).setUint32(1536,0,true),b=>b[2083]=3,b=>new DataView(b.buffer).setUint32(1220,1,true)]){
  const b=hwp();mutate(b);await assert.rejects(inspectDocument(b));
 }
});
test('unknown and oversized containers refuse',async()=>{
 await assert.rejects(inspectDocument(new Uint8Array(100)));await assert.rejects(inspectDocument(new Uint8Array(8*1024*1024+1)));
});

