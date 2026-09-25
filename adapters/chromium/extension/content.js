(() => {
  if (globalThis.grammarAdapterInstalled) return;
  globalThis.grammarAdapterInstalled = true;
  const {DocumentSession, MAX_TEXT} = globalThis.GrammarCore;
  let epoch = null, session = null, editor = null, chosen = null, composing = false;
  let root = null, shadow = null, status = null, list = null, card = null;
  let timer = null, heartbeat = null, expiry = null, observer = null, inflight = false, pending = null;
  let selectedIndex = null, editBox = null, sequence = 0;
  const sensitive = /password|passwd|secret|token|credit|card.?number|ssn|social.?security|medical|health|otp|one.?time|verification|auth|private|sensitive/i;
  function labelText(label) {
    if (!label) return '';
    const queue=[label];let text='',visited=0;
    while(queue.length && visited++<32 && text.length<512) {
      const node=queue.shift();
      if(node.nodeType===Node.TEXT_NODE) text+=node.substringData(0,512-text.length);
      else if(node.nodeType===Node.ELEMENT_NODE && !node.matches('textarea,input,[contenteditable]')) {
        if(node.childNodes.length>32)return 'sensitive';
        queue.push(...node.childNodes);
      }
    }
    return queue.length ? 'sensitive' : text;
  }
  function supported(el) {
    if (!el || !el.isConnected || el.ownerDocument !== document || el.getRootNode() !== document ||
        el.closest('[inert],[hidden],[aria-hidden="true"],[data-grammar-sensitive],[data-sensitive]:not([data-sensitive="false"])') || el.matches(':disabled,[readonly],[aria-readonly="true"],[aria-disabled="true"]')) return false;
    const style = getComputedStyle(el);
    if (style.display === 'none' || style.visibility !== 'visible' || el.getClientRects().length === 0) return false;
    // Never read value/textContent before these structural and sensitivity checks.
    const hints = ['id','name','type','autocomplete','aria-label','data-sensitive'].map(k => el.getAttribute(k) || '').join(' ');
    if(el.labels?.length>8)return false;
    const labels = [...(el.labels || [])].map(labelText).join(' ');
    const labelled = (el.getAttribute('aria-labelledby') || '').slice(0,512).split(/\s+/).slice(0,8)
      .map(id => labelText(document.getElementById(id))).join(' ');
    const ancestorLabel = el.parentElement?.closest('[aria-label]')?.getAttribute('aria-label') || '';
    if (sensitive.test(hints+' '+labels+' '+labelled+' '+ancestorLabel) || (el.getAttribute('autocomplete') && !['on','off'].includes(el.getAttribute('autocomplete'))) ||
        el.closest('form')?.querySelector('input[type="password"]')) return false;
    if (el.tagName === 'TEXTAREA') return !el.readOnly;
    return el.getAttribute('contenteditable') === 'true' &&
      !el.parentElement?.closest('[contenteditable="true"]') &&
      el.childNodes.length <= 16 && [...el.childNodes].every(n => n.nodeType === Node.TEXT_NODE);
  }
  function read() {
    if (!session?.active || !supported(editor) || composing || document.hidden) return null;
    const length = editor.tagName === 'TEXTAREA' ? editor.textLength : [...editor.childNodes].reduce((n,node)=>n+node.length,0);
    if (length > MAX_TEXT) return null;
    const text = editor.tagName === 'TEXTAREA' ? editor.value : editor.textContent;
    return text.length <= MAX_TEXT ? text : null;
  }
  function button(label, callback) {
    const b = document.createElement('button'); b.textContent = label; b.type = 'button';
    b.addEventListener('click', event => { if (event.isTrusted) callback(event); });
    return b;
  }
  function wipeField() {
    clearTimeout(timer); clearTimeout(expiry); pending = null;
    session?.clear(); session = null; editor = null; composing = false;
    selectedIndex = null; editBox = null;
    list?.replaceChildren(); card?.replaceChildren();
  }
  function disable(notify = false) {
    const oldEpoch = epoch; epoch = null; wipeField();
    clearInterval(heartbeat); heartbeat = null; observer?.disconnect(); observer = null;
    document.removeEventListener('focusin', focus);
    document.removeEventListener('input', input, true);
    document.removeEventListener('compositionstart', compositionStart, true);
    document.removeEventListener('compositionend', compositionEnd, true);
    document.removeEventListener('keydown', keyboard, true);
    document.removeEventListener('visibilitychange', visibility);
    window.removeEventListener('pagehide', pagehide);
    root?.remove(); root = shadow = status = list = card = null;
    if (notify && oldEpoch) chrome.runtime.sendMessage({op:'disable',epoch:oldEpoch}).catch(() => {});
  }
  function focus(event) {
    if (root?.contains(event.target)) return;
    chosen = supported(event.target) ? event.target : null;
  }
  function invalidate(clear = true) {
    if (!session) return;
    if(clear) session.cache = []; list.replaceChildren(); card.replaceChildren(); selectedIndex = null;
  }
  function input(event) { if (event.target === editor) { invalidate(false); schedule(); } }
  function compositionStart(event) { if (event.target === editor) { composing = true; invalidate(false); clearTimeout(timer); } }
  function compositionEnd(event) { if (event.target === editor) { composing = false; schedule(); } }
  function keyboard(event) { if (event.isTrusted && event.altKey && event.shiftKey && event.code === 'KeyG') { event.preventDefault(); disable(true); } }
  function visibility() { if (document.hidden) { wipeField(); if (status) status.textContent = 'Paused. Enable the field again when ready.'; } }
  function pagehide() { disable(true); }
  function schedule() { clearTimeout(timer); timer = setTimeout(analyzeChanged, 250); }
  function analyzeChanged() {
    if (!session) return;
    const text = read();
    if (text === null) { wipeField(); status.textContent = 'Field unavailable. No text was sent.'; return; }
    const request = session.update(text);
    if (!request) return;
    pending = request; pump();
  }
  async function pump() {
    if (inflight || !pending || !session) return;
    const request = pending, owner = session; pending = null; inflight = true;
    try {
      const response = await chrome.runtime.sendMessage({version:1,op:'analyze',id:String(++sequence),epoch,
        revision:request.revision,text:request.text});
      if (session !== owner || !epoch) return;
      if (!response || response.error) { status.textContent = 'Local engine unavailable. Disable, check host, then enable again.'; invalidate(); return; }
      if (response.epoch !== epoch || response.revision !== request.revision || read() !== owner.text) { invalidate(); schedule(); return; }
      if (owner.publish(request,response.suggestions)) render();
    } catch { if (session === owner && status) status.textContent = 'Connection lost. Enable this document again.'; }
    finally { inflight = false; if (pending) pump(); }
  }
  function render() {
    list.replaceChildren(); card.replaceChildren(); selectedIndex = null;
    status.textContent = 'Local Instant active · ' + session.cache.length + ' suggestions · no cloud';
    session.cache.forEach((s,index) => {
      const b = button((s.source || 'Insert') + ' → ' + s.replacement, () => show(index));
      b.setAttribute('aria-label', 'Suggestion ' + (index+1) + ': ' + s.message);
      b.addEventListener('mouseenter', () => show(index));
      b.addEventListener('focus', () => show(index));
      list.append(b);
    });
    // Equivalent annotation beside the field. It never wraps or mutates editor DOM.
    const rect = editor.getBoundingClientRect();
    root.style.top = Math.max(8, Math.min(innerHeight - 200, rect.top)) + 'px';
    root.style.left = Math.max(8, Math.min(innerWidth - 360, rect.right + 8)) + 'px';
    root.style.right = 'auto';
    clearTimeout(expiry); expiry = setTimeout(() => invalidate(), 60000);
  }
  function show(index) {
    const s = session?.suggestion(index);
    if (!s) return;
    if (selectedIndex === index && editBox) return;
    selectedIndex = index; card.replaceChildren();
    const label = document.createElement('label'); label.textContent = s.message + ' · Edit replacement';
    editBox = document.createElement('textarea'); editBox.value = s.replacement; editBox.maxLength = MAX_TEXT;
    editBox.setAttribute('aria-label','Edit replacement'); label.append(editBox); card.append(label);
    card.append(button('Accept', () => apply(index, s.replacement)),button('Apply edit', () => apply(index,editBox.value)),
      button('Dismiss', () => {session.dismiss(index);render();}),button('Ignore', () => {session.dismiss(index,true);render();}),
      button('Copy', async () => { try { await navigator.clipboard.writeText(editBox.value); status.textContent='Copied'; } catch { editBox.focus(); editBox.select(); status.textContent='Press Ctrl+C to copy selected replacement'; } }));
    const deep = document.createElement('p'); deep.textContent = 'Deep is unavailable in this candidate. No Provider is called.'; card.append(deep);
  }
  function apply(index, replacement) {
    const current = read(), mutation = current !== null ? session.replacement(index,current,replacement) : null;
    if (!mutation || composing) { if(current!==null) session?.update(current); invalidate(); status.textContent='Changed or unsupported field. Apply rejected.'; return; }
    const target = editor, owner = session;
    // beforeinput handlers may modify the page. Revalidate AFTER synchronous page callbacks.
    const before = new InputEvent('beforeinput',{bubbles:true,cancelable:true,inputType:'insertReplacementText',data:replacement});
    if (!target.dispatchEvent(before) || session !== owner || editor !== target || read() !== current) {
      invalidate(); status.textContent='Editor rejected replacement or changed. Copy only.'; return;
    }
    if (target.tagName === 'TEXTAREA') {
      const start=target.selectionStart,end=target.selectionEnd,direction=target.selectionDirection;
      HTMLTextAreaElement.prototype.setRangeText.call(target,replacement,mutation.start,mutation.end,'preserve');
      // Preserve unrelated caret/selection, with browser setRangeText offset adjustment.
      if (end <= mutation.start) target.setSelectionRange(start,end,direction);
    } else {
      // Only simple text nodes are admitted. No rich editor formatting is rewritten.
      const nodes=[...target.childNodes]; let offset=0,startNode=null,endNode=null,startOffset=0,endOffset=0;
      for (const node of nodes) {
        if (!startNode && mutation.start <= offset+node.length) {startNode=node;startOffset=mutation.start-offset;}
        if (!endNode && mutation.end <= offset+node.length) {endNode=node;endOffset=mutation.end-offset;}
        offset+=node.length;
      }
      if (!startNode || !endNode) { invalidate(); status.textContent='Range unavailable. Copy only.'; return; }
      const range=document.createRange();range.setStart(startNode,startOffset);range.setEnd(endNode,endOffset);
      range.deleteContents();range.insertNode(document.createTextNode(replacement));
    }
    target.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertReplacementText',data:replacement}));
    const actual=read(); if(actual!==null && session===owner) owner.update(actual); invalidate();
    status.textContent=actual===mutation.next ? 'Applied; content verified. Native undo is not guaranteed.' : 'Editor changed during Apply. No retry; inspect the field.';
    schedule();
  }
  function enable(newEpoch) {
    disable(); epoch=newEpoch; chosen=supported(document.activeElement) ? document.activeElement : null;
    root=document.createElement('div');root.id='grammar-local-assist';
    root.style.cssText='position:fixed;right:12px;top:12px;z-index:2147483647;width:340px;';
    shadow=root.attachShadow({mode:'open'});
    const style=document.createElement('style');style.textContent=':host{all:initial}section{font:13px system-ui;color:#15202b;background:#fff;border:2px solid #18684b;border-radius:10px;padding:10px;box-shadow:0 4px 16px #0003;max-height:50vh;overflow:auto}button{font:inherit;margin:3px;padding:6px;border:1px solid #779;border-radius:4px;background:#f4f8f6;color:#15202b;cursor:pointer}button:focus-visible,textarea:focus-visible{outline:3px solid #2065cd}textarea{box-sizing:border-box;width:100%;min-height:55px}p{margin:5px 0}';
    const panel=document.createElement('section');panel.setAttribute('aria-label','Grammar local writing assist');
    status=document.createElement('p');status.setAttribute('role','status');status.textContent='Document enabled. Select a non-sensitive field, then enable it.';
    list=document.createElement('div');list.setAttribute('aria-label','Cached suggestions');card=document.createElement('div');
    panel.append(status,button('Enable this field',()=>{
      const target=chosen;
      wipeField();
      if (!supported(target)) {status.textContent='Unsupported or sensitive field. No text read.';return;}
      editor=target;session=new DocumentSession(epoch);status.textContent='Local Instant active';schedule();
    }),button('Pause field',()=>{wipeField();status.textContent='Paused; cache cleared';}),button('Disable document',()=>disable(true)),list,card);
    shadow.append(style,panel);document.documentElement.append(root);
    document.addEventListener('focusin',focus);document.addEventListener('input',input,true);
    document.addEventListener('compositionstart',compositionStart,true);document.addEventListener('compositionend',compositionEnd,true);
    document.addEventListener('keydown',keyboard,true);document.addEventListener('visibilitychange',visibility);window.addEventListener('pagehide',pagehide);
    observer=new MutationObserver(mutations=>{
      if (!root?.isConnected) {disable(true);return;}
      if (editor && !supported(editor)) {wipeField();status.textContent='Field changed capability. Disabled.';}
      else if (editor && editor.tagName!=='TEXTAREA' && mutations.some(m=>m.target===editor || editor.contains(m.target))) {invalidate(false);schedule();}
    });
    observer.observe(document.documentElement,{subtree:true,childList:true,attributes:true,characterData:true,
      attributeFilter:['type','readonly','disabled','hidden','inert','contenteditable','autocomplete','data-sensitive','data-grammar-sensitive','aria-hidden','aria-label','aria-labelledby','aria-disabled','aria-readonly','id','name','style','class']});
    heartbeat=setInterval(async()=>{
      const activeEpoch=epoch;
      try {const response=await chrome.runtime.sendMessage({op:'heartbeat',epoch});if (epoch===activeEpoch && !response?.ok) disable();}
      catch {if (epoch===activeEpoch) disable();}
      // Bounded reconciliation catches programmatic textarea value changes without key logging.
      if (session && !composing && !document.hidden) {const current=read();if(current===null){wipeField();}else if(current!==session.text){invalidate(false);schedule();}}
    },1000);
  }
  chrome.runtime.onMessage.addListener((message,sender,respond)=>{
    if(sender.id!==chrome.runtime.id)return;
    if(message.op==='enable'&&typeof message.epoch==='string'){enable(message.epoch);respond({ready:true});}
    else if(message.op==='disable'){disable();respond({ok:true});}
  });
})();
