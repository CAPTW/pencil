import {createAcceptGate} from './accept-gate.mjs';

// Trusted in-editor host component. No global listener, storage or network.
// Admission must reject sensitive/unsupported documents before any text read.
export function createReviewSession(Controller, collectEvidence, deps, eligible, now) {
  let enabled=false, current=null, running=null, busy=false, disposed=false;
  const fail=code=>{throw Object.assign(new Error(code),{code});};
  const clear=()=>{current?.gate.close();running?.gate.close();current=null;};
  const allowed=()=>{if(disposed||!enabled||eligible()!==true)fail('NOT_ENABLED_OR_ELIGIBLE');};
  return Object.freeze({
    enable(){if(disposed)fail('CLOSED');enabled=true;},
    disable(){enabled=false;clear();},
    valid(id){
      try{allowed();if(!current||current.id!==id)return false;return current.gate.validate(current.token);}
      catch{clear();return false;}
    },
    capture(){
      if(busy)fail('BUSY');clear();allowed();
      const selection=deps.input.getSelection();
      if(!selection)fail('SELECT_WHOLE_PLAIN_PARAGRAPH');
      const {start,end}=selection;
      if(!start||!end||start.parentParaIndex!==undefined||end.parentParaIndex!==undefined||
         start.sectionIndex!==end.sectionIndex||start.paragraphIndex!==end.paragraphIndex||
         !Number.isSafeInteger(start.sectionIndex)||start.sectionIndex<0||
         !Number.isSafeInteger(start.paragraphIndex)||start.paragraphIndex<0||start.charOffset!==0)fail('SELECT_WHOLE_PLAIN_PARAGRAPH');
      const length=deps.wasm.getParagraphLength(start.sectionIndex,start.paragraphIndex);
      if(!Number.isSafeInteger(length)||length<1||length>4000||end.charOffset!==length)fail('SELECT_WHOLE_PLAIN_PARAGRAPH');
      const target={kind:'body_paragraph',section:start.sectionIndex,paragraph:start.paragraphIndex,charOffset:0,length};
      // collectEvidence refuses fields/controls/mixed formatting before target text.
      const evidence=collectEvidence(deps.wasm,target);
      allowed();
      const input=new Proxy(deps.input,{get(object,key){
        if(key==='executeDocumentAgentOperation')return (desc,render)=>object.executeDocumentAgentOperation({...desc,operation(wasm){allowed();return desc.operation(wasm);}},render);
        const value=object[key];return typeof value==='function'?value.bind(object):value;
      }});
      const gate=createAcceptGate(Controller,{...deps,input},now);
      try {
        gate.enable();const state=gate.getDocumentState();
        const token=gate.capture({schemaVersion:1,commandId:crypto.randomUUID(),expectedDocumentEpoch:state.documentEpoch,
          expectedChangeSeq:state.changeSeq,expectedDocumentSha256:state.documentSha256,target,
          expectedBeforeSha256:evidence.textSha256,expectedFormatSha256:evidence.formatSha256,
          expectedAdjacentContextSha256:evidence.adjacentContextSha256,replacement:evidence.text});
        const id=crypto.randomUUID();current={id,gate,token};
        return Object.freeze({id,source:evidence.text});
      }catch(e){gate.close();throw e;}
    },
    async accept(id,replacement){
      if(busy)fail('BUSY');
      try{allowed();}catch(e){clear();throw e;}
      const captured=current;current=null;
      if(!captured||captured.id!==id){captured?.gate.close();fail('STALE_REVIEW');}
      busy=true;running=captured;
      try{return await captured.gate.accept(captured.token,replacement);}
      finally{captured.gate.close();running=null;busy=false;}
    },
    close(){disposed=true;enabled=false;clear();}
  });
}
