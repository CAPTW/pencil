const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const rows=JSON.parse(fs.readFileSync(process.argv[2],'utf8')),ctx={};
vm.runInNewContext(fs.readFileSync('adapters/chromium/extension/core.js','utf8'),ctx);
let checks=0;const check=(condition)=>{assert(condition);checks++;};
for(const row of rows){
 const doc=new ctx.GrammarCore.DocumentSession(row.response.epoch);
 for(let i=1;i<row.response.revision;i++)doc.update('synthetic seed '+i);
 const request=doc.update(row.source);
 check(request.revision===row.response.revision);
 check(doc.publish(request,row.response.suggestions,100));
 check(doc.suggestion(0,101).replacement===row.expected);
 check(doc.replacement(0,row.source,row.expected,101).next===row.expected);
 check(doc.replacement(0,row.source+' changed',row.expected,101)===null);
 doc.update(row.source+' changed');check(!doc.publish(request,row.response.suggestions,101));
 doc.clear();check(doc.suggestion(0,101)===null);
}
console.log(JSON.stringify({status:'PASS',checks,scope:'NATIVE_GEMMA_RECEIPT_PRODUCTION_CACHE_REPLAY_NOT_UI'}));
