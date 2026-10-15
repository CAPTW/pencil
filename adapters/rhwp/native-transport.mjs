// Extension-owned document only. No page-message listener, HTTP or cloud fallback.
export function createNativeTransport({runtime,location,host,allowed,timeoutMs=10000}) {
  if(!runtime?.id || !/^[a-p]{32}$/.test(runtime.id) ||
     location?.protocol!=='chrome-extension:' || location.host!==runtime.id ||
     typeof host!=='string' || !/^[a-z0-9_]+(?:\.[a-z0-9_]+)+$/.test(host) ||
     typeof allowed!=='function' || !Number.isInteger(timeoutMs) || timeoutMs<1 || timeoutMs>10000)
    throw new Error('NATIVE_CONTEXT_REFUSED');
  let active=null,closed=false;
  const permission=()=>{try{return !closed&&allowed()===true;}catch{return false;}};
  const analyze=(request,signal)=>{
    if(!permission()||signal?.aborted)return Promise.reject(new Error('NATIVE_NOT_ALLOWED'));
    if(active)return Promise.reject(new Error('NATIVE_BUSY'));
    if(request?.version!==1||request.op!=='analyze'||typeof request.text!=='string'||request.text.length>8192||
       typeof request.id!=='string'||typeof request.epoch!=='string'||!Number.isSafeInteger(request.revision))
      return Promise.reject(new Error('NATIVE_REQUEST_REFUSED'));
    // Copy exact local-only envelope; caller cannot smuggle a Deep operation/fields.
    const envelope={version:1,op:'analyze',id:request.id,epoch:request.epoch,revision:request.revision,text:request.text};
    if(request.intent){
      const {mode,target}=request.intent;
      if(['paraphrase','polite'].includes(mode)&&Object.keys(request.intent).length===1)envelope.intent={mode};
      else if(mode==='translate'&&['ko','en'].includes(target)&&Object.keys(request.intent).length===2)envelope.intent={mode,target};
      else return Promise.reject(new Error('NATIVE_INTENT_REFUSED'));
    }
    return new Promise((resolve,reject)=>{
      let port,timer,done=false;
      const finish=(error,value)=>{
        if(done)return;done=true;clearTimeout(timer);signal?.removeEventListener('abort',abort);
        if(port){port.onMessage.removeListener(message);port.onDisconnect.removeListener(disconnected);try{port.disconnect();}catch{}}
        active=null;error?reject(new Error(error)):resolve(value);
      };
      const abort=()=>finish('NATIVE_CANCELLED');
      const disconnected=()=>{void runtime.lastError;finish('NATIVE_DISCONNECTED');};
      const message=value=>{
        if(!permission())return finish('NATIVE_NOT_ALLOWED');
        if(!value||value.id!==envelope.id||value.epoch!==envelope.epoch||value.revision!==envelope.revision)
          return finish('NATIVE_RESPONSE_REFUSED');
        finish(null,value);
      };
      active=abort;
      signal?.addEventListener('abort',abort,{once:true});
      timer=setTimeout(()=>finish('NATIVE_TIMEOUT'),timeoutMs);
      try{
        if(!permission()||signal?.aborted)return abort();
        port=runtime.connectNative(host);
        port.onMessage.addListener(message);port.onDisconnect.addListener(disconnected);
        port.postMessage(envelope);
      }catch{finish('NATIVE_UNAVAILABLE');}
    });
  };
  return Object.freeze({analyze,close(){closed=true;active?.();}});
}
