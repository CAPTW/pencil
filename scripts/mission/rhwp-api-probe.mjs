// Synthetic upstream API qualification only; never installed Grammar acceptance.
// No screenshots, OCR, user files, clipboard, Provider calls or external browsing.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile, writeFile, mkdir, rm, copyFile} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {resolve, join} from 'node:path';
import {createHash} from 'node:crypto';

const [rootArg, playwrightArg, chromeArg, outputArg] = process.argv.slice(2);
assert.ok(rootArg && playwrightArg && chromeArg && outputArg, 'root playwright chrome output required');
const root=resolve(rootArg), output=resolve(outputArg);
const {chromium}=createRequire(import.meta.url)(resolve(playwrightArg));
const pin=JSON.parse(await readFile(new URL('../../adapters/rhwp/upstream.json',import.meta.url),'utf8'));
const proof=JSON.parse(await readFile(join(root,'upstream-receipt.json'),'utf8'));
assert.equal(proof.sourceCommit,pin.commit);assert.equal(proof.coreIntegrity,pin.coreIntegrity);
for(const file of proof.files){
 const bytes=await readFile(join(root,'upstream',file.path));
 assert.equal(createHash('sha256').update(bytes).digest('hex'),file.sha256,file.path);
}
assert.equal('sha512-'+createHash('sha512').update(await readFile(join(root,'core-0.8.7.tgz'))).digest('base64'),pin.coreIntegrity);
if(process.argv[6]==='--verify-only'){console.log(JSON.stringify({status:'SOURCE_VERIFIED',files:proof.files.length,commit:pin.commit}));process.exit(0);}
await mkdir(output,{recursive:false});
const profile=join(output,'owned-profile');
const origin='http://127.0.0.1:17707';
const sdk='/@fs/'+join(root,'upstream/npm/editor/index.js').replaceAll('\\','/');
const core='/@fs/'+join(root,'upstream/pkg/rhwp.js').replaceAll('\\','/');
const result={status:'RUNNING',scope:'RHWP_API_SYNTHETIC_NOT_GRAMMAR_ACCEPTANCE',upstreamCommit:pin.commit,version:pin.version,checks:[],findings:[],networkBlocked:0};
let browser, watchdog, page;
const record=name=>result.checks.push(name);
try {
 browser=await chromium.launchPersistentContext(profile,{executablePath:resolve(chromeArg),headless:true,chromiumSandbox:true,serviceWorkers:'block',acceptDownloads:false});
 watchdog=setTimeout(()=>{result.timeout=true;browser.close().catch(()=>{});},120000);
 await browser.route('**/*',route=>{
  const url=new URL(route.request().url());
  if(url.origin===origin || ['data:','blob:'].includes(url.protocol))return route.continue();
  result.networkBlocked++;return route.abort();
 });
 await browser.addInitScript(()=>{
  localStorage.setItem('rhwp-settings',JSON.stringify({version:1,theme:{skin:'default',skinChosen:true},autosave:{recoveryEnabled:false,idleSaveEnabled:false}}));
 });
 page=await browser.newPage();
 if(['--instant-only','--gemma-only'].includes(process.argv[6])){
  let host,hostOrigin;
  if(process.argv[6]==='--gemma-only'){
   assert.equal(process.env.GRAMMAR_HOST_TRIAL,'20261011-owned');
   assert.ok(process.env.GRAMMAR_INSTALL_ROOT);
   const install=resolve(process.env.GRAMMAR_INSTALL_ROOT);
   host=join(install,'grammar-chromium-host.exe');
   const manifest=JSON.parse(await readFile(join(install,'MANIFEST.json'),'utf8'));
   assert.equal(createHash('sha256').update(await readFile(host)).digest('hex'),manifest.files.find(x=>x.path==='grammar-chromium-host.exe').sha256);
   const config=JSON.parse(await readFile(join(install,'grammar-chromium-host.origin.json'),'utf8'));
   assert.ok(config.gemma);hostOrigin=config.origin;result.sourceCommit=manifest.sourceCommit;
   result.scope='ACTUAL_GEMMA_RHWP_OWNED_BRIDGE_FIXTURE_NOT_INSTALLED_ROUTING';
  }else{
   const hostDir=join(output,'host');await mkdir(hostDir);host=join(hostDir,'grammar-chromium-host.exe');await copyFile(new URL('../../src-tauri/target/debug/grammar-chromium-host.exe',import.meta.url),host);
   hostOrigin='chrome-extension://'+'a'.repeat(32)+'/';await writeFile(join(hostDir,'grammar-chromium-host.origin.json'),JSON.stringify({origin:hostOrigin}));
  }
  result.hostSha256=createHash('sha256').update(await readFile(host)).digest('hex');result.hostCalls=0;result.hostExits=[];
  await page.exposeFunction('grammarLocalAnalyze',async request=>{
   assert.equal(request.op,'analyze');assert.equal(typeof request.text,'string');assert.ok(request.text.length<=8192);
   const body=Buffer.from(JSON.stringify({version:1,op:'analyze',id:request.id,epoch:request.epoch,revision:request.revision,text:request.text,...(request.intent?{intent:request.intent}:{})}));
   const frame=Buffer.alloc(4);frame.writeUInt32LE(body.length);result.hostCalls++;
   return await new Promise((ok,no)=>{
    const child=spawn(host,[hostOrigin],{windowsHide:true,stdio:['pipe','pipe','pipe']});let chunks=[],size=0,error=null;
    const timer=setTimeout(()=>{error=new Error('HOST_TIMEOUT');child.kill();},10000);
    child.on('error',e=>{error=e;});child.stdout.on('data',b=>{size+=b.length;if(size>256*1024){error=new Error('HOST_OUTPUT_LIMIT');child.kill();}else {chunks.push(b);const out=Buffer.concat(chunks);if(out.length>=4&&out.length===4+out.readUInt32LE(0))child.stdin.end();}});
    child.stderr.on('data',()=>{error=new Error('HOST_STDERR');});child.stdin.on('error',e=>{error=e;});
    child.on('close',code=>{clearTimeout(timer);result.hostExits.push(code);if(error||code!==0)return no(error??new Error('HOST_EXIT'));
     try{const out=Buffer.concat(chunks);assert.ok(out.length>=4);assert.equal(out.readUInt32LE(0),out.length-4);ok(JSON.parse(out.subarray(4).toString('utf8')));}catch(e){no(e);}
    });child.stdin.write(Buffer.concat([frame,body]));
   });
  });
 }
 page.on('console',m=>{if(m.text().startsWith('GATE_STAGE'))console.log(m.text());});
 const errors=[];result.pageErrors=errors;page.on('pageerror',e=>errors.push(String(e)));
 await page.goto(origin+'/grammar-owned-harness.html');
 await page.evaluate(async({sdk,core,origin})=>{
  const {createEditor}=await import(sdk);
  const mod=await import(core);
  globalThis.measureTextWidth=(font,text)=>{const c=document.createElement('canvas').getContext('2d');c.font=font;return c.measureText(text).width;};
  await mod.default({module_or_path:core.replace(/rhwp\.js$/,'rhwp_bg.wasm')});
  const doc=mod.HwpDocument.createEmpty();doc.createBlankDocument();
  doc.insertText(0,0,0,'Header');doc.insertParagraph(0,1);doc.insertText(0,1,0,'Synthetic 한국어 😀 please seperate this.');
  doc.insertParagraph(0,2);doc.insertText(0,2,0,'Unselected tail');
  window.synthetic={hwp:Array.from(doc.exportHwp()),hwpx:Array.from(doc.exportHwpx())};doc.free();
  window.editor=await createEditor('#host',{studioUrl:origin+'/',renderer:'canvas2d',handshakeTimeoutMs:10000});
  window.expected=['Header','Synthetic 한국어 😀 please seperate this.','Unselected tail'];
  window.readAll=()=>{
   const w=editor.element.contentWindow.__wasm;const rows=[];
   for(let s=0;s<w.getSectionCount();s++)for(let p=0;p<w.getParagraphCount(s);p++)rows.push(w.getTextRange(s,p,0,w.getParagraphLength(s,p)));
   return rows;
  };
  window.commandFor=async(replacement)=>{
   const state=await editor.getDocumentState();
   const target={kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length};
   await editor.focusTarget(target);
   const selection=await editor.getSelectionContext();
   const {collectTargetEvidence}=await import('/src/document-agent/controller.ts');
   const evidence=collectTargetEvidence(editor.element.contentWindow.__wasm,target);
   return {state,selection,evidence,command:{schemaVersion:1,commandId:crypto.randomUUID(),expectedDocumentEpoch:state.documentEpoch,expectedChangeSeq:state.changeSeq,expectedDocumentSha256:state.documentSha256,target,expectedBeforeSha256:evidence.textSha256,expectedFormatSha256:evidence.formatSha256,expectedAdjacentContextSha256:evidence.adjacentContextSha256,replacement}};
  };
 },{sdk,core,origin});
 for(const format of (['--review-only','--instant-only','--gemma-only','--bootstrap-only','--bootstrap-denied-only'].includes(process.argv[6])?[]:['hwp','hwpx'])){
  const bytes=await page.evaluate(format=>synthetic[format],format);
  const data=Buffer.from(bytes);await writeFile(join(output,'synthetic.'+format),data);
  result[format]={sha256:createHash('sha256').update(data).digest('hex'),bytes:data.length};
  await page.evaluate(async format=>{await editor.loadFile(new Uint8Array(synthetic[format]),'synthetic.'+format,{suppressDialogs:true,skipUnsavedGuard:true});},format);
  assert.deepEqual(await page.evaluate(()=>readAll()),['Header','Synthetic 한국어 😀 please seperate this.','Unselected tail']);record(format+' synthetic export/open exact text');
  for(const field of ['expectedBeforeSha256','expectedFormatSha256','expectedDocumentSha256','expectedAdjacentContextSha256']){
   const refused=await page.evaluate(async field=>{
    const bound=await commandFor('MUST NOT WRITE');const before=readAll();
    bound.command[field]='0'.repeat(64);
    try{await editor.applyTextCommand(bound.command);return {accepted:true};}
    catch(e){return {accepted:false,code:e.code,before,after:readAll()};}
   },field);
   assert.equal(refused.accepted,false);assert.ok(refused.code);assert.deepEqual(refused.after,refused.before);
   record(format+' refuses '+field+' mismatch with exact preservation');
  }
  const trial=await page.evaluate(async()=>{
   const bound=await commandFor('Reviewed 한국어 😀 sentence.');
   const before=readAll();const receipt=await editor.applyTextCommand(bound.command);const after=readAll();
   const state=await editor.getDocumentState();
   await editor.revertTextCommand({schemaVersion:1,commandId:receipt.commandId,expectedDocumentEpoch:receipt.documentEpoch,expectedChangeSeq:receipt.afterChangeSeq,expectedAfterDocumentSha256:receipt.afterDocumentSha256,expectedAfterSha256:receipt.afterTextSha256});
   return {selection:bound.selection,before,after,reverted:readAll(),receipt,state};
  });
  assert.equal(trial.selection.editable,true);assert.equal(trial.selection.collapsed,false);
  assert.deepEqual(trial.after,['Header','Reviewed 한국어 😀 sentence.','Unselected tail']);assert.deepEqual(trial.reverted,trial.before);
  assert.equal(trial.receipt.afterDocumentSha256,trial.state.documentSha256);record(format+' exact API apply/revert and outside text preserved');
  const stale=await page.evaluate(async()=>{
   const old=await commandFor('STALE');const current=await commandFor('User synthetic edit');await editor.applyTextCommand(current.command);
   try{await editor.applyTextCommand(old.command);return {accepted:true,text:readAll()};}catch(e){return {accepted:false,code:e.code,text:readAll()};}
  });
  assert.equal(stale.accepted,false);assert.equal(stale.code,'CHANGE_SEQ_MISMATCH');assert.deepEqual(stale.text,['Header','User synthetic edit','Unselected tail']);record(format+' stale revision refuses without overwriting edit');
  const saved=await page.evaluate(async format=>Array.from(await (format==='hwp'?editor.exportHwp():editor.exportHwpx())),format);
  await writeFile(join(output,'edited.'+format),Buffer.from(saved));
  await page.evaluate(async({saved,format})=>editor.loadFile(new Uint8Array(saved),'edited.'+format,{suppressDialogs:true,skipUnsavedGuard:true}),{saved,format});
  assert.deepEqual(await page.evaluate(()=>readAll()),stale.text);record(format+' edited export/reopen exact text');
  const moved=await page.evaluate(async()=>{
   const bound=await commandFor('Changed old selection');
   await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:2,charOffset:0,length:Array.from(readAll()[2]).length});
   const selection=await editor.getSelectionContext();
   try{await editor.applyTextCommand(bound.command);return {accepted:true,selection,text:readAll()};}catch(e){return {accepted:false,code:e.code,selection,text:readAll()};}
  });
  assert.equal(moved.selection.target.paragraph,2);
  result.findings.push({format,kind:'SELECTION_FENCE_PROBE',...moved});
  const guarded=await page.evaluate(async()=>{
   const {createAcceptGate}=await import('/grammar-accept-gate.mjs');
   const {DocumentAgentController}=await import('/src/document-agent/controller.ts');
   const w=editor.element.contentWindow;let clock=0;const rows=[];
   for(const scenario of ['normal','moved','disabled','expired','source','operation-selection','operation-disable']){
    console.log('GATE_STAGE',scenario);
    const bound=await commandFor('Guarded replacement');
    let gate;
    const input=new Proxy(w.__inputHandler,{get(target,key){
     if(key==='executeDocumentAgentOperation')return (desc,render)=>{
      // Race precisely after controller preflight, before real history execution.
      if(scenario==='operation-selection')target.focusBodyParagraph(0,2,Array.from(readAll()[2]).length);
      if(scenario==='operation-disable')gate.disable();
      return target.executeDocumentAgentOperation(desc,render);
     };
     const value=target[key];return typeof value==='function'?value.bind(target):value;
    }});
    gate=createAcceptGate(DocumentAgentController,{wasm:w.__wasm,input,eventBus:w.__eventBus,isDirty:()=>true,render:()=>w.__canvasView.refreshDocumentAgentMutation()},()=>clock);
    gate.enable();const state=gate.getDocumentState();
    Object.assign(bound.command,{expectedDocumentEpoch:state.documentEpoch,expectedChangeSeq:state.changeSeq,expectedDocumentSha256:state.documentSha256});
    const token=gate.capture(bound.command);
    if(scenario==='moved')await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:2,charOffset:0,length:Array.from(readAll()[2]).length});
    if(scenario==='disabled')gate.disable();
    if(scenario==='expired')clock+=30000;
    if(scenario==='source'){const other=await commandFor('User intervening edit');await editor.applyTextCommand(other.command);}
    const before=readAll();let code='APPLIED';
    try{await gate.accept(token);}catch(e){code=e.code??String(e);}
    const after=readAll();let replay;
    try{await gate.accept(token);replay='APPLIED';}catch(e){replay=e.code;}
    rows.push({scenario,code,before,after,replay});gate.close();
   }
   return rows;
  });
  for(const row of guarded){
   if(row.scenario==='normal'){assert.equal(row.code,'APPLIED');assert.deepEqual(row.after,[row.before[0],'Guarded replacement',row.before[2]]);}
   else {assert.notEqual(row.code,'APPLIED');assert.deepEqual(row.after,row.before);}
   assert.equal(row.replay,'INVALID_TOKEN');record(format+' target gate '+row.scenario+' exact reread and single use');
  }
  result[format].guarded=guarded;
  const renderRace=await page.evaluate(async()=>{
   const {createAcceptGate}=await import('/grammar-accept-gate.mjs');
   const {DocumentAgentController}=await import('/src/document-agent/controller.ts');
   const w=editor.element.contentWindow,bound=await commandFor('Pending render');let injected=false;
   const gate=createAcceptGate(DocumentAgentController,{wasm:w.__wasm,input:w.__inputHandler,eventBus:w.__eventBus,isDirty:()=>true,render:async()=>{
    if(!injected){injected=true;w.__wasm.replaceText(0,2,0,Array.from(readAll()[2]).length,'Intervening application edit');w.__eventBus.emit('document-mutated','synthetic-external');}
    throw new Error('Injected render failure');
   }});
   gate.enable();const state=gate.getDocumentState();Object.assign(bound.command,{expectedDocumentEpoch:state.documentEpoch,expectedChangeSeq:state.changeSeq,expectedDocumentSha256:state.documentSha256});
   const history=w.__inputHandler.history;
   const priorCommands=[...history.undoStack,...history.redoStack];
   const token=gate.capture(bound.command);let code;
   try{await gate.accept(token);code='APPLIED';}catch(e){code=e.code??String(e);}
   const text=readAll();let reuse;
   try{gate.capture(bound.command);reuse='ALLOWED';}catch(e){reuse=e.code;}
   const remaining=[...history.undoStack,...history.redoStack];
   const unexpectedEntries=remaining.filter(c=>!priorCommands.includes(c)).length;
   const priorPreserved=priorCommands.every(c=>remaining.includes(c));
   gate.close();return {injected,code,text,reuse,unexpectedEntries,priorPreserved,userEditPreserved:text[2]==='Intervening application edit'};
  });
  assert.equal(renderRace.injected,true);assert.notEqual(renderRace.code,'APPLIED');assert.equal(renderRace.reuse,'AUTHORITY_REVOKED');
  assert.equal(renderRace.userEditPreserved,true);assert.equal(renderRace.text[1],'Pending render');
  // Observe actual prior entries, not an assumed count after upstream rollback.
  assert.equal(renderRace.unexpectedEntries,0);assert.equal(renderRace.priorPreserved,true);
  record(format+' render failure preserves intervening application edit and quarantines gate');
  result.findings.push({format,kind:'RENDER_FAILURE_APPLICATION_EDIT_PROBE',...renderRace});
  const historyCases=await page.evaluate(async format=>{
   const rows=[];
   for(const scenario of ['roundtrip','changed-before-undo','changed-before-redo']){
    await editor.loadFile(new Uint8Array(synthetic[format]),'history.'+format,{suppressDialogs:true,skipUnsavedGuard:true});
    const w=editor.element.contentWindow;
    const {createAcceptGate}=await import('/grammar-accept-gate.mjs');
    const {DocumentAgentController}=await import('/src/document-agent/controller.ts');
    const bound=await commandFor('History replacement');
    const gate=createAcceptGate(DocumentAgentController,{wasm:w.__wasm,input:w.__inputHandler,eventBus:w.__eventBus,isDirty:()=>true,render:()=>w.__canvasView.refreshDocumentAgentMutation()});
    gate.enable();const state=gate.getDocumentState();Object.assign(bound.command,{expectedDocumentEpoch:state.documentEpoch,expectedChangeSeq:state.changeSeq,expectedDocumentSha256:state.documentSha256});
    const before=readAll();await gate.accept(gate.capture(bound.command));const applied=readAll();
    if(scenario==='changed-before-undo'){w.__wasm.replaceText(0,2,0,Array.from(readAll()[2]).length,'External edit retained');w.__eventBus.emit('document-mutated','synthetic-external');}
    const undoTop=w.__inputHandler.history.peekUndoTop();
    try{w.__inputHandler.handleUndo();}catch(e){if(scenario!=='changed-before-undo'||e.message!=='GRAMMAR_HISTORY_SOURCE_CHANGED')throw e;}
    const undone=readAll();
    const undoRefusedPreservesEntry=scenario!=='changed-before-undo'||w.__inputHandler.history.peekUndoTop()===undoTop;
    if(scenario==='changed-before-redo'){w.__wasm.replaceText(0,2,0,Array.from(readAll()[2]).length,'External edit retained');w.__eventBus.emit('document-mutated','synthetic-external');}
    const redoTop=w.__inputHandler.history.peekRedoTop();
    if(scenario!=='changed-before-undo')try{w.__inputHandler.handleRedo();}catch(e){if(scenario!=='changed-before-redo'||e.message!=='GRAMMAR_HISTORY_SOURCE_CHANGED')throw e;}
    const redoRefusedPreservesEntry=scenario!=='changed-before-redo'||w.__inputHandler.history.peekRedoTop()===redoTop;
    const cycles=[];
    if(scenario==='roundtrip')for(let n=0;n<3;n++){w.__inputHandler.handleUndo();const undo=readAll();w.__inputHandler.handleRedo();cycles.push({undo,redo:readAll()});}
    rows.push({scenario,before,applied,undone,final:readAll(),cycles,undoRefusedPreservesEntry,redoRefusedPreservesEntry});gate.close();
   }
   return rows;
  },format);
  result[format].historyCases=historyCases;
  // Persist counterexample observations before assertions.
  await writeFile(join(output,'history-'+format+'.json'),JSON.stringify(historyCases,null,2));
  for(const row of historyCases){
   if(row.scenario==='roundtrip'){assert.deepEqual(row.undone,row.before);assert.deepEqual(row.final,row.applied);}
   else {assert.equal(row.final[2],'External edit retained');assert.equal(row.final[1],row.scenario==='changed-before-undo'?'History replacement':row.before[1]);}
   record(format+' history '+row.scenario+' exact text');
   assert.equal(row.undoRefusedPreservesEntry,true);assert.equal(row.redoRefusedPreservesEntry,true);
   for(const cycle of row.cycles){assert.deepEqual(cycle.undo,row.before);assert.deepEqual(cycle.redo,row.applied);}
  }
  // This measures an upstream API boundary, not a pass criterion for Grammar.
  // A hash-bound old target may still be accepted after selection moves.
 }
 if(process.argv[6]==='--review-only')for(const format of ['hwp','hwpx']){
  await page.evaluate(async format=>{
   await editor.loadFile(new Uint8Array(synthetic[format]),'review.'+format,{suppressDialogs:true,skipUnsavedGuard:true});
   const w=editor.element.contentWindow;
   const {createReviewSession}=await import('/grammar-review/review-session.mjs');
   const {mountReviewPanel}=await import('/grammar-review/review-panel.mjs');
   const {DocumentAgentController,collectTargetEvidence}=await import('/src/document-agent/controller.ts');
   window.reviewEligible=true;window.reviewReads=0;
   const wasm=new Proxy(w.__wasm,{get(target,key){const value=target[key];if(typeof value!=='function')return value;return (...args)=>{if(key==='getTextRange'||key==='borrowDocumentHandle')reviewReads++;return value.apply(target,args);};}});
   window.review=createReviewSession(DocumentAgentController,collectTargetEvidence,{wasm,input:w.__inputHandler,eventBus:w.__eventBus,isDirty:()=>true,render:()=>w.__canvasView.refreshDocumentAgentMutation()},()=>reviewEligible);
   window.removeReview=mountReviewPanel(document.body,review);
   await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length});
  },format);
  await page.getByRole('button',{name:'Capture selection',exact:true}).click();
  assert.equal(await page.evaluate(()=>reviewReads),0);record(format+' disabled review reads no document text');
  await page.getByRole('button',{name:'Enable review',exact:true}).click();
  await page.getByRole('button',{name:'Capture selection',exact:true}).click();
  assert.equal(await page.getByRole('textbox',{name:'Reviewed replacement'}).inputValue(),'Synthetic 한국어 😀 please seperate this.');
  assert.deepEqual(await page.evaluate(()=>readAll()),['Header','Synthetic 한국어 😀 please seperate this.','Unselected tail']);record(format+' capture returns only selected paragraph and does not mutate');
  await page.getByRole('textbox',{name:'Reviewed replacement'}).fill('Reviewed UI 한국어 😀');
  await page.getByRole('button',{name:'Accept',exact:true}).click();
  await page.getByRole('status').filter({hasText:/^Applied$/}).waitFor();
  assert.deepEqual(await page.evaluate(()=>readAll()),['Header','Reviewed UI 한국어 😀','Unselected tail']);record(format+' explicit edited UI Accept exact source reread');
  await page.evaluate(()=>editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length}));
  await page.getByRole('button',{name:'Capture selection',exact:true}).click();
  await page.getByRole('textbox',{name:'Reviewed replacement'}).fill('MUST NOT APPLY');
  await page.evaluate(()=>editor.focusTarget({kind:'body_paragraph',section:0,paragraph:2,charOffset:0,length:Array.from(readAll()[2]).length}));
  await page.getByRole('button',{name:'Accept',exact:true}).click();
  await page.getByRole('status').filter({hasText:/^SELECTION_CHANGED$/}).waitFor();
  assert.deepEqual(await page.evaluate(()=>readAll()),['Header','Reviewed UI 한국어 😀','Unselected tail']);record(format+' moved selection rejects edited UI Accept');
  await page.getByRole('button',{name:'Disable review',exact:true}).click();
  assert.equal(await page.getByRole('textbox',{name:'Reviewed replacement'}).inputValue(),'');
  assert.equal(await page.getByRole('button',{name:'Accept',exact:true}).isDisabled(),true);record(format+' Disable clears visible draft and authority');
  const refusal=await page.evaluate(async()=>{
   review.enable();reviewEligible=false;reviewReads=0;
   let denied;try{review.capture();}catch(e){denied=e.code;}
   const deniedReads=reviewReads;reviewEligible=true;
   await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length});
   const old=review.capture();const next=review.capture();let stale;
   try{await review.accept(old.id,'STALE');}catch(e){stale=e.code;}
   return {denied,deniedReads,stale,keys:Object.keys(next).sort(),text:readAll()};
  });
  assert.equal(refusal.denied,'NOT_ENABLED_OR_ELIGIBLE');assert.equal(refusal.deniedReads,0);record(format+' ineligible document refused before text/export');
  assert.equal(refusal.stale,'STALE_REVIEW');assert.deepEqual(refusal.keys,['id','source']);assert.deepEqual(refusal.text,['Header','Reviewed UI 한국어 😀','Unselected tail']);record(format+' recapture invalidates old review id without mutation');
  const boundaries=await page.evaluate(async()=>{
   const w=editor.element.contentWindow;
   const selected=w.__inputHandler.cursor.selectRange({sectionIndex:0,paragraphIndex:1,charOffset:1},{sectionIndex:0,paragraphIndex:1,charOffset:4});
   reviewReads=0;let partial;try{review.capture();}catch(e){partial=e.code;}
   const partialReads=reviewReads;
   await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length});
   const binding=review.capture();reviewEligible=false;let revoked;
   try{await review.accept(binding.id,'DENIED');}catch(e){revoked=e.code;}
   review.disable();reviewEligible=true;review.enable();
   const original=review.capture();const external=await commandFor('New application source');await editor.applyTextCommand(external.command);
   let changed;try{await review.accept(original.id,'OLD DRAFT');}catch(e){changed=e.code;}
   return {selected,partial,partialReads,revoked,changed,text:readAll()};
  });
  assert.equal(boundaries.selected,true);assert.equal(boundaries.partial,'SELECT_WHOLE_PLAIN_PARAGRAPH');assert.equal(boundaries.partialReads,0);record(format+' partial selection refused without source read');
  assert.equal(boundaries.revoked,'NOT_ENABLED_OR_ELIGIBLE');record(format+' admission revoked after capture refuses Accept');
  assert.equal(boundaries.changed,'DOCUMENT_CHANGED');assert.deepEqual(boundaries.text,['Header','New application source','Unselected tail']);record(format+' changed application source preserves new text');
  await page.evaluate(()=>removeReview());
 }
 if(process.argv[6]==='--instant-only')for(const format of ['hwp','hwpx']){
  await page.evaluate(async format=>{
   await editor.loadFile(new Uint8Array(synthetic[format]),'instant.'+format,{suppressDialogs:true,skipUnsavedGuard:true});
   const w=editor.element.contentWindow;
   await import('/grammar-core.js');
   const {createInstantDraft}=await import('/grammar-review/instant-draft.mjs');
   const {createReviewSession}=await import('/grammar-review/review-session.mjs');
   const {mountReviewPanel}=await import('/grammar-review/review-panel.mjs');
   const {DocumentAgentController,collectTargetEvidence}=await import('/src/document-agent/controller.ts');
   window.review=createReviewSession(DocumentAgentController,collectTargetEvidence,{wasm:w.__wasm,input:w.__inputHandler,eventBus:w.__eventBus,isDirty:()=>true,render:()=>w.__canvasView.refreshDocumentAgentMutation()},()=>true);
   window.holdInstant=false;window.releaseInstant=null;
   const instant=createInstantDraft(async request=>{const result=await grammarLocalAnalyze(request);if(holdInstant)await new Promise(resolve=>{releaseInstant=resolve;});return result;},GrammarCore);
   window.removeReview=mountReviewPanel(document.body,review,instant);
   await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length});
  },format);
  const capture=()=>page.getByRole('button',{name:'Capture selection',exact:true}).click();
  const field=page.getByRole('textbox',{name:'Reviewed replacement'});
  await page.getByRole('button',{name:'Enable review',exact:true}).click();await capture();
  await page.getByRole('status').filter({hasText:/^Local Instant ready/}).waitFor();
  assert.equal(await field.inputValue(),'Synthetic 한국어 😀 please separate this.');
  assert.equal((await page.evaluate(()=>readAll()))[1],'Synthetic 한국어 😀 please seperate this.');record(format+' real native Instant corrects draft without document mutation');
  await page.getByRole('button',{name:'Accept',exact:true}).click();await page.getByRole('status').filter({hasText:/^Applied$/}).waitFor();
  assert.equal((await page.evaluate(()=>readAll()))[1],'Synthetic 한국어 😀 please separate this.');record(format+' explicit Accept applies Instant draft');
  for(const race of ['edit','disable','selection']){
   await page.evaluate(async()=>{await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length});holdInstant=true;releaseInstant=null;});
   if(race==='selection')await page.getByRole('button',{name:'Enable review',exact:true}).click();
   await capture();await page.waitForFunction(()=>typeof releaseInstant==='function');
   if(race==='edit')await field.fill('User draft must remain');
   if(race==='disable')await page.getByRole('button',{name:'Disable review',exact:true}).click();
   if(race==='selection')await page.evaluate(()=>editor.focusTarget({kind:'body_paragraph',section:0,paragraph:2,charOffset:0,length:Array.from(readAll()[2]).length}));
   await page.evaluate(async()=>{holdInstant=false;releaseInstant();await new Promise(r=>setTimeout(r,0));});
   if(race==='edit'){await page.getByRole('status').filter({hasText:/^Your edited draft was kept$/}).waitFor();assert.equal(await field.inputValue(),'User draft must remain');}
   if(race==='disable'){assert.equal(await field.inputValue(),'');assert.equal(await page.getByRole('button',{name:'Accept',exact:true}).isDisabled(),true);}
   if(race==='selection'){await page.getByRole('status').filter({hasText:/^Selection changed/}).waitFor();assert.equal(await page.getByRole('button',{name:'Accept',exact:true}).isDisabled(),true);}
   assert.equal((await page.evaluate(()=>readAll()))[1],'Synthetic 한국어 😀 please separate this.');record(format+' delayed Instant preserves '+race+' boundary');
  }
  await page.evaluate(()=>removeReview());
 }
 if(process.argv[6]==='--gemma-only'){
  result.samples=[];
  for(const format of ['hwp','hwpx'])for(const mode of ['ko','paraphrase','polite']){
   const source='Do not change the price. The total is 25 dollars. Send me the report today.';
   await page.evaluate(async({format,source})=>{
    await editor.loadFile(new Uint8Array(synthetic[format]),'gemma.'+format,{suppressDialogs:true,skipUnsavedGuard:true});
    const w=editor.element.contentWindow;
    w.__wasm.deleteText(0,1,0,w.__wasm.getParagraphLength(0,1));w.__wasm.insertText(0,1,0,source);
    await import('/grammar-core.js');
    const {createInstantDraft}=await import('/grammar-review/instant-draft.mjs');
    const {createReviewSession}=await import('/grammar-review/review-session.mjs');
    const {mountReviewPanel}=await import('/grammar-review/review-panel.mjs');
    const {DocumentAgentController,collectTargetEvidence}=await import('/src/document-agent/controller.ts');
    window.review=createReviewSession(DocumentAgentController,collectTargetEvidence,{wasm:w.__wasm,input:w.__inputHandler,eventBus:w.__eventBus,isDirty:()=>true,render:()=>w.__canvasView.refreshDocumentAgentMutation()},()=>true);
    window.removeReview=mountReviewPanel(document.body,review,createInstantDraft(grammarLocalAnalyze,GrammarCore,{writing:true}));
    await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(source).length});
    window.properties=()=>JSON.stringify({character:w.__wasm.getCharPropertiesAt(0,1,0),paragraph:w.__wasm.getParaPropertiesAt(0,1),style:w.__wasm.getStyleAt(0,1)});window.propertiesBefore=properties();
    window.formatBefore=collectTargetEvidence(w.__wasm,{kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(source).length}).formatSha256;
    window.readFormat=()=>collectTargetEvidence(w.__wasm,{kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:w.__wasm.getParagraphLength(0,1)}).formatSha256;
   },{format,source});
   await page.getByLabel('Local writing mode').selectOption(mode);
   await page.getByRole('button',{name:'Enable review',exact:true}).click();
   const start=performance.now();await page.getByRole('button',{name:'Capture selection',exact:true}).click();
   await page.getByRole('status').filter({hasText:/^Local Instant ready/}).waitFor();
   const outputText=await page.getByRole('textbox',{name:'Reviewed replacement'}).inputValue();
   const sample={format,mode,source,output:outputText,recommendationMs:performance.now()-start};
   assert.notEqual(outputText,source);assert.match(outputText,/25/);if(mode==='ko')assert.match(outputText,/[가-힣]/);
   assert.deepEqual(await page.evaluate(()=>readAll()),['Header',source,'Unselected tail']);
   const acceptStart=performance.now();await page.getByRole('button',{name:'Accept',exact:true}).click();await page.getByRole('status').filter({hasText:/^Applied$/}).waitFor();
   assert.deepEqual(await page.evaluate(()=>readAll()),['Header',outputText,'Unselected tail']);sample.acceptMs=performance.now()-acceptStart;
   assert.equal(await page.evaluate(()=>readFormat()),await page.evaluate(()=>formatBefore));assert.equal(await page.evaluate(()=>properties()),await page.evaluate(()=>propertiesBefore));record(format+' '+mode+' actual Gemma Accept exact text and format');
   await page.evaluate(()=>editor.element.contentWindow.__inputHandler.handleUndo());
   assert.deepEqual(await page.evaluate(()=>readAll()),['Header',source,'Unselected tail']);assert.equal(await page.evaluate(()=>readFormat()),await page.evaluate(()=>formatBefore));assert.equal(await page.evaluate(()=>properties()),await page.evaluate(()=>propertiesBefore));record(format+' '+mode+' Undo exact text and format');
   result.samples.push(sample);await page.evaluate(()=>removeReview());
  }
 }
 if(['--bootstrap-only','--bootstrap-denied-only'].includes(process.argv[6]))for(const format of ['hwp','hwpx']){
  await page.evaluate(async format=>{
   await editor.loadFile(new Uint8Array(synthetic[format]),'bootstrap.'+format,{suppressDialogs:true,skipUnsavedGuard:true});
   const w=editor.element.contentWindow;
   // Remove globals previously used to construct the test session. The compiled
   // editor initialization now supplies dependencies directly.
   delete w.__inputHandler;delete w.__eventBus;delete w.__canvasView;
   await editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length});
  },format);
  await page.waitForFunction(async()=>editor.element.contentDocument.getElementById('grammar-review-host')?.dataset.grammarGeneration===String((await editor.getDocumentState()).documentEpoch));
  const panel=page.frameLocator('iframe').locator('#grammar-review-host');
  assert.equal(await panel.getByRole('status').textContent(),'Disabled');record(format+' editor bootstrap mounts disabled without dev dependency globals');
  if(process.argv[6]==='--bootstrap-denied-only'){
   await page.evaluate(()=>{const w=editor.element.contentWindow;window.bootReads=0;window.oldBootRead=w.__wasm.getTextRange;w.__wasm.getTextRange=function(...args){bootReads++;return oldBootRead.apply(this,args);};});
   await panel.getByRole('button',{name:'Enable review',exact:true}).click();await panel.getByRole('button',{name:'Capture selection',exact:true}).click();
   assert.equal(await panel.getByRole('status').textContent(),'NOT_ENABLED_OR_ELIGIBLE');assert.equal(await page.evaluate(()=>bootReads),0);
   await page.evaluate(()=>{editor.element.contentWindow.__wasm.getTextRange=oldBootRead;});
   assert.equal(await panel.getByRole('textbox',{name:'Reviewed replacement'}).inputValue(),'');record(format+' default build admission denies capture before source read');
   continue;
  }
  await panel.getByRole('button',{name:'Enable review',exact:true}).click();await panel.getByRole('button',{name:'Capture selection',exact:true}).click();
  assert.equal(await panel.getByRole('textbox',{name:'Reviewed replacement'}).inputValue(),'Synthetic 한국어 😀 please seperate this.');
  await panel.getByRole('textbox',{name:'Reviewed replacement'}).fill('Bootstrap explicit replacement');await panel.getByRole('button',{name:'Accept',exact:true}).click();
  await panel.getByRole('status').filter({hasText:/^Applied$/}).waitFor();
  assert.deepEqual(await page.evaluate(()=>readAll()),['Header','Bootstrap explicit replacement','Unselected tail']);record(format+' compiled in-editor bootstrap explicit Accept exact reread');
  await page.evaluate(()=>editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length}));
  await panel.getByRole('button',{name:'Capture selection',exact:true}).click();
  await page.evaluate(async format=>editor.loadFile(new Uint8Array(synthetic[format]),'replacement.'+format,{suppressDialogs:true,skipUnsavedGuard:true}),format);
  await page.waitForFunction(async()=>editor.element.contentDocument.getElementById('grammar-review-host')?.dataset.grammarGeneration===String((await editor.getDocumentState()).documentEpoch));
  assert.equal(await panel.getByRole('textbox',{name:'Reviewed replacement'}).inputValue(),'');assert.equal(await panel.getByRole('status').textContent(),'Disabled');record(format+' new document clears draft and revokes opt-in');
  // Synthetic protection metadata fault; no password or encrypted user file.
  await page.evaluate(()=>editor.focusTarget({kind:'body_paragraph',section:0,paragraph:1,charOffset:0,length:Array.from(readAll()[1]).length}));
  await panel.getByRole('button',{name:'Enable review',exact:true}).click();await panel.getByRole('button',{name:'Capture selection',exact:true}).click();
  const beforeProtection=await page.evaluate(()=>readAll());
  await page.evaluate(()=>{const w=editor.element.contentWindow.__wasm;window.protectedReads=0;window.oldProtectedRead=w.getTextRange;w.getTextRange=function(...a){protectedReads++;return oldProtectedRead.apply(this,a);};w.requiresPasswordForSave=true;const button=[...editor.element.contentDocument.querySelectorAll('#grammar-review-host button')].find(b=>b.textContent==='Accept');if(button.disabled)throw Error('EXPECTED_READY_ACCEPT');button.click();});
  // Protection change and click share one JS task, before the UI poll.
  assert.equal(await page.evaluate(()=>protectedReads),0);
  await panel.getByRole('status').filter({hasText:/^Disabled$/}).waitFor();
  assert.equal(await panel.getByRole('textbox',{name:'Reviewed replacement'}).inputValue(),'');
  await page.evaluate(()=>{const w=editor.element.contentWindow.__wasm;w.requiresPasswordForSave=false;});
  await panel.getByRole('button',{name:'Enable review',exact:true}).click();await panel.getByRole('button',{name:'Capture selection',exact:true}).click();
  assert.equal(await panel.getByRole('status').textContent(),'NOT_ENABLED_OR_ELIGIBLE');assert.equal(await page.evaluate(()=>protectedReads),0);
  await page.evaluate(()=>{editor.element.contentWindow.__wasm.getTextRange=oldProtectedRead;});
  assert.deepEqual(await page.evaluate(()=>readAll()),beforeProtection);record(format+' protection revokes before reads, clears draft and cannot revive same document');
  await page.evaluate(()=>editor.element.contentWindow.dispatchEvent(new PageTransitionEvent('pagehide')));
  assert.equal(await panel.getByRole('button',{name:'Accept',exact:true}).count(),0);record(format+' pagehide disposes panel and authority');
  // Reload the editor iframe for the next document; parent SDK object handshake
  // is recreated rather than reviving a closed bootstrap.
  if(format==='hwp')await page.evaluate(async({sdk,origin})=>{editor.destroy();const {createEditor}=await import(sdk);editor=await createEditor('#host',{studioUrl:origin+'/',renderer:'canvas2d',handshakeTimeoutMs:10000});},{sdk,origin});
 }
 result.pageErrors=errors;
 await page.evaluate(()=>editor.destroy());
 result.indexedDatabases=await page.evaluate(async()=>{
  const rows=[];
  for(const info of await indexedDB.databases()){
   const db=await new Promise((ok,no)=>{const q=indexedDB.open(info.name);q.onsuccess=()=>ok(q.result);q.onerror=()=>no(q.error);});
   const stores={};
   for(const name of db.objectStoreNames)stores[name]=await new Promise((ok,no)=>{const q=db.transaction(name).objectStore(name).count();q.onsuccess=()=>ok(q.result);q.onerror=()=>no(q.error);});
   rows.push({name:info.name,stores});db.close();
  }
  return rows;
 });
 result.status='PASS_API_FIXTURE';
} catch(e){result.status='FAIL';result.error=String(e.stack??e);if(page)result.visibleStatus=await page.getByRole('status').allTextContents().catch(()=>[]);}
finally{
 clearTimeout(watchdog);
 try { if(browser)await browser.close(); } catch(e){result.status='FAIL';result.browserCloseError=String(e);}
 try {
  assert.equal(profile,join(output,'owned-profile'));
  await rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:200});
  result.profileRemoved=true;
 } catch(e){result.status='FAIL';result.profileRemoved=false;result.cleanupError=String(e);}
 await writeFile(join(output,'receipt.json'),JSON.stringify(result,null,2));
}
console.log(JSON.stringify({status:result.status,checks:result.checks.length,findings:result.findings.map(x=>({format:x.format,kind:x.kind,accepted:x.accepted})),error:result.error}));
if(result.status==='FAIL')process.exitCode=1;
