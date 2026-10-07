import test from 'node:test';
import assert from 'node:assert/strict';
import {createDocumentAdmission} from '../document-admission.mjs';
const model=()=>({documentGeneration:1,hasLoadedDocument:()=>true,
  requiresPasswordForSave:false,getSourceFormat:()=>'hwp',
  getTextRange(){throw new Error('TEXT_READ_FORBIDDEN');},exportDocument(){throw new Error('EXPORT_FORBIDDEN');}});
test('default host admission denies; metadata does not grant permission',()=>{
  assert.equal(createDocumentAdmission(model()).allowed(),false);
});
test('supported loaded plaintext and explicit host policy permit metadata guard',()=>{
  for(const format of ['hwp','hwpx']){const w=model();w.getSourceFormat=()=>format;assert.equal(createDocumentAdmission(w,()=>true).allowed(),true);}
});
test('password protection revokes and clearing save flag cannot revive grant',()=>{
  const w=model(),a=createDocumentAdmission(w,()=>true);assert.equal(a.allowed(),true);
  w.requiresPasswordForSave=true;assert.equal(a.allowed(),false);
  w.requiresPasswordForSave=false;assert.equal(a.allowed(),false);
  w.documentGeneration++;assert.equal(a.allowed(),true);
});
test('host revocation sticks for same generation',()=>{
  let allowed=true;const a=createDocumentAdmission(model(),()=>allowed);
  assert.equal(a.allowed(),true);allowed=false;assert.equal(a.allowed(),false);
  allowed=true;assert.equal(a.allowed(),false);
});
test('unloaded or unsupported documents refuse before host policy',()=>{
  for(const change of [w=>w.hasLoadedDocument=()=>false,w=>w.getSourceFormat=()=>'pdf']){
    const w=model();change(w);let calls=0;assert.equal(createDocumentAdmission(w,()=>{calls++;return true;}).allowed(),false);assert.equal(calls,0);
  }
});
test('unknown protection and metadata exceptions fail closed',()=>{
  for(const change of [w=>delete w.requiresPasswordForSave,w=>w.getSourceFormat=()=>{throw Error('metadata');}]){
    const w=model();change(w);assert.equal(createDocumentAdmission(w,()=>true).allowed(),false);
  }
});
test('metadata failure latches denial until new generation',()=>{
  const w=model(),a=createDocumentAdmission(w,()=>true);assert.equal(a.allowed(),true);
  w.getSourceFormat=()=>{throw Error('metadata');};assert.equal(a.allowed(),false);
  w.getSourceFormat=()=>'hwp';assert.equal(a.allowed(),false);
});
test('unknown generation refuses; close is terminal across generations',()=>{
  const w=model();w.documentGeneration=NaN;assert.equal(createDocumentAdmission(w,()=>true).allowed(),false);
  w.documentGeneration=1;const a=createDocumentAdmission(w,()=>true);a.close();w.documentGeneration++;assert.equal(a.allowed(),false);
});

test('generation getter exception cannot escape admission',()=>{
  const w=model();Object.defineProperty(w,'documentGeneration',{get(){throw Error('unavailable');}});
  assert.equal(createDocumentAdmission(w,()=>true).allowed(),false);
});
