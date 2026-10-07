// Internal scoped review UI. Does not request inference or transmit document text.
export function mountReviewPanel(host, session) {
  const doc=host.ownerDocument, root=doc.createElement('section');
  const status=doc.createElement('p');status.setAttribute('role','status');
  const draft=doc.createElement('textarea');draft.setAttribute('aria-label','Reviewed replacement');
  const button=(label,fn)=>{const b=doc.createElement('button');b.textContent=label;b.type='button';b.addEventListener('click',fn);root.append(b);return b;};
  let capture=null, revision=0;
  const clear=()=>{revision++;capture=null;draft.value='';accept.disabled=true;};
  button('Enable review',()=>{session.enable();status.textContent='Enabled — select one whole plain paragraph';});
  button('Capture selection',()=>{clear();try{capture=session.capture();draft.value=capture.source;accept.disabled=false;status.textContent='Review and edit before Accept';}catch(e){status.textContent=e.code??'Capture unavailable';}});
  root.append(draft);
  const accept=button('Accept',async()=>{
    if(!capture)return;const id=capture.id,version=revision;capture=null;accept.disabled=true;
    try{await session.accept(id,draft.value);if(version===revision)status.textContent='Applied';}
    catch(e){if(version===revision)status.textContent=e.code??'Apply outcome uncertain';}
  });accept.disabled=true;
  button('Disable review',()=>{session.disable();clear();status.textContent='Disabled';});
  root.append(status);host.append(root);status.textContent='Disabled';
  return ()=>{session.close();clear();root.remove();};
}
