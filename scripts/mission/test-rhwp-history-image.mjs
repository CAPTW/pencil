import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {historyImage} from '../../adapters/rhwp/history-image.mjs';
// Explicit owned synthetic file and pinned WASM only; never discovers user files.
const [pkg,input,out]=process.argv.slice(2);
const core=await import(pathToFileURL(pkg+'/rhwp.js'));
await core.default({module_or_path:await readFile(pkg+'/rhwp_bg.wasm')});
const hash=b=>createHash('sha256').update(b).digest('hex'),checks=[];
for(const format of ['hwp','hwpx']){
 const d=new core.HwpDocument(await readFile(input));
 try{
  d.setCaretPosition(0,1,0);const before=historyImage(d,format,hash);
  d.setCaretPosition(0,1,5);const caret=d.getStoredCaret();
  if(format==='hwp')assert.notEqual(historyImage(d,format,hash).hash,before.hash);
  assert.equal(historyImage(d,format,hash,before.caret).hash,before.hash);
  assert.equal(d.getStoredCaret(),caret);checks.push(format+' caret-only save metadata normalized and restored');
  const changed=JSON.parse(d.replaceText(0,1,0,1,'X'));assert.notEqual(changed.ok,false);const editedCaret=d.getStoredCaret();
  assert.notEqual(historyImage(d,format,hash,before.caret).hash,before.hash);
  assert.equal(d.getStoredCaret(),editedCaret);checks.push(format+' real text edit remains refused');
 }finally{d.free();}
}
await writeFile(out,JSON.stringify({status:'PASS',checks},null,2));console.log('PASS',checks.length);

