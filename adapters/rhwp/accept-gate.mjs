// Target-process component only. Not wired to the installed Grammar product.
// deps and enable/disable are trusted host capabilities, never page messages.
export function createAcceptGate(Controller, deps, now = () => performance.now()) {
  if(deps.input.grammarNoRenderRollback!==true)throw new Error('RHWP_SAFE_EXECUTOR_REQUIRED');
  let enabled = false, pending = null, active = null, closed = false, uncertain = false;
  const fail = code => { throw Object.assign(new Error(code), {code}); };
  const equalTarget = (a,b) => a && b && ['kind','section','paragraph','charOffset','length'].every(k=>a[k]===b[k]);
  const check = grant => {
    if (!enabled || closed || uncertain) fail('AUTHORITY_REVOKED');
    if (!grant || now() >= grant.expires) fail('GRANT_EXPIRED');
    const c=grant.command, state=controller.getDocumentState(), selection=controller.getSelectionContext();
    if (state.documentEpoch!==c.expectedDocumentEpoch || state.changeSeq!==c.expectedChangeSeq || state.documentSha256!==c.expectedDocumentSha256) fail('DOCUMENT_CHANGED');
    if (!selection.editable || selection.collapsed || !equalTarget(selection.target,c.target) || selection.selectedTextSha256!==c.expectedBeforeSha256) fail('SELECTION_CHANGED');
  };
  const input = {
    getCursorPosition:()=>deps.input.getCursorPosition(),
    getSelection:()=>deps.input.getSelection(),
    focusBodyParagraph:(...args)=>deps.input.focusBodyParagraph(...args),
    executeDocumentAgentOperation(desc,render) {
      const grant=active;
      check(grant); // Refuse before creating a history snapshot.
      return deps.input.executeDocumentAgentOperation({...desc,operationType:'grammar:apply',operation(wasm){
        check(grant); // Synchronous target-side fence; no await before mutation.
        grant.entered=true;
        return desc.operation(wasm);
      }},render);
    }
  };
  let controller=new Controller({...deps,input});
  return Object.freeze({
    enable(){if(closed || uncertain)fail('CLOSED');enabled=true;},
    disable(){enabled=false;pending=null;},
    capture(command){
      if(active)fail('BUSY');
      pending=null;
      const grant={command:structuredClone(command),expires:now()+30000,entered:false};
      check(grant);
      const token=crypto.randomUUID();pending={token,grant};return token;
    },
    async accept(token){
      if(active)fail('BUSY');
      const candidate=pending;pending=null;
      if(!candidate || candidate.token!==token)fail('INVALID_TOKEN');
      active=candidate.grant;
      try {check(active);return await controller.applyTextCommand(active.command);}
      catch(e){if(active.entered)uncertain=true;throw e;}
      finally {
        active=null;closed=true;enabled=false;
        controller?.dispose();controller=null;
      }
    },
    getDocumentState:()=>{if(closed)fail('CLOSED');return controller.getDocumentState();},
    close(){enabled=false;pending=null;closed=true;if(!active){controller?.dispose();controller=null;}}
  });
}
