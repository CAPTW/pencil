// Internal scoped review UI. Optional selected-text local analysis; no Provider.
export function mountReviewPanel(host, session, instant = null) {
  const doc=host.ownerDocument, root=doc.createElement('section');
  const status=doc.createElement('p');status.setAttribute('role','status');
  const draft=doc.createElement('textarea');draft.setAttribute('aria-label','Reviewed replacement');
  const button=(label,fn)=>{const b=doc.createElement('button');b.textContent=label;b.type='button';b.addEventListener('click',fn);root.append(b);return b;};
  let capture=null, revision=0, editRevision=0, analysis=null;
  draft.addEventListener('input',()=>{editRevision++;});
  const clear=()=>{revision++;analysis?.abort();analysis=null;capture=null;draft.value='';accept.disabled=true;};
  let intent=null;
  if(instant?.supportsWriting){
    let selectedMode='grammar';
    const mode=doc.createElement('select');mode.setAttribute('aria-label','Local writing mode');
    const choices=[['grammar','Grammar'],['ko','Translate to Korean'],['en','Translate to English'],['paraphrase','Natural rewrite'],['polite','Polite rewrite']];
    for(const [value,label] of choices){const option=doc.createElement('option');option.value=value;option.textContent=label;mode.append(option);}
    mode.addEventListener('change',()=>{
      if(capture&&draft.value!==capture.source&&!doc.defaultView.confirm('Discard this draft and change writing mode?')){mode.value=selectedMode;return;}
      selectedMode=mode.value;session.disable();clear();intent=mode.value==='grammar'?null:['ko','en'].includes(mode.value)?{mode:'translate',target:mode.value}:{mode:mode.value};status.textContent='Mode changed — enable review and capture again';
    });
    root.append(mode);
  }
  button('Enable review',()=>{session.enable();status.textContent='Enabled — select one whole plain paragraph';});
  button('Capture selection',async()=>{
    clear();
    try{
      capture=session.capture();draft.value=capture.source;accept.disabled=false;status.textContent='Review and edit before Accept';
      if(!instant)return;
      const bound=capture,version=revision,edit=editRevision,controller=new AbortController();analysis=controller;
      status.textContent='Local Instant running — draft remains editable';
      try{
        const result=await instant(bound,controller.signal,intent);
        if(controller.signal.aborted||revision!==version||capture?.id!==bound.id)return;
        if(!session.valid(bound.id)){accept.disabled=true;capture=null;status.textContent='Selection changed — capture again';return;}
        if(editRevision!==edit){status.textContent='Your edited draft was kept';return;}
        if(typeof result!=='string'||Array.from(result).length>4000)throw new Error('INSTANT_RESULT_REFUSED');
        draft.value=result;status.textContent='Local Instant ready — review before Accept';
      }catch{if(!controller.signal.aborted&&revision===version&&capture?.id===bound.id)status.textContent='Local Instant unavailable — current draft kept';}
      finally{if(analysis===controller)analysis=null;}
    }catch(e){status.textContent=e.code??'Capture unavailable';}
  });
  root.append(draft);
  const accept=button('Accept',async()=>{
    if(!capture)return;const id=capture.id,version=revision;capture=null;analysis?.abort();analysis=null;accept.disabled=true;
    try{await session.accept(id,draft.value);if(version===revision)status.textContent='Applied';}
    catch(e){if(version===revision)status.textContent=e.code??'Apply outcome uncertain';}
  });accept.disabled=true;
  button('Disable review',()=>{session.disable();clear();status.textContent='Disabled';});
  root.append(status);host.append(root);status.textContent='Disabled';
  return ()=>{session.close();clear();root.remove();};
}
