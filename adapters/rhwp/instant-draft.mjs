import {parseRichDraft,parseRichAlignment,alignmentDefaults} from './rich-review.mjs';
// Reuses Chromium's validated UTF-16 cache/review assembly. Transport is injected
// by a trusted local host; this component has no Provider or network capability.
export function createInstantDraft(analyze, Core, {writing=false}={}) {
  let busy=false;
  const run = async (capture, signal, intent=null) => {
    if(signal.aborted)throw new Error('INSTANT_CANCELLED');
    let bound=null;
    if(intent!==null){
      if(!writing||!intent||typeof intent!=='object'||Array.isArray(intent))throw new Error('INSTANT_INTENT_REFUSED');
      if(['paraphrase','polite'].includes(intent.mode)&&Object.keys(intent).length===1)bound={mode:intent.mode};
      else if(intent.mode==='translate'&&['ko','en'].includes(intent.target)&&Object.keys(intent).length===2)bound={mode:'translate',target:intent.target};
      else throw new Error('INSTANT_INTENT_REFUSED');
    }
    if(busy)throw new Error('INSTANT_BUSY');
    busy=true;
    const scopedSignal=capture.richParagraphs?AbortSignal.any([signal,AbortSignal.timeout(8000)]):signal;
    const infer=async(text,id,requestIntent=bound)=>{
      const owner=new Core.DocumentSession(id);
      try{
        const request=owner.update(text);if(!request)throw new Error('INSTANT_INPUT_REFUSED');
        const response=await analyze({version:1,op:'analyze',id,epoch:request.epoch,revision:request.revision,text:request.text,...(requestIntent?{intent:{...requestIntent}}:{})},scopedSignal);
        if(scopedSignal.aborted)throw new Error('INSTANT_CANCELLED');
        if(requestIntent&&(response?.intent?.mode!==requestIntent.mode||response?.intent?.target!==requestIntent.target))throw new Error('INSTANT_INTENT_REPLY_REFUSED');
        if(response.id!==id||response.epoch!==request.epoch||response.revision!==request.revision||response.error||!owner.publish(request,response.suggestions))throw new Error('INSTANT_REPLY_REFUSED');
        const review=Core.SelectionReview.capture(owner,0,text.length);if(!review)throw new Error('INSTANT_REVIEW_REFUSED');const draft=review.draft;review.cancel();return draft;
      }finally{owner.clear();}
    };
    try{
      if(!capture.richParagraphs)return await infer(capture.source,capture.id);
      if(!bound)return parseRichDraft(await infer(capture.source,capture.id),capture.richParagraphs);
      // Two explicit stages: unchanged ordinary writing, then style-only alignment.
      // No retry/fallback; both stages share one cancellation/deadline and slot.
      const plain=capture.richParagraphs.map(p=>p.runs.map(r=>r.text).join('')).join('\n\n');
      const written=await infer(plain,capture.id);
      if(written.split('\n\n').length!==capture.richParagraphs.length||Array.from(written).length>4000)throw new Error('RICH_STRUCTURE_CHANGED');
      const defaults=alignmentDefaults(capture.richParagraphs);
      const queries=capture.richParagraphs.map((p,i)=>({paragraph:`p${i}`,original:p.runs.map(r=>r.text).join(''),fixed:written.split('\n\n')[i],find:[...new Set(p.runs.map(r=>r.id))].filter(id=>id!==defaults[i]).map(id=>({tag:`r${id}`,originalWords:p.runs.filter(r=>r.id===id).map(r=>r.text)}))}));
      const envelope='<pencil-align>\n<original>'+JSON.stringify(queries)+'</original>\n</pencil-align>';
      const draft=parseRichAlignment(await infer(envelope,capture.id+'-format',{mode:'style_align'}),capture.richParagraphs,written);
      const joined=draft.text.split('\n\n').map(p=>p.split('\n').join('')).join('\n\n');
      if(joined!==written)throw new Error('RICH_ALIGNMENT_CHANGED_TEXT');
      return draft;
    }finally{busy=false;}

  };
  Object.defineProperty(run,'supportsWriting',{value:writing});
  return run;
}
