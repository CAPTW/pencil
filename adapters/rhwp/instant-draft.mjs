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
    const owner=new Core.DocumentSession(capture.id);
    try{
      const request=owner.update(capture.source);
      if(!request)throw new Error('INSTANT_INPUT_REFUSED');
      const response=await analyze({version:1,op:'analyze',id:capture.id,epoch:request.epoch,revision:request.revision,text:request.text,...(bound?{intent:{...bound}}:{})},signal);
      if(signal.aborted)throw new Error('INSTANT_CANCELLED');
      if(bound&&(response?.intent?.mode!==bound.mode||response?.intent?.target!==bound.target))throw new Error('INSTANT_INTENT_REPLY_REFUSED');
      if(response.id!==capture.id||response.epoch!==request.epoch||response.revision!==request.revision||response.error||!owner.publish(request,response.suggestions))throw new Error('INSTANT_REPLY_REFUSED');
      const review=Core.SelectionReview.capture(owner,0,capture.source.length);
      if(!review)throw new Error('INSTANT_REVIEW_REFUSED');
      const draft=review.draft;review.cancel();return draft;
    }finally{owner.clear();busy=false;}
  };
  Object.defineProperty(run,'supportsWriting',{value:writing});
  return run;
}
