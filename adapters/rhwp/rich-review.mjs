// Bounded body selection. Stable run IDs carry styles across translated order.
// No proportional position mapping; outside selection stays byte-for-byte local.
const fail=code=>{throw Object.assign(new Error(code),{code});};
const cp=s=>Array.from(s).length;
const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const esc=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;');
const sameStyle=(a,b)=>equal(a.properties??{shape:a.shape??a.id},b.properties??{shape:b.shape??b.id});
const tag=r=>r.tag??`r${r.id}`;
const unesc=s=>s.replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&amp;','&');
export function richEnvelope(paragraphs){return paragraphs.map((p,i)=>`<p${i}>`+p.runs.map(r=>`<${tag(r)}>${esc(r.text)}</${tag(r)}>`).join('')+`</p${i}>`).join('\n');}
export function richReviewText(paragraphs){return paragraphs.map(p=>p.runs.map(r=>r.text).join('\n')).join('\n\n');}
export function parseRichReply(text,paragraphs){return parseRichDraft(text,paragraphs).text;}
export function parseRichDraft(text,paragraphs){
 if(typeof text!=='string'||cp(text)>6000)fail('RICH_STRUCTURE_CHANGED');
 let rest=text.trim();const result=[];
 for(let i=0;i<paragraphs.length;i++){
  const open=`<p${i}>`;if(!rest.startsWith(open))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(open.length);
  const runs=[];
  const remaining=new Map(paragraphs[i].runs.map(r=>[tag(r),r]));
  const seen=new Set();
  while(!rest.startsWith(`</p${i}>`)){
   if(runs.length>=96)fail('RICH_STRUCTURE_CHANGED');
   const endTag=rest.indexOf('>'),name=rest.slice(1,endTag),run=rest.startsWith('<')&&remaining.get(name);if(!run)fail('RICH_STRUCTURE_CHANGED');seen.add(name);
   const a=`<${tag(run)}>`,b=`</${tag(run)}>`;if(!rest.startsWith(a))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(a.length);
   const end=rest.indexOf(b);if(end<0)fail('RICH_STRUCTURE_CHANGED');const raw=rest.slice(0,end);
   if(/[<>\r\n\u0000-\u001f\u007f]/u.test(raw)||/&(?!amp;|lt;|gt;)/u.test(raw))fail('RICH_STRUCTURE_CHANGED');
   const value=unesc(raw);if(!value.trim())fail('RICH_STRUCTURE_CHANGED');runs.push({...run,text:value});rest=rest.slice(end+b.length);
  }
  if(!paragraphs[i].runs.every(r=>runs.some(out=>sameStyle(r,out))))fail('RICH_STRUCTURE_CHANGED');
  const close=`</p${i}>`;if(!rest.startsWith(close))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(close.length).trimStart();result.push({...paragraphs[i],runs});
 }
 if(rest||cp(richReviewText(result))>4000)fail('RICH_STRUCTURE_CHANGED');return {text:richReviewText(result),runOrder:result.map(p=>p.runs.map(r=>r.id))};
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
 if(!start||!end||start.parentParaIndex!==undefined||end.parentParaIndex!==undefined||start.sectionIndex!==end.sectionIndex||!Number.isSafeInteger(start.sectionIndex)||start.sectionIndex<0||!Number.isSafeInteger(start.paragraphIndex)||!Number.isSafeInteger(end.paragraphIndex)||start.paragraphIndex<0||end.paragraphIndex<start.paragraphIndex||end.paragraphIndex-start.paragraphIndex>=8||!Number.isSafeInteger(start.charOffset)||!Number.isSafeInteger(end.charOffset)||start.charOffset<0||end.charOffset<1||start.charOffset>=w.getParagraphLength(start.sectionIndex,start.paragraphIndex)||end.charOffset>w.getParagraphLength(end.sectionIndex,end.paragraphIndex)||start.paragraphIndex===end.paragraphIndex&&start.charOffset>=end.charOffset)fail('SELECT_BODY_TEXT_RANGE');
 allowed();let total=0,runId=0;const paragraphs=[];
 // Refuse sensitive/unsupported fields in every target before reading any target text.
 for(let p=start.paragraphIndex;p<=end.paragraphIndex;p++){
  const n=w.getParagraphLength(start.sectionIndex,p);total+=n;if(!Number.isSafeInteger(n)||n<1||total>3000||w.getControlTextPositions(start.sectionIndex,p).length)fail('RICH_RANGE_UNSUPPORTED');
  for(let i=0;i<=n;i++)if(w.getFieldInfoAt({sectionIndex:start.sectionIndex,paragraphIndex:p,charOffset:i}).inField)fail('RICH_RANGE_UNSUPPORTED');
 }
 const before=snapshot(w);
 for(let p=start.paragraphIndex;p<=end.paragraphIndex;p++){
  const original=before.find(x=>x.section===start.sectionIndex&&x.paragraph===p),chars=Array.from(original.text);if(chars.length!==original.chars.length||/[\r\n\u0000-\u001f\u007f]/u.test(original.text))fail('RICH_RANGE_UNSUPPORTED');
  const from=p===start.paragraphIndex?start.charOffset:0,to=p===end.paragraphIndex?end.charOffset:chars.length;
  const runs=[];
  for(let i=from;i<to;i++){
   const shape=original.chars[i].charShapeId;if(!Number.isSafeInteger(shape)||shape<0)fail('RICH_FORMAT_UNSUPPORTED');
   let run=runs.at(-1);if(!run||!equal(run.properties,original.chars[i])){run={id:runId++,text:'',shape,properties:original.chars[i]};runs.push(run);}run.text+=chars[i];
  }
  if(runId>24||runs.some(r=>!r.text.trim()))fail('RICH_RANGE_UNSUPPORTED');
  for(let i=0;i<runs.length;i++){const previous=runs.slice(0,i).find(r=>sameStyle(r,runs[i]));if(previous)runs[i].id=previous.id;}
  paragraphs.push({...original,from,to,runs});
 }
 const controller=new Controller(deps),state=controller.getDocumentState(),expires=now()+30000;let closed=false,reviewParagraphs=paragraphs;
 const check=()=>{allowed();if(closed||now()>=expires)fail('STALE_REVIEW');if(!equal(input.getSelection(),selection)||!equal(controller.getDocumentState(),state))fail('DOCUMENT_OR_SELECTION_CHANGED');};
 return {source:richEnvelope(paragraphs),reviewSource:richReviewText(paragraphs),richParagraphs:paragraphs,
  bindDraft(draft){check();if(!draft||!Array.isArray(draft.runOrder)||draft.runOrder.length!==paragraphs.length)fail('RICH_STRUCTURE_CHANGED');
   const ordered=paragraphs.map((p,i)=>{const ids=draft.runOrder[i];if(!Array.isArray(ids)||ids.length<1||ids.length>96)fail('RICH_STRUCTURE_CHANGED');const runs=ids.map(id=>{const r=p.runs.find(r=>r.id===id);if(!r)fail('RICH_STRUCTURE_CHANGED');return r;});if(!p.runs.every(r=>runs.some(out=>sameStyle(r,out))))fail('RICH_STRUCTURE_CHANGED');return {...p,runs};});
   reviewedRuns(draft.text,ordered);reviewParagraphs=ordered;return draft.text;},
  valid(){try{check();return true;}catch{return false;}},close(){closed=true;controller.dispose();},
  async accept(text){
   check();const replacement=reviewedRuns(text,reviewParagraphs);const expected=structuredClone(before);
   for(const p of replacement){const row=expected.find(x=>x.section===p.section&&x.paragraph===p.paragraph);const chars=Array.from(p.text);row.text=chars.slice(0,p.from).join('')+p.runs.map(r=>r.text).join('')+chars.slice(p.to).join('');row.chars=[...p.chars.slice(0,p.from),...p.runs.flatMap(r=>Array.from(r.text,()=>r.properties)),...p.chars.slice(p.to)];}
   let entered=false;
   try{
    await input.executeDocumentAgentOperation({kind:'snapshot',operationType:'grammar:apply',operation(){
     check();if(!equal(snapshot(w),before))fail('DOCUMENT_CHANGED');entered=true;
     for(const p of replacement){const row=expected.find(x=>x.section===p.section&&x.paragraph===p.paragraph);const result=w.replaceText(p.section,p.paragraph,0,cp(p.text),row.text);if(!result.ok||result.newLength!==cp(row.text))fail('RICH_APPLY_UNCERTAIN');
      let offset=0;while(offset<row.chars.length){let end=offset+1;while(end<row.chars.length&&equal(row.chars[offset],row.chars[end]))end++;w.setCharShapeId(p.section,p.paragraph,offset,end,row.chars[offset].charShapeId);offset=end;}w.setParaShapeId(p.section,p.paragraph,p.para.paraShapeId);}
     if(!equal(snapshot(w),expected))fail('RICH_APPLY_UNCERTAIN');const last=replacement.at(-1);return {sectionIndex:last.section,paragraphIndex:last.paragraph,charOffset:last.from+cp(last.runs.map(r=>r.text).join(''))};
    },meta:{actionId:'grammar:rich-apply',domain:'text',refresh:'full',dirtyScope:'paragraph',selection:'moveToResult'}},deps.render);
    if(!equal(snapshot(w),expected))fail('RICH_APPLY_UNCERTAIN');return {status:'applied'};
   }catch(e){if(entered)fail('RICH_APPLY_UNCERTAIN');throw e;}finally{closed=true;controller.dispose();}
  }
 };
}

// Alignment returns only exact non-default substrings of the immutable result.
// Ambiguous repeated text, overlap and unknown styles refuse; never fuzzy-match.
export function alignmentDefaults(paragraphs){return paragraphs.map(p=>{const counts=new Map();for(const r of p.runs)counts.set(r.id,(counts.get(r.id)??0)+cp(r.text));return [...counts].sort((a,b)=>b[1]-a[1])[0][0];});}
export function parseRichAlignment(text,paragraphs,written){
 const blocks=written.split('\n\n'),defaults=alignmentDefaults(paragraphs);if(blocks.length!==paragraphs.length||typeof text!=='string'||cp(text)>6000)fail('RICH_STRUCTURE_CHANGED');
 let rest=text.trim();const result=[];
 for(let i=0;i<paragraphs.length;i++){
  const open=`<p${i}>`,close=`</p${i}>`;if(!rest.startsWith(open))fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(open.length);const anchors=[];const seen=new Set();
  while(!rest.startsWith(close)){
   const match=/^<r(\d+)>([^<>]*)<\/r\1>/.exec(rest);if(!match||anchors.length>=96)fail('RICH_STRUCTURE_CHANGED');rest=rest.slice(match[0].length);
   const id=Number(match[1]),run=paragraphs[i].runs.find(r=>r.id===id);if(!run||id===defaults[i]||/&(?!amp;|lt;|gt;)/u.test(match[2]))fail('RICH_STRUCTURE_CHANGED');
   const value=unesc(match[2]);if(!value.trim()||/[\r\n\u0000-\u001f\u007f]/u.test(value))fail('RICH_STRUCTURE_CHANGED');const start=blocks[i].indexOf(value);if(start<0||blocks[i].indexOf(value,start+1)!==-1)fail('RICH_ALIGNMENT_CHANGED_TEXT');
   // Boundaries must not split a surrogate pair.
   for(const x of [start,start+value.length])if(x>0&&x<blocks[i].length&&/[\uD800-\uDBFF]/.test(blocks[i][x-1])&&/[\uDC00-\uDFFF]/.test(blocks[i][x]))fail('RICH_STRUCTURE_CHANGED');
   anchors.push({start,end:start+value.length,id,text:value});seen.add(id);
  }
  rest=rest.slice(close.length).trimStart();if(!paragraphs[i].runs.every(r=>r.id===defaults[i]||seen.has(r.id)))fail('RICH_STRUCTURE_CHANGED');
  anchors.sort((a,b)=>a.start-b.start);const runs=[];let offset=0;for(const a of anchors){if(a.start<offset)fail('RICH_STRUCTURE_CHANGED');if(a.start>offset)runs.push({id:defaults[i],text:blocks[i].slice(offset,a.start)});runs.push(a);offset=a.end;}if(offset<blocks[i].length)runs.push({id:defaults[i],text:blocks[i].slice(offset)});
  if(!runs.length||runs.length>96||runs.some(r=>!r.text.trim())||!runs.some(r=>r.id===defaults[i]))fail('RICH_STRUCTURE_CHANGED');result.push(runs);
 }
 if(rest)fail('RICH_STRUCTURE_CHANGED');return {text:result.map(rs=>rs.map(r=>r.text).join('\n')).join('\n\n'),runOrder:result.map(rs=>rs.map(r=>r.id))};
}
