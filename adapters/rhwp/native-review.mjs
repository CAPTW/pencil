import {createNativeTransport} from './native-transport.mjs';
import {createInstantDraft} from './instant-draft.mjs';
import {createDocumentAdmission} from './document-admission.mjs';
import {installReview} from './bootstrap.mjs';

// Trusted extension application initialization. No window/global API is installed.
// The application must provide verified per-load provenance through eligible.
export function installNativeReview({runtime,location,nativeHost,Core,...options}) {
  const doc=options.host.ownerDocument;
  const admission=createDocumentAdmission(options.deps.wasm,options.eligible);
  const transport=createNativeTransport({runtime,location,host:nativeHost,
    allowed:()=>doc.visibilityState==='visible'&&admission.allowed()});
  let stop;
  try{stop=installReview({...options,instant:createInstantDraft(transport.analyze,Core,{writing:true})});}
  catch(error){transport.close();admission.close();throw error;}
  return ()=>{stop();transport.close();admission.close();};
}
