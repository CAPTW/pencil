import assert from 'node:assert/strict';

// Real UI gestures and document rereads; only timing observers are injected.
export async function runRewriteUI(page,worker,cases,evidence={}) {
 const rows=evidence.rows=[];
 const selectMode=async value=>{
  const select=page.getByLabel('Local review mode',{exact:true});
  await select.focus();await select.press('Home');
  for(let n=0;n<['paraphrase','en','ko'].indexOf(value);n++)await select.press('ArrowDown');
  assert.equal(await select.inputValue(),value);
 };
 const waitResponse=async count=>{
  for(let n=0;n<100;n++) {if(await worker.evaluate(n=>gemmaMetrics.responses>n,count))return;await page.waitForTimeout(50);}
  throw Error('local response unavailable');
 };
 for(const [index,row] of cases.entries()) {
  const field=index%2?'editable':'writing',el=page.locator('#'+field);
  await page.getByRole('button',{name:'Pause field',exact:true}).click();
  await el.fill(row.source);await el.click();
  const responses=await worker.evaluate(()=>gemmaMetrics.responses);
  await page.getByRole('button',{name:'Enable this field',exact:true}).click();
  await waitResponse(responses);
  const choice=row.intent.mode==='paraphrase'?'paraphrase':row.intent.target;
  await selectMode(choice);
  await page.evaluate(()=>{
   const shadow=document.getElementById('grammar-local-assist').shadowRoot;
   const b=[...shadow.querySelectorAll('button')].find(b=>b.textContent==='Review locally');
   window.rewriteTiming={};
   b.addEventListener('click',()=>{
    const start=performance.now();
    const observer=new MutationObserver(()=>{
     if(shadow.querySelector('[aria-label="Cached suggestions"] button')){
      window.rewriteTiming.recommendMs=performance.now()-start;observer.disconnect();
     }
    });observer.observe(shadow,{subtree:true,childList:true});
    setTimeout(()=>observer.disconnect(),5000);
   },{capture:true,once:true});
  });
  await page.getByRole('button',{name:'Review locally',exact:true}).click();
  await page.getByRole('button',{name:/Suggestion 1:/}).click();
  const edit=page.getByRole('textbox',{name:'Edit replacement',exact:true});
  const replacement=await edit.inputValue();
  assert.deepEqual(await worker.evaluate(()=>gemmaMetrics.lastIntent),row.intent);
  const count=await worker.evaluate(()=>gemmaMetrics.requests);
  const before=await el.evaluate(e=>e.tagName==='TEXTAREA'?e.value:e.textContent);
  assert.equal(before,row.source,'review must not mutate source');
  if(index===0) {
   await edit.fill(replacement+' ');
   await page.getByLabel('Local review mode',{exact:true}).press('ArrowUp');
   assert.equal(await page.getByLabel('Local review mode',{exact:true}).inputValue(),choice);
   assert.equal(await edit.inputValue(),replacement+' ');
   await edit.fill(replacement);
  }
  const appliedText=index===1?replacement+' Reviewed.':replacement;
  const action=index===1?'Apply edit':'Accept';
  if(index===1)await edit.fill(appliedText);
  await page.evaluate(({field,action})=>{
   const shadow=document.getElementById('grammar-local-assist').shadowRoot;
   const b=[...shadow.querySelectorAll('button')].find(b=>b.textContent===action);
   b.addEventListener('click',()=>{const start=performance.now();setTimeout(()=>{
    const el=document.getElementById(field);window.rewriteTiming.applyMs=performance.now()-start;
    window.rewriteTiming.reread=el.tagName==='TEXTAREA'?el.value:el.textContent;
   });},{capture:true,once:true});
  },{field,action});
  await page.getByRole('button',{name:action,exact:true}).click();
  await page.getByRole('button',{name:'Pause field',exact:true}).click();
  await page.waitForFunction(()=>window.rewriteTiming.applyMs!==undefined);
  const timing=await page.evaluate(()=>window.rewriteTiming);
  assert.equal(timing.reread,appliedText);
  assert.equal(await el.evaluate(e=>e.tagName==='TEXTAREA'?e.value:e.textContent),appliedText);
  assert.equal(await worker.evaluate(()=>gemmaMetrics.requests),count,'cache/Accept cannot infer');
  rows.push({id:row.id,field,intent:row.intent,source:row.source,replacement,action,appliedText,...timing});
 }
 // Switch target while inference is running: late response cannot repopulate cache.
 await page.locator('#writing').fill('Do not approve the release yet.');await page.locator('#writing').click();
 const before=await worker.evaluate(()=>gemmaMetrics.responses);
 await page.getByRole('button',{name:'Enable this field',exact:true}).click();
 await waitResponse(before);
 await selectMode('ko');
 const pending=await worker.evaluate(()=>gemmaMetrics.responses);
 await page.getByRole('button',{name:'Review locally',exact:true}).click();
 await selectMode('en');
 await waitResponse(pending);
 await page.waitForTimeout(80);
 assert.equal(await page.getByRole('button',{name:/Suggestion 1:/}).count(),0);
 assert.equal(await page.locator('#writing').inputValue(),'Do not approve the release yet.');
 const sent=await worker.evaluate(()=>gemmaMetrics.requests);
 await page.locator('#password').click();
 await page.getByRole('button',{name:'Review locally',exact:true}).click();
 assert.equal(await worker.evaluate(()=>gemmaMetrics.requests),sent,'sensitive focus cannot redirect to old field');
 await page.getByRole('button',{name:'Pause field',exact:true}).click();
 return Object.assign(evidence,{rows,checks:['dirty draft prevents mode change','mode change discards in-flight result','sensitive focus refuses old field','exact reread after Accept','cache/Accept sends no inference'],
  latencyPass:rows.every(r=>r.recommendMs<=1000&&r.applyMs<=500)});
}
