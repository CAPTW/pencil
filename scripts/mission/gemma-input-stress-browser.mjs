import assert from 'node:assert/strict';
import {armRecommendationClock,readRecommendationClock} from './gemma-incremental-browser.mjs';

export async function runInputStress(page,worker,evidence) {
 evidence.rows=[];
 const cdp=await page.context().newCDPSession(page);
 const middle='The team has reviewed the draft and confirmed the deadline. She have two books, but they are not ready to publish either one. The price remains $12.50, and the next meeting is scheduled for 3 p.m.';
 const corrected=middle.replace('She have','She has');
 const prefix='The first paragraph is unchanged.\n',suffix='\nThe final paragraph must remain unchanged.';
 const metrics=()=>worker.evaluate(()=>({requests:gemmaMetrics.requests,responses:gemmaMetrics.responses,supersessions:gemmaMetrics.supersessions,cancelled:gemmaMetrics.cancelled,timing:gemmaMetrics.timing,lastSource:gemmaMetrics.lastSource}));
 async function responseAfter(n) {
   const end=Date.now()+12000;while((await metrics()).responses<=n){assert.ok(Date.now()<end,'baseline response deadline');await page.waitForTimeout(20);}
 }
 try {
 for(const field of ['writing','editable'])for(const kind of ['continuous','ime-korean','long-window']) {
  const row={field,kind,status:'RUNNING'};evidence.rows.push(row);
  const el=page.locator('#'+field);let expected,expectedField;
  try {
   row.requestBase=(await metrics()).requests;
   if(field==='editable'&&kind==='long-window') {
    await el.fill(prefix+corrected+suffix);row.locatorFillNodes=await el.evaluate(e=>[...e.childNodes].map(n=>n.nodeName));
   }
   await el.fill('');await el.click();await page.getByRole('button',{name:'Enable this field',exact:true}).click();
   if(kind==='long-window') {
    const count=(await metrics()).responses;
    if(field==='editable') await el.evaluate((e,text)=>{e.textContent=text;e.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertText'}));},prefix+corrected+suffix);
    else await el.fill(prefix+corrected+suffix);
    await responseAfter(count);
    // A settled baseline makes the next change a real incremental middle line.
    await page.waitForTimeout(300);
    await armRecommendationClock(page,field);
    if(field==='editable') {
      // Seed one plain text node, then edit a word through Chromium input.
      // Multiline locator.fill creates DIV/BR markup outside this adapter scope.
      await el.evaluate(e=>{e.focus();const n=e.firstChild,pos=n.textContent.indexOf('She has')+4;const r=document.createRange();r.setStart(n,pos);r.setEnd(n,pos+3);const s=getSelection();s.removeAllRanges();s.addRange(r);});
      await page.keyboard.insertText('have');
    } else await el.fill(prefix+middle+suffix);
    expected=corrected;expectedField=prefix+corrected+suffix;row.changedChars=middle.length;row.documentChars=expectedField.length;
   } else if(kind==='continuous') {
    await armRecommendationClock(page,field);
    await el.pressSequentially('She have three books.',{delay:30});
    expected=expectedField='She has three books.';
   } else {
    const count=(await metrics()).requests;
    await armRecommendationClock(page,field);
    await el.focus();
    await cdp.send('Input.imeSetComposition',{text:'회의가 취소됬습니다.',selectionStart:'회의가 취소됬습니다.'.length,selectionEnd:'회의가 취소됬습니다.'.length});
    await page.waitForTimeout(800);
    assert.equal((await metrics()).requests,count,'preedit must not infer');
    assert.equal(await page.getByRole('button',{name:/Suggestion 1:/}).count(),0,'preedit must not publish');
    await cdp.send('Input.insertText',{text:'회의가 취소됬습니다.'});
    expected=expectedField='회의가 취소됐습니다.';
    row.imeScope='Chromium CDP trusted composition; physical Korean IME NOT_RUN';
   }
   const suggestion=page.getByRole('button',{name:/Suggestion 1:/});await suggestion.waitFor({timeout:12000});
   Object.assign(row,await readRecommendationClock(page));
   const before=await metrics();
   if(kind==='long-window')assert.equal(before.lastSource,middle,'only changed complete line may be analyzed');
   if(kind==='continuous')assert.equal(before.lastSource,expectedField.replace(' has ',' have '),'only final typing source');
   await suggestion.hover();await suggestion.click();
   row.output=await page.getByRole('textbox',{name:'Edit replacement',exact:true}).inputValue();assert.equal(row.output,expected);
   assert.equal((await metrics()).requests,before.requests,'cache must not infer');
   const start=performance.now();await page.getByRole('button',{name:'Accept',exact:true}).click();
   row.actual=await el.evaluate(e=>e.tagName==='TEXTAREA'?e.value:e.textContent);row.acceptMs=performance.now()-start;
   assert.equal(row.actual,expectedField,'exact full field and outside range preserved');
   row.metrics=await metrics();row.requestDelta=row.metrics.requests-row.requestBase;assert.equal(row.requestDelta,kind==='long-window'?2:1,'bounded request count');assert.equal(row.metrics.requests,before.requests,'Accept must not infer');row.status='PASS';
  }catch(e){row.status='FAIL';row.error=String(e);row.metrics=await metrics();}
  finally {
   await page.evaluate(()=>window.pencilClock?.dispose());
   const pause=page.getByRole('button',{name:'Pause field',exact:true});if(await pause.count())await pause.click();
  }
 }
 }finally{await cdp.detach();}
 evidence.functionalPass=evidence.rows.every(r=>r.status==='PASS');
 evidence.recommendationMisses=evidence.rows.filter(r=>r.recommendMs>1000).map(r=>({field:r.field,kind:r.kind,ms:r.recommendMs}));
 evidence.paragraphMisses=evidence.rows.filter(r=>r.kind==='long-window'&&r.recommendMs>3000);
 evidence.acceptMisses=evidence.rows.filter(r=>r.acceptMs>500);
 evidence.latencyPass=evidence.functionalPass&&!evidence.recommendationMisses.length&&!evidence.acceptMisses.length;
}
