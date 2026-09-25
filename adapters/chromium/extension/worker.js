// Permissions are deliberately memory-only. Worker restart requires a new gesture.
const sessions = new Map();
const generations = new Map();
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
  if (s) { clearTimeout(s.timer); s.pending?.resolve({error: 'disabled'}); s.pending = null; s.port?.disconnect(); }
  chrome.tabs.sendMessage(tabId, {op: 'disable'}).catch(() => {});
}
async function enable(tabId) {
  if (policyChanging) return {status: 'Domain policy changing; enable again after completion'};
  close(tabId);
  const generation = generations.get(tabId), emergency = emergencyGeneration;
  const current = () => !policyChanging && generations.get(tabId) === generation && emergencyGeneration === emergency;
  const tab = await chrome.tabs.get(tabId), origin = originOf(tab.url);
  if (await denied(origin)) return {status: 'Domain blocked or unsupported'};
  if (!current()) return {status: 'Enable cancelled'};
  if (sessions.size >= 4 && !sessions.has(tabId)) return {status: 'Disable another document first (limit 4)'};
  const injected = await chrome.scripting.executeScript({target: {tabId}, files: ['core.js', 'content.js']});
  if (!current()) return {status: 'Enable cancelled'};
  const documentId = injected.find(frame => frame.frameId === 0)?.documentId;
  if (!documentId) return {status: 'Document unavailable'};
  if (sessions.size >= 4) return {status: 'Disable another document first (limit 4)'};
  const epoch = crypto.randomUUID();
  sessions.set(tabId, {origin, epoch, documentId, port: null, pending: null, timer: null});
  const response = await chrome.tabs.sendMessage(tabId, {op: 'enable', epoch}, {documentId});
  if (!current()) return {status: 'Enable cancelled'};
  if (!response?.ready) { close(tabId); return {status: 'Document unavailable'}; }
  return {status: 'Document enabled. Select a field, then Enable this field.'};
}
function analyze(s, request) {
  if (!HOST) return Promise.resolve({error: 'native_host_not_configured'});
  if (s.pending) return Promise.resolve({error: 'busy'});
  return new Promise(resolve => {
    s.pending = {id: request.id, resolve};
    const finish = (value, id = request.id) => { if(s.pending?.id !== id) return; clearTimeout(s.timer); const pending = s.pending; s.pending = null; pending?.resolve(value); };
    try {
      if (!s.port) {
        const port = chrome.runtime.connectNative(HOST); s.port = port;
        port.onMessage.addListener(message => {
          if (s.port !== port) return;
          if (message?.id === s.pending?.id && message.epoch === s.epoch) finish(message, message.id);
          else { finish({error: 'invalid_native_response'}, s.pending?.id); s.port = null; port.disconnect(); }
        });
        port.onDisconnect.addListener(() => { void chrome.runtime.lastError; if(s.port !== port) return; s.port = null; finish({error: 'native_unavailable'}, s.pending?.id); });
      }
      s.timer = setTimeout(() => { finish({error: 'native_timeout'}); s.port?.disconnect(); s.port = null; }, 5000);
      s.port.postMessage(request);
    } catch { finish({error: 'native_unavailable'}); }
  });
}
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  (async () => {
    // Only the extension popup may grant permission. No external messaging listener exists.
    if (sender.id === chrome.runtime.id && sender.url === chrome.runtime.getURL('popup.html')) {
      const {op, tabId} = message;
      if (op === 'stop') { emergencyGeneration++; for (const id of [...sessions.keys()]) close(id); return {status: 'All documents disabled'}; }
      if (!Number.isInteger(tabId)) return {status: 'Invalid tab'};
      if (op === 'enable') return enable(tabId);
      if (op === 'disable') { close(tabId); return {status: 'Disabled'}; }
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
        originOf(sender.url) !== s.origin || message.epoch !== s.epoch || await denied(s.origin)) return {error: 'permission_denied'};
    if (sessions.get(sender.tab?.id) !== s) return {error: 'permission_revoked'};
    if (s.documentId && sender.documentId !== s.documentId) { close(sender.tab.id); return {error: 'document_changed'}; }
    s.documentId = sender.documentId;
    if (message.op === 'disable') { close(sender.tab.id); return {ok: true}; }
    if (message.op === 'heartbeat') return {ok: true};
    if (message.op !== 'analyze' || message.version !== 1 || typeof message.text !== 'string' || message.text.length > 8192 ||
        typeof message.id !== 'string' || message.id.length > 128 || !Number.isSafeInteger(message.revision) || message.revision < 1) return {error: 'invalid_request'};
    return analyze(s, {version: 1, op: 'analyze', id: message.id, epoch: s.epoch, revision: message.revision, text: message.text});
  })().then(respond).catch(() => respond({error: 'unavailable'}));
  return true;
});
chrome.tabs.onRemoved.addListener(id => {close(id); generations.delete(id);});
chrome.tabs.onUpdated.addListener((id, change) => { if (generations.has(id) && (change.status === 'loading' || change.url)) close(id); });
