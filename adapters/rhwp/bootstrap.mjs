import {createReviewSession} from './review-session.mjs';
import {mountReviewPanel} from './review-panel.mjs';

// Called by the editor's trusted initialization, not from a global/message hook.
// Default admission denies everything. No deployment is authorized by mounting.
export function installReview({host,Controller,collectEvidence,deps,eligible=()=>false,instant=null}) {
  const doc=host.ownerDocument, view=doc.defaultView;
  let generation=deps.wasm.documentGeneration, remove=null, stopped=false;
  const reset=()=>{
    remove?.();remove=null;
    if(stopped)return;
    host.dataset.grammarGeneration=String(generation);
    const bound=generation;
    const session=createReviewSession(Controller,collectEvidence,deps,
      ()=>!stopped&&doc.visibilityState==='visible'&&deps.wasm.documentGeneration===bound&&eligible()===true);
    remove=mountReviewPanel(host,session,instant);
  };
  const check=()=>{if(deps.wasm.documentGeneration!==generation){generation=deps.wasm.documentGeneration;reset();}};
  const visibility=()=>{if(doc.visibilityState!=='visible')reset();};
  const stop=()=>{
    if(stopped)return;stopped=true;clearInterval(timer);remove?.();remove=null;
    doc.removeEventListener('visibilitychange',visibility);view.removeEventListener('pagehide',stop);
  };
  reset();
  const timer=setInterval(check,100);
  doc.addEventListener('visibilitychange',visibility);view.addEventListener('pagehide',stop);
  return stop;
}
