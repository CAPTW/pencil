import assert from 'node:assert/strict';

// Real production extension scheduling + native transport + Gemma. The caller
// owns the isolated profile/registration and reports fixture modifications.
export async function runIncremental(page, worker, evidence, rounds=1) {
  assert.ok(Number.isInteger(rounds)&&rounds>=1&&rounds<=3);
  evidence.rows=[];
  const cases=[
    {id:'quantity',first:'She have two books.',latest:'She have three books.',expected:'She has three books.'},
    {id:'negation',first:'He do not agree.',latest:'He do not agree yet.',expected:'He does not agree yet.'},
    {id:'past-date',first:'They was here yesterday.',latest:'They was here on Monday.',expected:'They were here on Monday.'},
  ];
  for(let round=1;round<=rounds;round++) for(const field of ['writing','editable']) for(const sample of cases) {
    const row={round,field,case:sample.id,status:'RUNNING'};evidence.rows.push(row);
    await page.locator('#'+field).click();
    await page.getByRole('button',{name:'Enable this field',exact:true}).click();
    const before=await worker.evaluate(()=>({requests:gemmaMetrics.requests,supersessions:gemmaMetrics.supersessions}));
    await page.locator('#'+field).fill(sample.first);
    const admissionDeadline=Date.now()+5000;
    while(await worker.evaluate(()=>gemmaMetrics.requests)<=before.requests) {
      assert.ok(Date.now()<admissionDeadline,'local request was not admitted');
      await page.waitForTimeout(10);
    }
    await armRecommendationClock(page,field);
    const started=performance.now();
    await page.locator('#'+field).fill(sample.latest);
    try {
      const suggestion=page.getByRole('button',{name:/Suggestion 1:/});
      await suggestion.waitFor({timeout:12000});
      row.automationWaitMs=performance.now()-started;
      Object.assign(row,await readRecommendationClock(page));
      const count=await worker.evaluate(()=>gemmaMetrics.requests);
      await suggestion.hover();await suggestion.click();
      const edit=page.getByRole('textbox',{name:'Edit replacement',exact:true});
      row.output=await edit.inputValue();
      assert.equal(row.output,sample.expected,'only the latest source may publish');
      assert.equal(await worker.evaluate(()=>gemmaMetrics.requests),count,'cache lookup must not infer');
      const accept=performance.now();
      await page.getByRole('button',{name:'Accept',exact:true}).click();
      row.actual=await page.locator('#'+field).evaluate(el=>el.tagName==='TEXTAREA'?el.value:el.textContent);
      row.acceptMs=performance.now()-accept;
      assert.equal(row.actual,row.output);
      await page.getByRole('button',{name:'Pause field',exact:true}).click();
      assert.equal(await worker.evaluate(()=>gemmaMetrics.requests),count,'Accept must not infer');
      row.metrics=await worker.evaluate(()=>({requests:gemmaMetrics.requests,supersessions:gemmaMetrics.supersessions,cancelled:gemmaMetrics.cancelled,timing:gemmaMetrics.timing}));
      assert.ok(row.metrics.supersessions>before.supersessions,'must exercise an active supersession');
      row.status='PASS';
    } catch(error) {
      row.status='FAIL';row.error=String(error);
      row.metrics=await worker.evaluate(()=>({requests:gemmaMetrics.requests,supersessions:gemmaMetrics.supersessions,cancelled:gemmaMetrics.cancelled,last:gemmaMetrics.last,timing:gemmaMetrics.timing}));
      throw error;
    }
  }
  evidence.latencyPass=evidence.rows.every(row=>row.recommendMs<=1000&&row.acceptMs<=500);
  evidence.misses=evidence.rows.filter(row=>row.recommendMs>1000||row.acceptMs>500).map(row=>({field:row.field,case:row.case}));
  evidence.latency=Object.fromEntries(['recommendMs','acceptMs'].map(key=>{
    const values=evidence.rows.map(row=>row[key]).sort((a,b)=>a-b);
    return [key,{n:values.length,p50:values[Math.ceil(values.length*.5)-1],p95:values[Math.ceil(values.length*.95)-1],p99:values[Math.ceil(values.length*.99)-1],max:values.at(-1)}];
  }));
}

// Observe page input and suggestion layout in the same clock domain. Locator
// waitFor uses adaptive polling and is retained separately, not called UI latency.
export async function armRecommendationClock(page,field) {
  await page.evaluate(field=>{
    window.pencilClock?.dispose();
    const target=document.getElementById(field);
    const shadow=document.getElementById('grammar-local-assist').shadowRoot;
    const clock={};let frame=0;
    const input=e=>{if(e.target===target){clock.inputAt=performance.now();delete clock.domAt;delete clock.frameAt;}};
    document.addEventListener('input',input,true);
    const observer=new MutationObserver(()=>{
      const button=shadow.querySelector('[aria-label^="Suggestion 1:"]');
      if(clock.inputAt===undefined||clock.domAt!==undefined||!button||!button.getClientRects().length)return;
      clock.domAt=performance.now();const inputAt=clock.inputAt;
      frame=requestAnimationFrame(()=>{
        if(clock.inputAt===inputAt&&button.isConnected&&button.getClientRects().length)clock.frameAt=performance.now();
      });
    });
    observer.observe(shadow,{subtree:true,childList:true});
    clock.dispose=()=>{document.removeEventListener('input',input,true);observer.disconnect();cancelAnimationFrame(frame);};
    window.pencilClock=clock;
  },field);
}
export async function readRecommendationClock(page) {
  await page.waitForFunction(()=>Number.isFinite(window.pencilClock?.frameAt),null,{timeout:12000});
  return page.evaluate(()=>{
    const c=window.pencilClock;
    const result={recommendMs:c.frameAt-c.inputAt,domAvailableMs:c.domAt-c.inputAt,measurement:'input-event-to-visible-suggestion-rAF'};
    c.dispose();return result;
  });
}
