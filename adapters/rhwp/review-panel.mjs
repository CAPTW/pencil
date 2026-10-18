// Internal scoped review UI. Optional selected-text local analysis; no Provider.
export function mountReviewPanel(host, session, instant = null) {
  const doc=host.ownerDocument, root=doc.createElement('section');
  const status=doc.createElement('p');status.setAttribute('role','status');
  const hint=doc.createElement('p');hint.textContent='For mixed formatting, each review line keeps its bound style, including reordered translated spans; blank lines separate paragraphs. Keep these boundaries.';root.append(hint);
  const draft=doc.createElement('textarea');draft.setAttribute('aria-label','Reviewed replacement');
  const button=(label,fn)=>{const b=doc.createElement('button');b.textContent=label;b.type='button';b.addEventListener('click',fn);root.append(b);return b;};
  const preview=doc.createElement('div');preview.setAttribute('aria-label','Formatted replacement preview');root.append(preview);
  let order=null;
  const showPreview=()=>{preview.textContent='';if(!capture?.richParagraphs)return;
    const blocks=draft.value.split('\n\n');capture.richParagraphs.forEach((p,i)=>{const row=doc.createElement('p');const lines=(blocks[i]??'').split('\n');(order?.[i]??p.runs.map(r=>r.id)).forEach((id,j)=>{const r=p.runs.find(r=>r.id===id),span=doc.createElement('span');span.setAttribute('data-run-id',String(id));span.setAttribute('data-paragraph',String(i));span.setAttribute('title','Original styled text: '+r.text);span.textContent=lines[j]??'';if(span.style){span.style.whiteSpace='pre-wrap';span.style.fontWeight=r.properties?.bold?'bold':'normal';span.style.fontStyle=r.properties?.italic?'italic':'normal';}row.append(span);});preview.append(row);});
  };
  let capture=null, revision=0, editRevision=0, analysis=null;
  draft.addEventListener('input',()=>{editRevision++;showPreview();});
  const clear=()=>{revision++;analysis?.abort();analysis=null;capture=null;order=null;draft.value='';preview.textContent='';accept.disabled=true;};
  let intent=null;
  if(instant?.supportsWriting){
    let selectedMode='grammar';
    const mode=doc.createElement('select');mode.setAttribute('aria-label','Local writing mode');
    const choices=[['grammar','Grammar'],['ko','Translate to Korean'],['en','Translate to English'],['paraphrase','Natural rewrite'],['polite','Polite rewrite']];
    for(const [value,label] of choices){const option=doc.createElement('option');option.value=value;option.textContent=label;mode.append(option);}
    mode.addEventListener('change',()=>{
      if(capture&&draft.value!==(capture.reviewSource??capture.source)&&!doc.defaultView.confirm('Discard this draft and change writing mode?')){mode.value=selectedMode;return;}
      selectedMode=mode.value;session.disable();clear();intent=mode.value==='grammar'?null:['ko','en'].includes(mode.value)?{mode:'translate',target:mode.value}:{mode:mode.value};status.textContent='Mode changed — enable review and capture again';
    });
    root.append(mode);
  }
  button('Enable review',()=>{session.enable();status.textContent='Enabled — select body text';});
  button('Capture selection',async()=>{
    clear();
    try{
      capture=session.capture();draft.value=capture.reviewSource??capture.source;showPreview();accept.disabled=false;status.textContent='Review and edit before Accept';
      if(!instant)return;
      const bound=capture,version=revision,edit=editRevision,controller=new AbortController();analysis=controller;
      status.textContent='Local Instant running — draft remains editable';
      try{
        let result=await instant(bound,controller.signal,intent);
        if(controller.signal.aborted||revision!==version||capture?.id!==bound.id)return;
        if(!session.valid(bound.id)){accept.disabled=true;capture=null;status.textContent='Selection changed — capture again';return;}
        if(editRevision!==edit){status.textContent='Your edited draft was kept';return;}
        if(bound.richParagraphs){const value=result;result=session.bindRichDraft(bound.id,value);order=value.runOrder;}
        if(typeof result!=='string'||Array.from(result).length>4000)throw new Error('INSTANT_RESULT_REFUSED');
        draft.value=result;showPreview();status.textContent='Local Instant ready — review before Accept';
      }catch(e){if(!controller.signal.aborted&&revision===version&&capture?.id===bound.id){const code=['RICH_ALIGNMENT_CHANGED_TEXT','RICH_STRUCTURE_CHANGED','INSTANT_REPLY_REFUSED','INSTANT_CANCELLED','NATIVE_DISCONNECTED','NATIVE_TIMEOUT'].includes(e?.message)?e.message:'INSTANT_UNAVAILABLE';root.setAttribute('data-local-result-code',code);status.textContent=code==='RICH_ALIGNMENT_CHANGED_TEXT'?'Local formatting changed the suggested text — draft kept':code==='RICH_STRUCTURE_CHANGED'?'Local formatting could not be matched — draft kept':'Local Instant unavailable — current draft kept';}}
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
