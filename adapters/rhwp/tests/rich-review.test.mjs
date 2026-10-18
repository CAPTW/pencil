import test from 'node:test';
import assert from 'node:assert/strict';
import {richEnvelope,richReviewText,parseRichReply,parseRichDraft,reviewedRuns,createRichReview} from '../rich-review.mjs';
const paragraphs=[{runs:[{id:0,text:'Do not alter the price.',shape:1},{id:1,text:' Total: 25 dollars.',shape:2}]},{runs:[{id:2,text:'Send it today.',shape:3}]}];
test('full-context envelope preserves ordered paragraph and style identities',()=>{const e=richEnvelope(paragraphs);assert.equal(parseRichReply(e,paragraphs),richReviewText(paragraphs));assert.equal(reviewedRuns(richReviewText(paragraphs),paragraphs)[0].runs[1].shape,2);});
test('missing foreign injected and duplicated paragraphs refuse',()=>{const e=richEnvelope(paragraphs);for(const value of [e.replace('<r0>','<r1>'),e.replace('</p0>',''),e+e,e.replace('25','<img>25'),e.replace('25','&unknown;25')])assert.throws(()=>parseRichReply(value,paragraphs),/RICH_STRUCTURE_CHANGED/);});
test('markup source escaped without entity double decode',()=>{const p=[{runs:[{id:0,text:'x < y & &lt;',shape:1}]}];assert.equal(parseRichReply(richEnvelope(p),p),'x < y & &lt;');});
test('manual deletion of format or paragraph boundary refuses before mutation',()=>{for(const s of ['one line','a\nb\nc','a\n\nb\n\nc','a\n\n\nc'])assert.throws(()=>reviewedRuns(s,paragraphs),/RICH_STRUCTURE_CHANGED/);});
function fixture(render=async()=>{}){
 let selection={start:{sectionIndex:0,paragraphIndex:0,charOffset:0},end:{sectionIndex:0,paragraphIndex:1,charOffset:6}},revision=0,allowed=true,mutations=0,reads=0;
 const rows=[{text:'AB.CD.',ids:[1,1,1,2,2,2]},{text:'EF.GH.',ids:[3,3,3,4,4,4]},{text:'outside',ids:Array(7).fill(5)}];
 const w={documentGeneration:1,getSectionCount:()=>1,getParagraphCount:()=>rows.length,getParagraphLength:(s,p)=>rows[p].text.length,getTextRange:(s,p,a,n)=>{reads++;return rows[p].text.slice(a,a+n);},getControlTextPositions:()=>[],getFieldInfoAt:()=>({inField:false}),getCharPropertiesAt:(s,p,i)=>({charShapeId:rows[p].ids[i]}),getParaPropertiesAt:()=>({paraShapeId:7}),getStyleAt:()=>({id:0,name:'Normal'}),replaceText(s,p,a,n,t){mutations++;rows[p]={text:t,ids:Array(t.length).fill(0)};return {ok:true,newLength:t.length};},setCharShapeId(s,p,a,b,id){rows[p].ids.fill(id,a,b);},setParaShapeId(){}};
 const input={grammarNoRenderRollback:true,getSelection:()=>selection,async executeDocumentAgentOperation(d,render){d.operation(w);revision++;await render();}};
 class Controller{getDocumentState(){return {revision,image:JSON.stringify(rows)};}dispose(){}}
 const api={w,rows,setSelection:(a,b)=>{selection={start:a,end:b};},create:()=>createRichReview(Controller,{wasm:w,input,render:()=>render(rows)},()=>{if(!allowed)throw Error('disabled');}),move:()=>{selection.end.charOffset--;},edit:()=>{rows[2].text='user';},disable:()=>{allowed=false;},counts:()=>({mutations,reads})};return api;
}
test('production rich operation applies all bound runs and preserves non-target',async()=>{const f=fixture(),r=f.create();await r.accept('One\nTwo\n\nThree\nFour');assert.deepEqual(f.rows.map(x=>x.text),['OneTwo','ThreeFour','outside']);assert.deepEqual(f.rows[0].ids,[1,1,1,2,2,2]);assert.equal(f.counts().mutations,2);assert.equal(r.valid(),false);});
test('selection changes, external edits, revocation and broken topology never mutate',async()=>{for(const scenario of ['move','edit','disable','topology']){const f=fixture(),r=f.create();if(scenario!=='topology')f[scenario]();await assert.rejects(()=>r.accept(scenario==='topology'?'one':'One\nTwo\n\nThree\nFour'));assert.equal(f.counts().mutations,0);}});
test('fields rejected before target text reads',()=>{const f=fixture();f.w.getFieldInfoAt=()=>({inField:true});assert.throws(f.create,/RICH_RANGE_UNSUPPORTED/);assert.deepEqual(f.counts(),{mutations:0,reads:0});});

test('render failure never restores over a later application edit',async()=>{const f=fixture(async rows=>{rows[2].text='later user edit';throw Error('render');}),r=f.create();await assert.rejects(()=>r.accept('One\nTwo\n\nThree\nFour'),/RICH_APPLY_UNCERTAIN/);assert.equal(f.rows[2].text,'later user edit');assert.equal(r.valid(),false);});
test('successful render with unexpected external edit cannot report Applied',async()=>{const f=fixture(async rows=>{rows[2].text='later user edit';}),r=f.create();await assert.rejects(()=>r.accept('One\nTwo\n\nThree\nFour'),/RICH_APPLY_UNCERTAIN/);assert.equal(f.rows[2].text,'later user edit');});

test('inline and partial selection preserve outside characters and properties',async()=>{const f=fixture();f.rows[0].text='AB CD.';f.setSelection({sectionIndex:0,paragraphIndex:0,charOffset:1},{sectionIndex:0,paragraphIndex:0,charOffset:5});const r=f.create();assert.equal(r.source,'<p0><r0>B </r0><r1>CD</r1></p0>');await r.accept('b \nc');assert.equal(f.rows[0].text,'Ab c.');assert.deepEqual(f.rows[0].ids,[1,1,1,2,2]);assert.equal(f.rows[1].text,'EF.GH.');});
test('translated order carries original properties and missing identity refuses',async()=>{const f=fixture(),r=f.create();const draft=parseRichDraft('<p0><r1>two</r1><r0>one</r0></p0><p1><r3>four</r3><r2>three</r2></p1>',r.richParagraphs);assert.throws(()=>r.bindDraft({...draft,runOrder:[[0,0],[2,3]]}),/RICH_STRUCTURE_CHANGED/);await r.accept(r.bindDraft(draft));assert.equal(f.rows[0].text,'twoone');assert.deepEqual(f.rows[0].ids,[2,2,2,1,1,1]);});
test('cross paragraph identities cannot be moved',()=>{assert.throws(()=>parseRichDraft('<p0><r2>x</r2><r1>y</r1></p0><p1><r0>z</r0></p1>',paragraphs),/RICH_STRUCTURE_CHANGED/);});
test('partial range never sends unselected prefix or suffix to inference',()=>{const f=fixture();f.setSelection({sectionIndex:0,paragraphIndex:0,charOffset:1},{sectionIndex:0,paragraphIndex:1,charOffset:2});const r=f.create();assert.equal(r.source,'<p0><r0>B.</r0><r1>CD.</r1></p0>\n<p1><r2>EF</r2></p1>');});

test('discontinuous translated spans retain the same original style safely',async()=>{const f=fixture(),r=f.create();const parsed=parseRichDraft('<p0><r0>A</r0><r1>B</r1><r0>C</r0></p0><p1><r2>D</r2><r3>E</r3></p1>',r.richParagraphs);await r.accept(r.bindDraft(parsed));assert.equal(f.rows[0].text,'ABC');assert.deepEqual(f.rows[0].ids,[1,2,1]);});

test('split-span amplification and modified source labels refuse',()=>{const p=[{runs:[{id:0,tag:'r0_source',text:'source'}]}];assert.throws(()=>parseRichDraft('<p0>'+('<r0_source>x</r0_source>'.repeat(97))+'</p0>',p),/RICH_STRUCTURE_CHANGED/);assert.throws(()=>parseRichDraft('<p0><r0_other>x</r0_other></p0>',p),/RICH_STRUCTURE_CHANGED/);});

test('identical noncontiguous styles share one class without changing outside text',async()=>{const f=fixture();f.setSelection({sectionIndex:0,paragraphIndex:0,charOffset:0},{sectionIndex:0,paragraphIndex:0,charOffset:6});f.rows[0].ids=[1,1,2,2,1,1];const r=f.create();assert.deepEqual(r.richParagraphs[0].runs.map(r=>r.id),[0,1,0]);const draft=parseRichDraft('<p0><r1>bold</r1><r0> plain</r0></p0>',r.richParagraphs);await r.accept(r.bindDraft(draft));assert.deepEqual(f.rows[0].ids,[2,2,2,2,1,1,1,1,1,1]);assert.equal(f.rows[1].text,'EF.GH.');});


test('exact anchor alignment keeps fixed text and refuses ambiguity or overlap',async()=>{
 const {parseRichAlignment}=await import('../rich-review.mjs');const p=[{runs:[{id:0,text:'Please re'},{id:1,text:'check'},{id:0,text:' the amount.'}]}];
 assert.deepEqual(parseRichAlignment('<p0><r1>verify</r1></p0>',p,'Please verify the amount again.'),{text:'Please \nverify\n the amount again.',runOrder:[[0,1,0]]});
 for(const [reply,fixed] of [['<p0><r1>missing</r1></p0>','Please verify.'],['<p0><r1>verify</r1></p0>','verify then verify'],['<p0><r1>verify</r1><r1>verify it</r1></p0>','Please verify it.'],['<p0><r99>verify</r99></p0>','Please verify.'],['<p0></p0>','Please verify.'],['<p0><r0>Please</r0><r1>verify</r1></p0>','Please verify.']])assert.throws(()=>parseRichAlignment(reply,p,fixed),/RICH_/);
});


test('alignment retains quoted fixed text byte for byte while styling a word',async()=>{const {parseRichAlignment}=await import('../rich-review.mjs');const p=[{runs:[{id:0,text:'Keep the quoted literal and '},{id:1,text:'check'}]}];const fixed='Keep "speling" and verify.';const r=parseRichAlignment('<p0><r1>verify</r1></p0>',p,fixed);assert.equal(r.text.split('\n').join(''),fixed);});


test('style anchors cannot split a surrogate pair or cross paragraph IDs',async()=>{const {parseRichAlignment}=await import('../rich-review.mjs');const p=[{runs:[{id:0,text:'normal surrounding text'},{id:1,text:'x'}]}];assert.throws(()=>parseRichAlignment('<p0><r1>\uD83D</r1></p0>',p,'a \uD83D\uDE00 b'),/RICH_/);assert.deepEqual(parseRichAlignment('<p0><r1>\uD83D\uDE00</r1></p0>',p,'a \uD83D\uDE00 b'),{text:'a \n\uD83D\uDE00\n b',runOrder:[[0,1,0]]});assert.throws(()=>parseRichAlignment('<p0><r2>x</r2></p0><p1><r1>y</r1></p1>',[...p,{runs:[{id:2,text:'y'}]}],'a x b\n\ny'),/RICH_/);});


test('repeated anchors require explicit bounded occurrence and retain other instances',async()=>{const {parseRichAlignment}=await import('../rich-review.mjs');const p=[{runs:[{id:0,text:'check first; then '},{id:1,text:'check'},{id:0,text:' last.'}]}];const fixed='check first; then check last.';assert.deepEqual(parseRichAlignment('<p0><r1 occurrence="2">check</r1></p0>',p,fixed),{text:'check first; then \ncheck\n last.',runOrder:[[0,1,0]]});for(const v of ['0','3','97','-1','1.5','01'])assert.throws(()=>parseRichAlignment(`<p0><r1 occurrence="${v}">check</r1></p0>`,p,fixed),/RICH_/);assert.throws(()=>parseRichAlignment('<p0><r1>check</r1></p0>',p,fixed),/RICH_/);assert.throws(()=>parseRichAlignment('<p0><r1 occurrence="2">check</r1><r1 occurrence="2">check</r1></p0>',p,fixed),/RICH_/);});


test('alignment refuses surrounding prose even when the model wraps a correct anchor',async()=>{
 const {parseRichAlignment}=await import('../rich-review.mjs');const p=[{runs:[{id:0,text:'Do not check; '},{id:1,text:'check'},{id:0,text:' later.'}]}];
 assert.throws(()=>parseRichAlignment('<p0>Do not check; <r1 occurrence="2">check</r1> later.</p0>',p,'Do not check; check later.'),/RICH_/);
});
