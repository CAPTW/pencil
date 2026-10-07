import {test} from 'node:test';
import assert from 'node:assert/strict';
import {summarizeCorpus} from './gemma-corpus-browser.mjs';
test('exact output cannot mask a subsequent Apply reread failure',()=>{
 const result=summarizeCorpus([{id:'one',qualityPass:true,error:'reread mismatch',recommendMs:400,applyMs:20}],[{id:'one'}]);
 assert.equal(result.qualityPass,false);
});
test('partial and unavailable results are not complete coverage',()=>{
 const result=summarizeCorpus([{id:'one',error:'transport unavailable',recommendMs:1200}],[{id:'one'},{id:'two'}]);
 assert.equal(result.qualityPass,false);assert.deepEqual(result.notRun,['two']);assert.deepEqual(result.recommendationMisses,['one']);
});
test('quality and latency remain separately visible',()=>{
 const result=summarizeCorpus([{id:'one',qualityPass:true,recommendMs:1001,applyMs:501}],[{id:'one'}]);
 assert.equal(result.qualityPass,true);assert.deepEqual(result.recommendationMisses,['one']);assert.deepEqual(result.applyMisses,['one']);
});
