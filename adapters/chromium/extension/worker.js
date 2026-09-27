// Permissions are deliberately memory-only. Worker restart requires a new gesture.
const sessions = new Map();
const generations = new Map();
const retiring = new Set();
let nativeUnsafe = false;
const providers = new Set(['codex', 'antigravity', 'claude']);
// Content-free cleanup ownership survives worker restart. Legacy string markers
// remain blocked: they have no native receipt and cannot be inferred complete.
let deepMarkers = new Map();
let markerStorageInvalid = false;
let recovering = false;
const validToken = t => typeof t==='string' && /^[a-f0-9]{8}(-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(t);
const validTicket = t => t && typeof t==='object' && !Array.isArray(t) &&
  Object.keys(t).length===4 && ['installation','slot','generation','token'].every(key=>Object.hasOwn(t,key)) &&
  typeof t.installation==='string' && /^[a-f0-9]{32}$/.test(t.installation) &&
  Number.isInteger(t.slot) && t.slot >= 0 && t.slot < 4 &&
  Number.isSafeInteger(t.generation) && t.generation > 0 &&
  validToken(t.token);
const sameTicket = (a,b) => validTicket(a) && validTicket(b) &&
  a.installation===b.installation && a.slot===b.slot && a.generation===b.generation && a.token===b.token;
const cleanupReady = chrome.storage.local.get('deepCleanupPending').then(({deepCleanupPending=[]})=>{
  if(!Array.isArray(deepCleanupPending) || deepCleanupPending.length>4) {markerStorageInvalid=true;nativeUnsafe=true;return;}
  for(const entry of deepCleanupPending) {
    const key=typeof entry==='string' ? entry : entry?.ticket?.token || entry?.token;
    if(typeof key!=='string' || !key || key.length>128 || deepMarkers.has(key) ||
      (typeof entry!=='string' && !(entry && !Array.isArray(entry) && Object.keys(entry).length===2 && (entry.phase==='reserving' ? validToken(entry.token) : validTicket(entry.ticket) && ['pending','ack'].includes(entry.phase)))))markerStorageInvalid=true;
    else deepMarkers.set(key,entry);
  }
  if(markerStorageInvalid || deepMarkers.size)nativeUnsafe=true;
}).catch(()=>{nativeUnsafe=true;markerStorageInvalid=true;});
let markerQueue=Promise.resolve();
function markDeep(marker,value) {
  const next=markerQueue.then(async()=>{
    await cleanupReady;
    if(markerStorageInvalid)throw Error('cleanup_storage_invalid');
    const updated=new Map(deepMarkers);value ? updated.set(marker,value) : updated.delete(marker);
    if(updated.size>4)throw Error('cleanup_capacity');
    await chrome.storage.local.set({deepCleanupPending:[...updated.values()]});deepMarkers=updated;
  });
  markerQueue=next.catch(()=>{nativeUnsafe=true;markerStorageInvalid=true;});return next;
}
// Control messages never contain document identity/text and cannot invoke inference.
function cleanupControl(op, fields) {
  if(!HOST)return Promise.resolve({error:'native_host_not_configured'});
  const id=crypto.randomUUID(),epoch='cleanup-control';
  return new Promise(resolve=>{
    let port,done=false,timer;
    const finish=value=>{if(done)return;done=true;clearTimeout(timer);port?.disconnect();resolve(value);};
    try {
      port=chrome.runtime.connectNative(HOST);
      port.onMessage.addListener(value=>finish(value?.id===id && value.epoch===epoch && value.revision===1 && (!fields.cleanup_ticket || sameTicket(value.cleanup_ticket,fields.cleanup_ticket)) ? value : {error:'invalid_cleanup_response'}));
      port.onDisconnect.addListener(()=>{void chrome.runtime.lastError;finish({error:'cleanup_unavailable'});});
      timer=setTimeout(()=>finish({error:'cleanup_timeout'}),5000);
      port.postMessage({version:1,op,id,epoch,revision:1,text:'',...fields});
    }catch {finish({error:'cleanup_unavailable'});}
  });
}
// Reserve+persist and acknowledge+erase share one queue: an acknowledged slot
// cannot be reused while its old browser marker is still durable.
let lifecycleQueue=Promise.resolve();
function lifecycle(action) {const next=lifecycleQueue.then(action);lifecycleQueue=next.catch(()=>{});return next;}
const releasingTickets=new Map();
function releaseTicket(ticket,alreadyComplete=false) {
  if(releasingTickets.has(ticket.token))return releasingTickets.get(ticket.token);
  const promise=releaseTicketOwned(ticket,alreadyComplete).finally(()=>releasingTickets.delete(ticket.token));
  releasingTickets.set(ticket.token,promise);return promise;
}
async function releaseTicketOwned(ticket, alreadyComplete=false) {
  const record=deepMarkers.get(ticket.token);
  if(!record || !sameTicket(record.ticket,ticket))return false;
  if(record.phase!=='ack') {
    const checked=alreadyComplete ? {cleanup_complete:true} : await cleanupControl('cleanup-query',{cleanup_ticket:ticket});
    if(checked.error || checked.cleanup_complete!==true)return false;
    await markDeep(ticket.token,{ticket,phase:'ack'});
  }
  return lifecycle(async()=>{
    const ack=await cleanupControl('cleanup-ack',{cleanup_ticket:ticket});
    if(ack.error || ack.cleanup_complete!==true)return false;
    await markDeep(ticket.token,null);return true;
  });
}
async function recoverCleanup() {
  await cleanupReady;
  if(recovering || [...sessions.values(),...retiring].some(s=>s.admitting))return {status:'Cleanup check busy; try again'};
  if(markerStorageInvalid)return {status:'Cleanup records unavailable; Deep remains blocked'};
  recovering=true;
  try {
    for(let record of [...deepMarkers.values()]) {
      if(typeof record==='string')continue;
      if(record.phase==='reserving') {
        const found=await cleanupControl('cleanup-find',{cleanup_token:record.token});
        if(found.error || !validTicket(found.cleanup_ticket) || found.cleanup_ticket.token!==record.token)continue;
        record={ticket:found.cleanup_ticket,phase:'pending'};
        await markDeep(record.ticket.token,record);
      }
      if(!await releaseTicket(record.ticket))continue;
      for(const s of new Set([...sessions.values(),...retiring])) {
        if(s.pending?.marker!==record.ticket.token && s.deepMarker!==record.ticket.token)continue;
        clearTimeout(s.timer);s.pending?.resolve({error:'cancelled'});s.pending=null;
        const port=s.port;s.port=null;port?.disconnect();retiring.delete(s);
      }
    }
    if(!deepMarkers.size && !markerStorageInvalid) {
      nativeUnsafe=false;
      // As the status says: every Deep grant from before the failure is gone,
      // so sending again needs a fresh, explicit grant.
      for(const [tabId,s] of sessions) {
        if(!s.deepProvider)continue;
        s.grantGeneration++;s.deepProvider=null;
        chrome.tabs.sendMessage(tabId,{op:'deep-policy',epoch:s.epoch,provider:null},{documentId:s.documentId}).catch(()=>{});
      }
    }
    return {status:deepMarkers.size ? 'Cleanup unconfirmed; Deep remains blocked. Local Instant is available.' : 'Cleanup confirmed. Grant Deep permission again to send.'};
  }catch {nativeUnsafe=true;return {status:'Cleanup check failed; Deep remains blocked'};}
  finally {recovering=false;}
}
let emergencyGeneration = 0;
let policyChanging = false;
import {HOST} from './host-config.js';
const blockedDefault = /(^|\.)(accounts\.google\.com|login\.microsoftonline\.com)$/i;
const originOf = url => { try { const u = new URL(url); return /^https?:$/.test(u.protocol) ? u.origin : null; } catch { return null; } };
async function denied(origin) {
  const { deniedOrigins = [] } = await chrome.storage.local.get('deniedOrigins');
  return !origin || blockedDefault.test(new URL(origin).hostname) || deniedOrigins.includes(origin);
}
function close(tabId) {
  generations.set(tabId, (generations.get(tabId) || 0) + 1);
  const s = sessions.get(tabId); sessions.delete(tabId);
  if (s) {
    s.deepProvider = null; s.closed = true;
    if(s.admitting) {s.admitting.cancelled=true;retiring.add(s);}
    if (s.pending?.op === 'deep') { retiring.add(s); cancelDeep(s); s.pending.resolve({error:'disabled'}); }
    else { clearTimeout(s.timer); s.pending?.resolve({error:'disabled'}); s.pending = null; s.port?.disconnect(); }
  }
  if(s) chrome.tabs.sendMessage(tabId, {op: 'disable', epoch:s.epoch}, {documentId:s.documentId}).catch(() => {});
}
async function enable(tabId) {
  if (policyChanging) return {status: 'Domain policy changing; enable again after completion'};
  close(tabId);
  const generation = generations.get(tabId), emergency = emergencyGeneration;
  const current = () => !policyChanging && generations.get(tabId) === generation && emergencyGeneration === emergency;
  const tab = await chrome.tabs.get(tabId), origin = originOf(tab.url);
  if (await denied(origin)) return {status: 'Domain blocked or unsupported'};
  if (!current()) return {status: 'Enable cancelled'};
  if (sessions.size + retiring.size >= 4 && !sessions.has(tabId)) return {status: 'Disable another document first (limit 4)'};
  const injected = await chrome.scripting.executeScript({target: {tabId}, files: ['core.js', 'content.js']});
  if (!current()) return {status: 'Enable cancelled'};
  const documentId = injected.find(frame => frame.frameId === 0)?.documentId;
  if (!documentId) return {status: 'Document unavailable'};
  if (sessions.size + retiring.size >= 4) return {status: 'Disable another document first (limit 4)'};
  const epoch = crypto.randomUUID();
  sessions.set(tabId, {origin, epoch, documentId, port: null, pending: null, timer: null, deepProvider: null, grantGeneration: 0, closed: false});
  let response;
  try { response = await chrome.tabs.sendMessage(tabId, {op: 'enable', epoch}, {documentId}); }
  catch {
    // A failed old delivery must not revoke a newer grant on the same tab.
    if(current() && sessions.get(tabId)?.epoch===epoch) close(tabId);
    return {status:'Document unavailable'};
  }
  if (!current()) return {status: 'Enable cancelled'};
  if (!response?.ready) { close(tabId); return {status: 'Document unavailable'}; }
  return {status: 'Document enabled. Select a field, then Enable this field.'};
}
function cancelDeep(s) {
  const p = s.pending;
  if (!p || p.op !== 'deep' || p.cancelling) return;
  p.cancelling = true; clearTimeout(s.timer);
  try { s.port.postMessage({version:1,op:'cancel',id:p.id,epoch:s.epoch,revision:p.revision,text:''}); }
  catch { nativeUnsafe = true; }
  s.timer = setTimeout(() => {
    // Missing cleanup receipt blocks new Deep admission. Keep ownership, never claim cleanup.
    nativeUnsafe = true; p.resolve({error:'cleanup_unconfirmed'});
  }, 5000);
}
function analyze(s, request) {
  if (!HOST) return Promise.resolve({error: 'native_host_not_configured'});
  if (s.pending) return Promise.resolve({error: 'busy'});
  return new Promise(resolve => {
    s.pending = {id:request.id,op:request.op,revision:request.revision,provider:request.provider,marker:s.deepMarker,ticket:request.cleanup_ticket,resolve};
    const finish = async (value, id = request.id) => {
      if(s.pending?.id !== id) return;
      clearTimeout(s.timer); const pending = s.pending;
      if (pending.op === 'deep' && value?.cleanup_complete!==true) {
        nativeUnsafe = true; cancelDeep(s); pending.resolve({error:'cleanup_unconfirmed'}); return;
      }
      if(pending.op==='deep') {try {if(!await releaseTicket(pending.ticket,true))throw Error('cleanup_ack_missing');}catch {nativeUnsafe=true;pending.resolve({error:'cleanup_unconfirmed'});return;}}
      if(s.pending!==pending)return;
      clearTimeout(s.timer);s.pending = null; pending.resolve(pending.cancelling ? {error:'cancelled'} : value);
      if (s.closed) { retiring.delete(s); const port=s.port; s.port=null; port?.disconnect(); }
    };
    try {
      if (!s.port) {
        const port = chrome.runtime.connectNative(HOST); s.port = port;
        port.onMessage.addListener(message => {
          if (s.port !== port) return;
          if (message?.id === s.pending?.id && message.epoch === s.epoch && (s.pending.op!=='deep' || (message.revision===s.pending.revision && (message.error || message.provider===s.pending.provider)))) finish(message, message.id);
          else if(s.pending?.op==='deep') {nativeUnsafe=true;cancelDeep(s);s.pending.resolve({error:'cleanup_unconfirmed'});} else { finish({error:'invalid_native_response'},s.pending?.id);s.port=null;port.disconnect(); }
        });
        port.onDisconnect.addListener(() => { void chrome.runtime.lastError; if(s.port !== port) return; if(s.pending?.op==='deep'){nativeUnsafe=true;retiring.add(s);}else if(!s.admitting)retiring.delete(s); s.port = null; finish({error: 'native_unavailable'}, s.pending?.id); });
      }
      s.timer = setTimeout(() => { if(request.op==='deep') { cancelDeep(s); return; } finish({error: 'native_timeout'}); s.port?.disconnect(); s.port = null; }, request.op==='deep' ? 120000 : 5000);
      s.port.postMessage(request);
    } catch { finish({error: 'native_unavailable'}); }
  });
}
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  (async () => {
    // Only the extension popup may grant permission. No external messaging listener exists.
    if (sender.id === chrome.runtime.id && sender.url === chrome.runtime.getURL('popup.html')) {
      const {op, tabId} = message;
      if(op==='cleanup-recover')return recoverCleanup();
      if (op === 'stop') { emergencyGeneration++; for (const id of [...sessions.keys()]) close(id); return {status: 'All documents disabled'}; }
      if (!Number.isInteger(tabId)) return {status: 'Invalid tab'};
      if (op === 'enable') return enable(tabId);
      if (op === 'disable') { close(tabId); return {status: 'Disabled'}; }
      if (op === 'deep-consent' || op === 'deep-revoke') {
        await cleanupReady;
        const s=sessions.get(tabId);
        if(!s || s.closed || policyChanging) return {status:'Enable the current document first'};
        if(op==='deep-consent' && (message.consent!==true || !providers.has(message.provider) || nativeUnsafe || recovering)) return {status:'Deep consent or runtime unavailable'};
        cancelDeep(s); s.grantGeneration++; if(s.admitting)s.admitting.cancelled=true; s.deepProvider = op==='deep-consent' ? message.provider : null;
        await chrome.tabs.sendMessage(tabId,{op:'deep-policy',epoch:s.epoch,provider:s.deepProvider},{documentId:s.documentId});
        return {status:s.deepProvider ? 'Deep allowed for this document; each send still requires a click' : 'Deep permission revoked'};
      }
      if (op === 'deny' || op === 'allow') {
        if (policyChanging) return {status: 'Domain policy changing; try again after completion'};
        policyChanging = true;
        emergencyGeneration++;
        // Policy changes revoke every active and pending grant before any async work.
        for (const id of new Set([...sessions.keys(), tabId])) close(id);
        try {
          const origin = originOf((await chrome.tabs.get(tabId)).url);
          if (!origin) return {status: 'Unsupported domain'};
          const {deniedOrigins = []} = await chrome.storage.local.get('deniedOrigins');
          const next = new Set(deniedOrigins); op === 'deny' ? next.add(origin) : next.delete(origin);
          if (next.size > 256) return {status: 'Domain block limit reached'};
          await chrome.storage.local.set({deniedOrigins: [...next]});
          return {status: op === 'deny' ? 'Domain blocked; all documents disabled' : 'Block removed; enable explicitly'};
        } finally { policyChanging = false; }
      }
      return {status: 'Unsupported action'};
    }
    const s = sessions.get(sender.tab?.id);
    if (sender.id !== chrome.runtime.id || sender.frameId !== 0 || !s || typeof sender.documentId !== 'string' || !sender.documentId ||
        originOf(sender.url) !== s.origin || message.epoch !== s.epoch) return {error: 'permission_denied'};
    if (s.documentId && sender.documentId !== s.documentId) { close(sender.tab.id); return {error: 'document_changed'}; }
    s.documentId = sender.documentId;
    // Revocation must be synchronous even while an earlier send awaits storage.
    // One generation per document is bounded; no cancelled-ID history is retained.
    if (message.op === 'cancel-deep') {
      s.cancelGeneration=(s.cancelGeneration || 0)+1;
      if(s.admitting?.id===message.id)s.admitting.cancelled=true;
      if(s.pending?.id===message.id)cancelDeep(s);return {ok:true};
    }
    const cancellation=s.cancelGeneration || 0, grant=s.grantGeneration;
    if(await denied(s.origin)) return {error:'permission_denied'};
    if (sessions.get(sender.tab?.id) !== s) return {error: 'permission_revoked'};
    if (message.op === 'disable') { close(sender.tab.id); return {ok: true}; }
    if (message.op === 'heartbeat') return {ok: true, deepProvider:s.deepProvider};
    if (!['analyze','deep'].includes(message.op) || message.version !== 1 || typeof message.text !== 'string' || message.text.length > 8192 ||
        typeof message.id !== 'string' || !message.id || message.id.length > 128 || !Number.isSafeInteger(message.revision) || message.revision < 1) return {error: 'invalid_request'};
    const request={version:1,op:message.op,id:message.id,epoch:s.epoch,revision:message.revision,text:message.text};
    if(s.admitting)return {error:'busy'};
    if(message.op==='deep') {
      if(!message.text.trim())return {error:'invalid_request'};
      if(!HOST)return {error:'native_host_not_configured'};
      await cleanupReady;
      if(s.closed || sessions.get(sender.tab.id)!==s || cancellation!==(s.cancelGeneration || 0) || grant!==s.grantGeneration || !s.deepProvider || message.provider!==s.deepProvider || nativeUnsafe || recovering)return {error:'deep_permission_denied'};
      if(s.pending || s.admitting)return {error:'busy'};
      const provider=s.deepProvider,marker=crypto.randomUUID();
      const admission={id:request.id,cancelled:false,generation:s.grantGeneration};s.admitting=admission;
      try {
        s.deepMarker=marker;
        const ticket=await lifecycle(async()=>{
          if(nativeUnsafe || [...deepMarkers.values()].some(x=>x?.phase==='ack'))throw Error('cleanup_unconfirmed');
          // Persist intent before native reservation, so a lost reply is discoverable.
          await markDeep(marker,{token:marker,phase:'reserving'});
          if(s.closed || admission.cancelled || admission.generation!==s.grantGeneration || s.deepProvider!==provider || nativeUnsafe || recovering) {
            await markDeep(marker,null);return null;
          }
          const reserved=await cleanupControl('cleanup-reserve',{cleanup_token:marker});
          if(reserved.error==='cleanup_busy') {
            // The host refused before recording anything: drop the intent and
            // let the user send again, instead of blocking Deep until recovery.
            await markDeep(marker,null);return 'busy';
          }
          if(reserved.error || !validTicket(reserved.cleanup_ticket) || reserved.cleanup_ticket.token!==marker)throw Error('cleanup_reservation_unavailable');
          const ticket=reserved.cleanup_ticket;
          await markDeep(marker,{ticket,phase:'pending'});return ticket;
        });
        if(ticket==='busy')return {error:'busy'};
        if(!ticket) {retiring.delete(s);return {error:'permission_revoked'};}

        if(s.closed || admission.cancelled || admission.generation!==s.grantGeneration || s.deepProvider!==provider || nativeUnsafe || recovering) {
          if(await releaseTicket(ticket))retiring.delete(s);else nativeUnsafe=true;
          return {error:'permission_revoked'};
        }
        s.deepMarker=marker;
        return analyze(s,{...request,provider,consent:true,cleanup_ticket:ticket});
      } catch {nativeUnsafe=true;if(!markerStorageInvalid && !deepMarkers.has(marker))retiring.delete(s);return {error:'cleanup_unconfirmed'};}

      finally {if(s.admitting===admission)s.admitting=null;}
    }
    return analyze(s,request);
  })().then(respond).catch(() => respond({error: 'unavailable'}));
  return true;
});
chrome.tabs.onRemoved.addListener(id => {close(id); generations.delete(id);});
chrome.tabs.onUpdated.addListener((id, change) => { if (generations.has(id) && (change.status === 'loading' || change.url)) close(id); });
