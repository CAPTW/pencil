// Whole body-paragraph rich selection. No inferred character-position mapping.
// Model markup binds each original run; absent/reordered markers fail closed.
const fail=code=>{throw Object.assign(new Error(code),{code});};
const cp=s=>Array.from(s).length;
const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const esc=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;');
const unesc=s=>s.replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&amp;','&');
export function richEnvelope(paragraphs){return paragraphs.map((p,i)=>`<p${i}>`+p.runs.map(r=>`<r${r.id}>${esc(r.text)}</r${r.id}>`).join('')+`</p${i}>`).join('\n');}
export function richReviewText(paragraphs){return paragraphs.map(p=>p.runs.map(r=>r.text).join('\n')).join('\n\n');}
export function parseRichReply(text,paragraphs){
 if(typeof text!=='string'||cp(text)>6000)fail('RICH_STRUCTURE_CHANGED');
 let rest=text.trim();const result=[];
 for(let i=0;i<paragraphs.length;i++){
  const open=`<p${i}>`;if(!rest.startsWith(open))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(open.length);
  const runs=[];
  for(const run of paragraphs[i].runs){
   const a=`<r${run.id}>`,b=`</r${run.id}>`;if(!rest.startsWith(a))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(a.length);
   const end=rest.indexOf(b);if(end<0)fail('RICH_STRUCTURE_CHANGED');const raw=rest.slice(0,end);
   if(/[<>\r\n\u0000-\u001f\u007f]/u.test(raw)||/&(?!amp;|lt;|gt;)/u.test(raw))fail('RICH_STRUCTURE_CHANGED');
   const value=unesc(raw);if(!value.trim())fail('RICH_STRUCTURE_CHANGED');runs.push({...run,text:value});rest=rest.slice(end+b.length);
  }
  const close=`</p${i}>`;if(!rest.startsWith(close))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(close.length).trimStart();result.push({...paragraphs[i],runs});
 }
 if(rest||cp(richReviewText(result))>4000)fail('RICH_STRUCTURE_CHANGED');return richReviewText(result);
}
export function reviewedRuns(text,paragraphs){
 if(typeof text!=='string'||cp(text)>4000||/[\r\u0000-\u0009\u000b-\u001f\u007f]/u.test(text))fail('RICH_STRUCTURE_CHANGED');
 const blocks=text.split('\n\n');if(blocks.length!==paragraphs.length)fail('RICH_STRUCTURE_CHANGED');
 return paragraphs.map((p,i)=>{const lines=blocks[i].split('\n');if(lines.length!==p.runs.length||lines.some(v=>!v.trim()))fail('RICH_STRUCTURE_CHANGED');return {...p,runs:p.runs.map((r,j)=>({...r,text:lines[j]}))};});
}
function paragraph(w,s,p){
 const length=w.getParagraphLength(s,p);if(!Number.isSafeInteger(length)||length<0||length>4000)fail('RICH_RANGE_UNSUPPORTED');
 return {section:s,paragraph:p,text:w.getTextRange(s,p,0,length),chars:Array.from({length},(_,i)=>w.getCharPropertiesAt(s,p,i)),para:w.getParaPropertiesAt(s,p),style:w.getStyleAt(s,p)};
}
function snapshot(w){
 const result=[];let total=0;
 for(let s=0;s<w.getSectionCount();s++)for(let p=0;p<w.getParagraphCount(s);p++){
  total+=w.getParagraphLength(s,p);if(result.length>=128||total>32000)fail('RICH_DOCUMENT_BUDGET');result.push(paragraph(w,s,p));
 }return result;
}
export function createRichReview(Controller,deps,allowed,now=()=>performance.now()){
 const {wasm:w,input}=deps;if(input.grammarNoRenderRollback!==true)fail('RHWP_SAFE_EXECUTOR_REQUIRED');
 const selection=structuredClone(input.getSelection());const {start,end}=selection??{};
 if(!start||!end||start.parentParaIndex!==undefined||end.parentParaIndex!==undefined||start.sectionIndex!==end.sectionIndex||!Number.isSafeInteger(start.sectionIndex)||start.sectionIndex<0||!Number.isSafeInteger(start.paragraphIndex)||!Number.isSafeInteger(end.paragraphIndex)||start.paragraphIndex<0||end.paragraphIndex<start.paragraphIndex||end.paragraphIndex-start.paragraphIndex>=8||start.charOffset!==0||end.charOffset!==w.getParagraphLength(end.sectionIndex,end.paragraphIndex))fail('SELECT_WHOLE_BODY_PARAGRAPHS');
 allowed();let total=0,runId=0;const paragraphs=[];
 // Refuse sensitive/unsupported fields in every target before reading any target text.
 for(let p=start.paragraphIndex;p<=end.paragraphIndex;p++){
  const n=w.getParagraphLength(start.sectionIndex,p);total+=n;if(!Number.isSafeInteger(n)||n<1||total>3000||w.getControlTextPositions(start.sectionIndex,p).length)fail('RICH_RANGE_UNSUPPORTED');
  for(let i=0;i<=n;i++)if(w.getFieldInfoAt({sectionIndex:start.sectionIndex,paragraphIndex:p,charOffset:i}).inField)fail('RICH_RANGE_UNSUPPORTED');
 }
 const before=snapshot(w);
 for(let p=start.paragraphIndex;p<=end.paragraphIndex;p++){
  const original=before.find(x=>x.section===start.sectionIndex&&x.paragraph===p),chars=Array.from(original.text);if(chars.length!==original.chars.length||/[\r\n\u0000-\u001f\u007f]/u.test(original.text))fail('RICH_RANGE_UNSUPPORTED');
  const runs=[];
  for(let i=0;i<chars.length;i++){
   const shape=original.chars[i].charShapeId;if(!Number.isSafeInteger(shape)||shape<0)fail('RICH_FORMAT_UNSUPPORTED');
   let run=runs.at(-1);if(!run||!equal(run.properties,original.chars[i])){run={id:runId++,text:'',shape,properties:original.chars[i]};runs.push(run);}run.text+=chars[i];
  }
  if(runId>24||runs.some(r=>!r.text.trim()))fail('RICH_RANGE_UNSUPPORTED');
  // Moving emphasis across language-specific word order needs a separate protocol.
  // Until qualified, admit only complete sentence-aligned style runs.
  if(runs.length>1&&runs.some(r=>!/[.!?。！？]["'’”)]?$/u.test(r.text.trim())))fail('RICH_INLINE_BOUNDARY_UNQUALIFIED');paragraphs.push({...original,runs});
 }
 const controller=new Controller(deps),state=controller.getDocumentState(),expires=now()+30000;let closed=false;
 const check=()=>{allowed();if(closed||now()>=expires)fail('STALE_REVIEW');if(!equal(input.getSelection(),selection)||!equal(controller.getDocumentState(),state))fail('DOCUMENT_OR_SELECTION_CHANGED');};
 return {source:richEnvelope(paragraphs),reviewSource:richReviewText(paragraphs),richParagraphs:paragraphs,
  valid(){try{check();return true;}catch{return false;}},close(){closed=true;controller.dispose();},
  async accept(text){
   check();const replacement=reviewedRuns(text,paragraphs);const expected=structuredClone(before);
   for(const p of replacement){const row=expected.find(x=>x.section===p.section&&x.paragraph===p.paragraph);row.text=p.runs.map(r=>r.text).join('');row.chars=p.runs.flatMap(r=>Array.from(r.text,()=>r.properties));}
   let entered=false;
   try{
    await input.executeDocumentAgentOperation({kind:'snapshot',operationType:'grammar:apply',operation(){
     check();if(!equal(snapshot(w),before))fail('DOCUMENT_CHANGED');entered=true;
     for(const p of replacement){const text=p.runs.map(r=>r.text).join('');const result=w.replaceText(p.section,p.paragraph,0,cp(p.text),text);if(!result.ok||result.newLength!==cp(text))fail('RICH_APPLY_UNCERTAIN');let offset=0;for(const r of p.runs){w.setCharShapeId(p.section,p.paragraph,offset,offset+cp(r.text),r.shape);offset+=cp(r.text);}w.setParaShapeId(p.section,p.paragraph,p.para.paraShapeId);}
     if(!equal(snapshot(w),expected))fail('RICH_APPLY_UNCERTAIN');const last=replacement.at(-1);return {sectionIndex:last.section,paragraphIndex:last.paragraph,charOffset:cp(last.runs.map(r=>r.text).join(''))};
    },meta:{actionId:'grammar:rich-apply',domain:'text',refresh:'full',dirtyScope:'paragraph',selection:'moveToResult'}},deps.render);
    if(!equal(snapshot(w),expected))fail('RICH_APPLY_UNCERTAIN');return {status:'applied'};
   }catch(e){if(entered)fail('RICH_APPLY_UNCERTAIN');throw e;}finally{closed=true;controller.dispose();}
  }
 };
}
