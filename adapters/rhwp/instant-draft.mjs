// Reuses Chromium's validated UTF-16 cache/review assembly. Transport is injected
// by a trusted local host; this component has no Provider or network capability.
export function createInstantDraft(analyze, Core) {
  let busy=false;
  return async (capture, signal) => {
    if(signal.aborted)throw new Error('INSTANT_CANCELLED');
    if(busy)throw new Error('INSTANT_BUSY');
    busy=true;
    const owner=new Core.DocumentSession(capture.id);
    try{
      const request=owner.update(capture.source);
      if(!request)throw new Error('INSTANT_INPUT_REFUSED');
      const response=await analyze({version:1,op:'analyze',id:capture.id,epoch:request.epoch,revision:request.revision,text:request.text},signal);
      if(signal.aborted)throw new Error('INSTANT_CANCELLED');
      if(response.id!==capture.id||response.epoch!==request.epoch||response.revision!==request.revision||response.error||!owner.publish(request,response.suggestions))throw new Error('INSTANT_REPLY_REFUSED');
      const review=Core.SelectionReview.capture(owner,0,capture.source.length);
      if(!review)throw new Error('INSTANT_REVIEW_REFUSED');
      const draft=review.draft;review.cancel();return draft;
    }finally{owner.clear();busy=false;}
  };
}
