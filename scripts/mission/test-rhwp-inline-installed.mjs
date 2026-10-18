import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
// Run only inside the owned single-session supervisor with an installed package.
const config=JSON.parse(await readFile(process.argv[2],'utf8'));
const r=config.evidenceRoot;
const {chromium}=createRequire(import.meta.url)(config.playwrightModule);
const manifest=JSON.parse(await readFile(r+'/install/MANIFEST.json','utf8'));
const ext=r+'/install/extension',origin='chrome-extension://'+manifest.extensionId;
const core=await import(pathToFileURL(config.wasmModule));
await core.default({module_or_path:await readFile(config.wasmBinary)});
const snapshot=async path=>{const d=new core.HwpDocument(await readFile(path));try{return Array.from({length:d.getSectionCount()},(_,s)=>Array.from({length:d.getParagraphCount(s)},(_,p)=>({text:d.getTextRange(s,p,0,d.getParagraphLength(s,p)),chars:Array.from({length:d.getParagraphLength(s,p)},(_,i)=>JSON.parse(d.getCharPropertiesAt(s,p,i))),para:JSON.parse(d.getParaPropertiesAt(s,p)),style:JSON.parse(d.getStyleAt(s,p))}))).flat();}finally{d.free();}};
let browser,page;const result={status:'FAIL',scope:'INSTALLED_NATIVE_MESSAGING_NO_ANALYSIS_BRIDGE',checks:[],samples:[],errors:[],externalRequests:[]};
try{
 assert.ok(config.headless===undefined||typeof config.headless==='boolean');
 browser=await chromium.launchPersistentContext(config.browserProfile,{executablePath:config.chromiumBinary,headless:config.headless??true,chromiumSandbox:true,acceptDownloads:true,ignoreDefaultArgs:['--disable-extensions'],args:['--disable-extensions-except='+ext,'--load-extension='+ext]});
 browser.on('request',q=>{if(/^https?:/.test(q.url()))result.externalRequests.push(new URL(q.url()).origin);});
 page=await browser.newPage();page.on('pageerror',e=>result.errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')console.log('PAGE_ERROR',m.text().slice(0,300));});page.on('dialog',d=>d.accept());
 if(config.diagnose){const debug=await browser.newCDPSession(page);await debug.send('Debugger.enable');await debug.send('Debugger.setPauseOnExceptions',{state:'all'});debug.on('Debugger.paused',async event=>{try{const frame=event.callFrames.find(f=>['parseRichDraft','parseRichAlignment'].includes(f.functionName));if(frame){const value=await debug.send('Debugger.evaluateOnCallFrame',{callFrameId:frame.callFrameId,expression:'({text,paragraphs:paragraphs.map(p=>p.runs.map(r=>({id:r.id,text:r.text})))})',returnByValue:true});(result.rejectedSyntheticReplies??=[]).push(value.result.value);}}finally{await debug.send('Debugger.resume').catch(()=>{});}});}
 await page.goto(origin+'/rhwp/index.html');await page.locator('#grammar-review-host').waitFor();console.log('PANEL_READY');
 assert.equal(await page.evaluate(()=>typeof __wasm),'undefined');assert.equal(await page.evaluate(()=>typeof grammarLocalAnalyze),'undefined');
 for(const fixture of (config.fixtures??[{id:'default',inputs:config.inputs}]))for(const scenario of (config.scenarios??['inline','partial','partial-span']))for(const format of ['hwp','hwpx'])for(const mode of (fixture.modes??config.modes??['ko','paraphrase','polite'])){
  const input=fixture.inputs[format];
  const before=await snapshot(input);const last=scenario==='partial-span'?2:1;const from=scenario==='inline'?0:16;const to=scenario==='inline'?Array.from(before[last].text).length:Array.from(before[last].text).length-16;const selected=before.slice(1,last+1);const source=selected.map((p,i)=>Array.from(p.text).slice(i===0?from:0,i===selected.length-1?to:undefined).join('')).join('\n');console.log('OPEN',format,mode,source.length);
  const oldGeneration=await page.locator('#grammar-review-host').getAttribute('data-grammar-generation');
  await page.locator('#file-input').setInputFiles(input);
  await page.waitForFunction(old=>!!document.querySelector('#grammar-review-host')&&!document.documentElement.classList.contains('rhwp-busy')&&document.querySelector('#grammar-review-host').dataset.grammarGeneration!==old,oldGeneration);
  await page.getByRole('button',{name:'Enable review',exact:true}).waitFor();
  await page.getByLabel('Local writing mode').selectOption(mode);
  await page.getByRole('button',{name:'Enable review',exact:true}).click();
  await page.locator('#scroll-container canvas').first().click({position:{x:120,y:120}});const inputBox=page.getByLabel('문서 편집 입력');await inputBox.focus();await inputBox.press('Control+Home');await inputBox.press('Control+ArrowDown');await inputBox.press('Home');
  for(let i=0;i<from;i++)await inputBox.press('ArrowRight');
  for(let i=0;i<Array.from(source).length;i++)await inputBox.press('Shift+ArrowRight');
  const start=performance.now();await page.getByRole('button',{name:'Capture selection',exact:true}).click();
  console.log('CAPTURE_STATUS',await page.locator('#grammar-review-host [role=status]').textContent());
  await page.locator('#grammar-review-host [role=status]').filter({hasText:/^Local Instant ready/}).waitFor({timeout:15000});
  const text=await page.getByLabel('Reviewed replacement').inputValue();
  if(config.corpus==='korean-names'){
   assert.match(text,/[가-힣]/);
   for(const name of fixture.names)assert.equal(text.split(name).length-1,source.split(name).length-1,'exact proper-name occurrence: '+name);
   assert.equal(text.split(fixture.literal).length-1,1);
   for(const pattern of fixture.required)assert.match(text,new RegExp(pattern));
   for(const pattern of fixture.forbidden)assert.doesNotMatch(text,new RegExp(pattern));
  }else{
if(config.corpus==='repeated'){assert.equal((text.match(/\"DRAFT\"/g)??[]).length,2);assert.equal((text.match(/DRAFT/g)??[]).length,2);if(mode==='ko')assert.doesNotMatch(text,/"DRAFT"을/);}else if(config.corpus==='mixed'){assert.equal((text.match(/"check"/g)??[]).length,1);}else assert.equal((text.match(/25/g)??[]).length,1);if(mode==='ko'){assert.match(text,/[가-힣]/);assert.doesNotMatch(text.replaceAll('DRAFT','').replaceAll('"check"',''),/[A-Za-z]/);}else assert.doesNotMatch(text,/[가-힣]/);
  }
  const spans=await page.locator('[aria-label="Formatted replacement preview"] [data-run-id]').evaluateAll(nodes=>nodes.map(n=>({id:Number(n.dataset.runId),paragraph:Number(n.dataset.paragraph),text:n.textContent,original:n.title})));
  const sample={fixture:fixture.id,scenario,format,mode,source,output:text,spans,recommendationMs:performance.now()-start};
  result.pendingSample=sample;
  const emphasized=spans.filter(s=>s.original==='Original styled text: '+(fixture.styledWord??'check'));assert.ok(emphasized.length);
  assert.match(emphasized.map(s=>s.text).join(''),fixture.expectedEmphasis?new RegExp(fixture.expectedEmphasis):mode==='ko'?/^(확인|검토)$/:/check|verif|review/i);
  assert.doesNotMatch(emphasized.map(s=>s.text).join(''),/25|달러|dollar|report|보고서/i); // A word's emphasis must not spread onto facts or a different noun.
  if(scenario==='partial-span'){const report=spans.filter(s=>s.original==='Original styled text: report').map(s=>s.text).join('');assert.match(report,mode==='ko'?/^보고서$/:/report/i);assert.doesNotMatch(report,/25|달러|dollar/i);}
  const joined=text.split('\n').join('');assert.deepEqual(joined.match(/\d+/g)??[],source.match(/\d+/g)??[],'preserve exact numeric tokens');for(const clause of joined.replace(/"[^"]*"/g,'').split(/[.;!?]/))assert.ok((clause.match(/\b(?:please|kindly)\b/gi)??[]).length<=1,'no redundant courtesy markers in the same request');if(!['mixed','korean-names'].includes(config.corpus))assert.match(joined,mode==='ko'?/다시/:/recheck|double-check|again/i);
  if(!['mixed','korean-names'].includes(config.corpus)&&mode!=='ko'&&joined.includes('check'))assert.equal(emphasized.map(s=>s.text).join(''),'check');
  if(config.corpus==='repeated'){
   assert.equal((joined.match(mode==='ko'?/지 마|지 않|안 됩니다|안 돼/g:/\bnot\b/g)??[]).length,scenario==='partial-span'?2:1);
   assert.match(joined,mode==='ko'?/지 마|지 않|안 됩니다|안 돼/:/do not|don't/i);
   assert.match(joined,mode==='ko'?/승인/:/approval/i);assert.match(joined,mode==='ko'?/전/:/before|prior to/i);assert.match(joined,mode==='ko'?/후/:/after|following/i);
   const firstBlock=spans.filter(s=>s.paragraph===0),highlight=firstBlock.findIndex(s=>s.original==='Original styled text: check');assert.ok(highlight>=0);
   const preceding=firstBlock.slice(0,highlight).map(s=>s.text).join('');assert.match(preceding,mode==='ko'?/확인|검토/:/check|verif|review/i); // The first repeated action remains unhighlighted.
   if(scenario==='partial-span'){const second=spans.filter(s=>s.paragraph===1),at=second.findIndex(s=>s.original==='Original styled text: report');assert.ok(at>=0);assert.match(second.slice(0,at).map(s=>s.text).join(''),mode==='ko'?/보고서/:/report/i);}
  }else if(config.corpus==='mixed'){
   const highlight=spans.findIndex(s=>s.original==='Original styled text: check');
   assert.match(spans.slice(0,highlight).map(s=>s.text).join(''),/"check"/);
   assert.match(joined,mode==='ko'?/보고서/:/report|document/i);
   if(fixture.semantic){for(const pattern of fixture.semantic[mode==='ko'?'ko':'en'])assert.match(joined,new RegExp(pattern,'i'));for(const pattern of (fixture.forbid?.[mode==='ko'?'ko':'en']??[]))assert.doesNotMatch(joined,new RegExp(pattern,'i'));}
   else {assert.match(joined,mode==='ko'?/오늘/:/today/i);assert.doesNotMatch(joined,/\b(?:not|never|tomorrow)\b/i);}
   if(config.requireSynonym)assert.match(emphasized.map(s=>s.text).join(''),/^(verify|review)$/i);
   if(config.requireContextRewrite){
    const before=spans.slice(0,highlight).map(s=>s.text).join('').split(/[.;!?]/).at(-1).trimStart();
    const after=spans.slice(highlight+1).map(s=>s.text).join('').split(/[.;!?]/)[0].trimEnd();
    const at=source.lastIndexOf('check');assert.ok(at>=0);
    const originalBefore=source.slice(0,at).split(/[.;!?]/).at(-1).trimStart();
    const originalAfter=source.slice(at+5).split(/[.;!?]/)[0].trimEnd();
    assert.notEqual(before+'@'+after,originalBefore+'@'+originalAfter,'must exercise changed target-clause context, not only changed surrounding paragraphs');
    if(config.requireBothContextRewrite){assert.notEqual(before,originalBefore);assert.notEqual(after,originalAfter);}
    assert.match(joined,/today(?:'s)?/);
   }
  }else if(config.corpus==='korean-names'){
   const highlight=spans.findIndex(s=>s.original==='Original styled text: '+fixture.styledWord);
   assert.ok(highlight>=0);assert.ok(spans.slice(0,highlight).map(s=>s.text).join('').includes(fixture.literal),'quoted same word must remain outside action emphasis');
   assert.equal(emphasized.length,1,'one exact action emphasis');
   assert.doesNotMatch(text,/부탁.*부탁|제발.*부탁/,'no duplicated courtesy');
  }else assert.match(text,mode==='ko'?/달러/:/dollar/i);
  const t=performance.now();await page.getByRole('button',{name:'Accept',exact:true}).click();await page.locator('#grammar-review-host [role=status]').filter({hasText:/^Applied$/}).waitFor();sample.acceptMs=performance.now()-t;
  const save=async suffix=>{const download=page.waitForEvent('download',{timeout:15000});await inputBox.focus();await inputBox.press('Control+s');const d=await download;const path=r+'/'+fixture.id+'-'+scenario+'-'+format+'-'+mode+'-'+suffix+'.'+format;await d.saveAs(path);return path;};
  const saved=await save('accepted');const after=await snapshot(saved);assert.equal(after.length,before.length);
  const blocks=text.split('\n\n');assert.equal(blocks.length,selected.length);
  let runId=0;
  for(let p=0;p<before.length;p++){
   if(p<1||p>last){assert.deepEqual(after[p],before[p]);continue;}
   const a=p===1?from:0,b=p===last?to:before[p].chars.length;
   const runs=[];for(let i=a;i<b;i++){const prop=before[p].chars[i];if(!runs.length||JSON.stringify(runs.at(-1).prop)!==JSON.stringify(prop))runs.push({id:runId++,prop});}
   for(let i=0;i<runs.length;i++){const previous=runs.slice(0,i).find(r=>JSON.stringify(r.prop)===JSON.stringify(runs[i].prop));if(previous)runs[i].id=previous.id;}
   const ordered=spans.filter(x=>x.paragraph===p-1);assert.ok(ordered.length>=1&&ordered.length<=96);assert.equal(new Set(ordered.map(x=>x.id)).size,new Set(runs.map(x=>x.id)).size);
   const chars=Array.from(before[p].text);assert.equal(after[p].text,chars.slice(0,a).join('')+ordered.map(x=>x.text).join('')+chars.slice(b).join(''));assert.deepEqual(after[p].para,before[p].para);assert.deepEqual(after[p].style,before[p].style);
   assert.deepEqual(after[p].chars,[...before[p].chars.slice(0,a),...ordered.flatMap(x=>Array.from(x.text,()=>{const r=runs.find(r=>r.id===x.id);assert.ok(r);return r.prop;})),...before[p].chars.slice(b)]);
  }
  result.checks.push(scenario+' '+format+' '+mode+' Accept exact saved text+format');
  await inputBox.focus();await inputBox.press('Control+z');const undone=await save('undo');assert.deepEqual(await snapshot(undone),before);result.checks.push(scenario+' '+format+' '+mode+' Undo exact saved text+format');
  const savedGeneration=await page.locator('#grammar-review-host').getAttribute('data-grammar-generation');await page.locator('#file-input').setInputFiles(saved);await page.waitForFunction(old=>!!document.querySelector('#grammar-review-host')&&!document.documentElement.classList.contains('rhwp-busy')&&document.querySelector('#grammar-review-host').dataset.grammarGeneration!==old,savedGeneration);const reopen=await save('reopened');assert.deepEqual(await snapshot(reopen),after);result.checks.push(scenario+' '+format+' '+mode+' reopen exact text+format');result.samples.push(sample);delete result.pendingSample;
 }
 for(const format of ['hwp','hwpx']){
  await page.locator('#file-input').setInputFiles(r+'/protected-synthetic.'+format);
  await page.locator('#sb-message').filter({hasText:'DOCUMENT_PROTECTION_UNKNOWN_OR_UNSUPPORTED'}).waitFor();
  assert.equal(await page.getByRole('button',{name:'Accept',exact:true}).count(),0);
  result.checks.push(format+' protected original bytes refused before editor admission');
 }
 assert.deepEqual(result.externalRequests,[]);result.checks.push('no HTTP document or model bridge requests');
 result.status='PASS';
}catch(e){result.error=String(e.stack??e);if(page)result.localCode=await page.locator('#grammar-review-host [data-local-result-code]').getAttribute('data-local-result-code').catch(()=>null);if(page)result.visible=await page.locator('body').innerText().catch(()=> 'unavailable');}
finally{if(browser)await browser.close();await writeFile(r+'/installed-receipt.json',JSON.stringify(result,null,2));}
console.log(result.status,result.checks.length);process.exitCode=result.status==='PASS'?0:1;
