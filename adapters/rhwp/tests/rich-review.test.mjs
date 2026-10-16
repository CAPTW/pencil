import test from 'node:test';
import assert from 'node:assert/strict';
import {richEnvelope,richReviewText,parseRichReply,reviewedRuns,createRichReview} from '../rich-review.mjs';
const paragraphs=[{runs:[{id:0,text:'Do not alter the price.',shape:1},{id:1,text:' Total: 25 dollars.',shape:2}]},{runs:[{id:2,text:'Send it today.',shape:3}]}];
test('full-context envelope preserves ordered paragraph and style identities',()=>{const e=richEnvelope(paragraphs);assert.equal(parseRichReply(e,paragraphs),richReviewText(paragraphs));assert.equal(reviewedRuns(richReviewText(paragraphs),paragraphs)[0].runs[1].shape,2);});
test('missing reordered injected and duplicated markers refuse',()=>{const e=richEnvelope(paragraphs);for(const value of [e.replace('<r0>','<r1>'),e.replace('</p0>',''),e+e,e.replace('25','<img>25'),e.replace('25','&unknown;25')])assert.throws(()=>parseRichReply(value,paragraphs),/RICH_STRUCTURE_CHANGED/);});
test('markup source escaped without entity double decode',()=>{const p=[{runs:[{id:0,text:'x < y & &lt;',shape:1}]}];assert.equal(parseRichReply(richEnvelope(p),p),'x < y & &lt;');});
test('manual deletion of format or paragraph boundary refuses before mutation',()=>{for(const s of ['one line','a\nb\nc','a\n\nb\n\nc','a\n\n\nc'])assert.throws(()=>reviewedRuns(s,paragraphs),/RICH_STRUCTURE_CHANGED/);});
function fixture(render=async()=>{}){
 let selection={start:{sectionIndex:0,paragraphIndex:0,charOffset:0},end:{sectionIndex:0,paragraphIndex:1,charOffset:6}},revision=0,allowed=true,mutations=0,reads=0;
 const rows=[{text:'AB.CD.',ids:[1,1,1,2,2,2]},{text:'EF.GH.',ids:[3,3,3,4,4,4]},{text:'outside',ids:Array(7).fill(5)}];
 const w={documentGeneration:1,getSectionCount:()=>1,getParagraphCount:()=>rows.length,getParagraphLength:(s,p)=>rows[p].text.length,getTextRange:(s,p,a,n)=>{reads++;return rows[p].text.slice(a,a+n);},getControlTextPositions:()=>[],getFieldInfoAt:()=>({inField:false}),getCharPropertiesAt:(s,p,i)=>({charShapeId:rows[p].ids[i]}),getParaPropertiesAt:()=>({paraShapeId:7}),getStyleAt:()=>({id:0,name:'Normal'}),replaceText(s,p,a,n,t){mutations++;rows[p]={text:t,ids:Array(t.length).fill(0)};return {ok:true,newLength:t.length};},setCharShapeId(s,p,a,b,id){rows[p].ids.fill(id,a,b);},setParaShapeId(){}};
 const input={grammarNoRenderRollback:true,getSelection:()=>selection,async executeDocumentAgentOperation(d,render){d.operation(w);revision++;await render();}};
 class Controller{getDocumentState(){return {revision,image:JSON.stringify(rows)};}dispose(){}}
 const api={w,rows,create:()=>createRichReview(Controller,{wasm:w,input,render:()=>render(rows)},()=>{if(!allowed)throw Error('disabled');}),move:()=>{selection.end.charOffset--;},edit:()=>{rows[2].text='user';},disable:()=>{allowed=false;},counts:()=>({mutations,reads})};return api;
}
test('production rich operation applies all bound runs and preserves non-target',async()=>{const f=fixture(),r=f.create();await r.accept('One\nTwo\n\nThree\nFour');assert.deepEqual(f.rows.map(x=>x.text),['OneTwo','ThreeFour','outside']);assert.deepEqual(f.rows[0].ids,[1,1,1,2,2,2]);assert.equal(f.counts().mutations,2);assert.equal(r.valid(),false);});
test('selection changes, external edits, revocation and broken topology never mutate',async()=>{for(const scenario of ['move','edit','disable','topology']){const f=fixture(),r=f.create();if(scenario!=='topology')f[scenario]();await assert.rejects(()=>r.accept(scenario==='topology'?'one':'One\nTwo\n\nThree\nFour'));assert.equal(f.counts().mutations,0);}});
test('fields rejected before target text reads',()=>{const f=fixture();f.w.getFieldInfoAt=()=>({inField:true});assert.throws(f.create,/RICH_RANGE_UNSUPPORTED/);assert.deepEqual(f.counts(),{mutations:0,reads:0});});

test('render failure never restores over a later application edit',async()=>{const f=fixture(async rows=>{rows[2].text='later user edit';throw Error('render');}),r=f.create();await assert.rejects(()=>r.accept('One\nTwo\n\nThree\nFour'),/RICH_APPLY_UNCERTAIN/);assert.equal(f.rows[2].text,'later user edit');assert.equal(r.valid(),false);});
test('successful render with unexpected external edit cannot report Applied',async()=>{const f=fixture(async rows=>{rows[2].text='later user edit';}),r=f.create();await assert.rejects(()=>r.accept('One\nTwo\n\nThree\nFour'),/RICH_APPLY_UNCERTAIN/);assert.equal(f.rows[2].text,'later user edit');});

test('unqualified mid-sentence style boundary refuses before mutation',()=>{const f=fixture();f.rows[0].text='AB CD.';assert.throws(f.create,/RICH_INLINE_BOUNDARY_UNQUALIFIED/);assert.equal(f.counts().mutations,0);});
