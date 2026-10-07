import assert from 'node:assert/strict';

export async function runCorpus(page,worker,corpus) {
 assert.equal(corpus.version,1);assert.equal(corpus.recommendationBudgetMs,1000);assert.equal(corpus.applyBudgetMs,500);
 const rows=[];
 try { for(const test of corpus.cases) {
  const field=rows.length%2 ? 'editable' : 'writing';
  const row={id:test.id,field,source:test.source,expected:test.expected,status:'STARTED'};rows.push(row);
  await page.evaluate(field=>{const el=document.getElementById(field);if(el.tagName==='TEXTAREA')el.value='';else el.textContent='';},field);
  await page.locator('#'+field).click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
  await worker.evaluate(()=>{gemmaMetrics.last=null;gemmaMetrics.timing=null;});
  await page.evaluate(({field,source})=>{
   const el=document.getElementById(field),shadow=document.getElementById('grammar-local-assist').shadowRoot;
   window.corpusTrial={};
   const observer=new MutationObserver(()=>{
    const status=shadow.querySelector('[role="status"]')?.textContent||'';
    if(status.includes('suggestions')||status.includes('unavailable')||status.includes('Connection lost')) {
     observer.disconnect();requestAnimationFrame(()=>{window.corpusTrial.done={ms:performance.now()-window.corpusTrial.start,status,startWall:window.corpusTrial.startWall,paintWall:Date.now()};});
    }
   });observer.observe(shadow,{subtree:true,childList:true,characterData:true});
   if(el.tagName==='TEXTAREA')el.value=source;else el.textContent=source;
   window.corpusTrial.startWall=Date.now();window.corpusTrial.start=performance.now();el.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertText',data:source}));
  },{field,source:test.source});
  await page.waitForFunction(()=>!!window.corpusTrial?.done,null,{timeout:7000});
  const done=await page.evaluate(()=>window.corpusTrial.done);
  const response=await worker.evaluate(()=>gemmaMetrics.last);
  Object.assign(row,{recommendMs:done.ms,status:done.status,response});
  const timing=await worker.evaluate(()=>gemmaMetrics.timing);
  if(timing&&Number.isFinite(timing.sent))row.stagesMs={inputToNativeSend:timing.sent-done.startWall,nativeRoundTrip:timing.received-timing.sent,responseToPaint:done.paintWall-timing.received,
   inputToContentSend:timing.contentSent-done.startWall,contentToWorker:timing.workerReceived-timing.contentSent,workerToNativeSend:timing.sent-timing.workerReceived,
   observedStorageReads:timing.storage?.length,observedStorageMs:timing.storage?.reduce((n,x)=>n+x.end-x.start,0)};
  if(!response||response.error||!Array.isArray(response.suggestions)) {row.error='transport unavailable';break;}
  assert.ok(response.suggestions.length<=1);
  row.actual=response.suggestions.length?response.suggestions[0].replacement:test.source;
  row.qualityPass=row.actual===test.expected;
  row.falseEdit=test.source===test.expected&&row.actual!==test.source;
  if(response.suggestions.length) {
   const suggestion=page.getByRole('button',{name:/Suggestion 1:/});await suggestion.click();
   assert.equal(await page.getByRole('textbox',{name:'Edit replacement',exact:true}).inputValue(),row.actual);
   if(row.qualityPass) {
    await page.evaluate(field=>{
     const shadow=document.getElementById('grammar-local-assist').shadowRoot;
     [...shadow.querySelectorAll('button')].find(b=>b.textContent==='Accept').addEventListener('click',()=>{
      const start=performance.now();setTimeout(()=>{const el=document.getElementById(field);window.corpusTrial.applied={ms:performance.now()-start,text:el.tagName==='TEXTAREA'?el.value:el.textContent};});
     },{once:true,capture:true});
    },field);
    await page.getByRole('button',{name:'Accept',exact:true}).click();
    await page.waitForFunction(()=>!!window.corpusTrial.applied);
    const applied=await page.evaluate(()=>window.corpusTrial.applied);row.applyMs=applied.ms;row.reread=applied.text;assert.equal(applied.text,row.expected);
   }
  }
  await page.getByRole('button',{name:'Pause field',exact:true}).click();
 }
 } catch(error) { if(rows.length) rows.at(-1).error=String(error); else throw error; }
 return summarizeCorpus(rows,corpus.cases);
}

export function summarizeCorpus(rows,cases) {
 const stats=key=>{const values=rows.map(r=>r[key]).filter(Number.isFinite).sort((a,b)=>a-b);return {n:values.length,p50:values[Math.ceil(values.length*.5)-1],p95:values[Math.ceil(values.length*.95)-1],p99:values[Math.ceil(values.length*.99)-1],max:values.at(-1)};};
 return {rows,total:cases.length,notRun:cases.slice(rows.length).map(t=>t.id),qualityPass:rows.length===cases.length&&rows.every(r=>r.qualityPass&&!r.error),falseEdits:rows.filter(r=>r.falseEdit).length,recommendationMisses:rows.filter(r=>r.recommendMs>1000).map(r=>r.id),applyMisses:rows.filter(r=>r.applyMs>500).map(r=>r.id),latency:{recommend:stats('recommendMs'),apply:stats('applyMs')}};
}
