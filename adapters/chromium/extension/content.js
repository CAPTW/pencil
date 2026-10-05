(() => {
  if (globalThis.grammarAdapterInstalled) return;
  globalThis.grammarAdapterInstalled = true;
  const {DocumentSession, SelectionReview, MAX_TEXT} = globalThis.GrammarCore;
  let epoch = null, session = null, editor = null, chosen = null, composing = false;
  let root = null, shadow = null, status = null, list = null, card = null;
  let panel = null, panelToggle = null, layoutObserver = null, layoutFrame = null;
  let timer = null, heartbeat = null, expiry = null, observer = null, inflight = false, pending = null, deferredInstant = null;
  let selectedIndex = null, editBox = null, sequence = 0;
  let deepProvider = null, deepRequest = null, deepButton = null, draftVersion = 0;
  let enableTarget = null, reanalyzeAll = false, held = false, busyWaits = 0;
  let selectionReview = null, reviewInput = null, reviewAccept = null, reviewTimer = null;
  let reviewRange = null, reviewOwnsFocus = false;
  let desktopReview = null, desktopReviewTimer = null, desktopReviewObserver = null;
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
      el.childNodes.length <= 16 && ([...el.childNodes].every(n => n.nodeType === Node.TEXT_NODE) ||
        ([...el.childNodes].filter(n=>n.nodeName==='BR').length===1 &&
          [...el.childNodes].every(n=>n.nodeType===Node.TEXT_NODE ? n.length===0 : n.nodeName==='BR' && n.attributes.length===0 && n.childNodes.length===0)));
  }
  function read() {
    if (!session?.active || !supported(editor) || composing || document.hidden) return null;
    const length = editor.tagName === 'TEXTAREA' ? editor.textLength : [...editor.childNodes].reduce((n,node)=>n+(node.nodeType===Node.TEXT_NODE ? node.length : 0),0);
    if (length > MAX_TEXT) return null;
    const text = editor.tagName === 'TEXTAREA' ? editor.value : editor.textContent;
    return text.length <= MAX_TEXT ? text : null;
  }
  function button(label, callback) {
    const b = document.createElement('button'); b.textContent = label; b.type = 'button';
    b.addEventListener('click', event => { if (event.isTrusted) callback(event); });
    return b;
  }
  function cancelDeepUI() {
    const request=deepRequest;deepRequest=null;
    if(request) chrome.runtime.sendMessage({op:'cancel-deep',epoch:request.epoch,id:request.id}).catch(()=>{});
  }
  function wipeField() {
    clearDesktopReview();
    clearReview();
    cancelDeepUI();
    clearTimeout(timer); clearTimeout(expiry); pending = null; deferredInstant = null; held = false; busyWaits = 0;
    session?.clear(); session = null; editor = null; composing = false;
    selectedIndex = null; editBox = null;
    list?.replaceChildren(); card?.replaceChildren();
  }
  function disable(notify = false) {
    const oldEpoch = epoch; wipeField(); epoch = null; deepProvider=null; deepButton=null;
    clearInterval(heartbeat); heartbeat = null; observer?.disconnect(); observer = null;
    layoutObserver?.disconnect(); layoutObserver = null;
    if (layoutFrame !== null) cancelAnimationFrame(layoutFrame); layoutFrame = null;
    window.removeEventListener('resize', scheduleLayout);
    window.removeEventListener('scroll', scheduleLayout, true);
    document.removeEventListener('focusin', focus);
    document.removeEventListener('input', input, true);
    document.removeEventListener('compositionstart', compositionStart, true);
    document.removeEventListener('compositionend', compositionEnd, true);
    document.removeEventListener('keydown', keyboard, true);
    document.removeEventListener('selectionchange', selectionChanged);
    document.removeEventListener('select', selectionChanged, true);
    document.removeEventListener('pointerdown', reviewFocusStart, true);
    document.removeEventListener('focusout', reviewFocusLeaving, true);
    document.removeEventListener('visibilitychange', visibility);
    window.removeEventListener('pagehide', pagehide);
    root?.remove(); root = shadow = status = list = card = panel = panelToggle = enableTarget = null; reanalyzeAll = false;
    if (notify && oldEpoch) chrome.runtime.sendMessage({op:'disable',epoch:oldEpoch}).catch(() => {});
  }
  // The field "Enable this field" would enable: the text field the user was
  // last in. An unsupported one stays chosen and is refused, never swapped
  // for another field. Links and buttons passed on the way to the panel do
  // not change it; the panel names the field.
  function fieldLike(el) {
    return el instanceof Element && (el.matches('textarea,input,select') || el.isContentEditable);
  }
  function describeField(el) {
    const label = [...(el.labels || [])].map(labelText).join(' ') || el.getAttribute('aria-label') ||
      el.getAttribute('placeholder') || el.getAttribute('name') || el.id || (el.tagName === 'TEXTAREA' ? 'text area' : 'editable text');
    return label.replace(/\s+/g, ' ').trim().slice(0, 40);
  }
  function choose(el) {
    chosen = el;
    if (enableTarget) enableTarget.textContent = chosen ?
      'Field to enable: ' + describeField(chosen) + (supported(chosen) ? '' : ' (not supported)') : 'Select a text field in the page first.';
  }
  function focus(event) {
    if(desktopReview && event.target!==editor)clearDesktopReview();
    if (root?.contains(event.target)) {reviewFocusStart(event);return;}
    if(selectionReview && event.target!==editor) revokeReview('Review target changed. Copy or dismiss the draft.');
    if (fieldLike(event.target)) choose(event.target);
  }
  function invalidate(clear = true) {
    clearDesktopReview();
    if (!session) return;
    if(reviewInput?.isConnected) {if(clear)session.cache=[];list.replaceChildren();revokeReview('Review expired or field changed. Copy or dismiss the draft.');return;}
    if(clear) session.cache = []; list.replaceChildren(); card.replaceChildren(); selectedIndex = null;
  }
  function input(event) { if (event.target === editor) { if (event.isTrusted) held = false; cancelDeepUI(); invalidate(false); schedule(); } }
  // The outcome of an explicit action stays readable until the user acts
  // again: later re-checks update the list, not this message.
  function announce(message) { if (status) { status.textContent = message; held = true; } }
  function deepSending(provider) { return 'Sending current enabled field to '+provider+'. Cancel or pause to stop.'; }
  // Keeps the suggestions still valid for `text` and re-checks what changed.
  function resync(text) {
    const request = text !== null && session ? session.update(text) : null;
    invalidate(false);
    if (request) pending = request;
    render(); schedule();
  }
  function compositionStart(event) { if (event.target === editor) { composing = true; held = false; cancelDeepUI(); invalidate(false); clearTimeout(timer); } }
  function compositionEnd(event) { if (event.target === editor) { composing = false; schedule(); } }
  function scheduleLayout() {
    if (!root || layoutFrame !== null) return;
    layoutFrame = requestAnimationFrame(() => {
      layoutFrame = null;
      if (!root?.isConnected) return;
      // Geometry only: layout changes never read document text or invoke analysis.
      panel.style.maxHeight = Math.max(0, Math.min(innerHeight / 2,
        innerHeight - panelToggle.getBoundingClientRect().height - 16)) + 'px';
      const bounds = root.getBoundingClientRect();
      const anchor = editor?.isConnected ? editor.getBoundingClientRect() : null;
      const left = anchor ? anchor.right + 8 : innerWidth - bounds.width - 12;
      const top = anchor ? anchor.top : 12;
      root.style.left = Math.max(8, Math.min(innerWidth - bounds.width - 8, left)) + 'px';
      root.style.top = Math.max(8, Math.min(innerHeight - bounds.height - 8, top)) + 'px';
      root.style.right = 'auto';
    });
  }
  function setPanelVisible(visible) {
    if (!panel) return;
    panel.hidden = !visible; scheduleLayout();
    panelToggle.setAttribute('aria-expanded', String(visible));
    panelToggle.textContent = visible ? 'Hide Grammar panel' : 'Grammar enabled · Show panel';
    if (!visible) {
      // Retain the draft/cache, but release focus and pointer access to the page.
      if (supported(editor)) editor.focus({preventScroll:true});
      else panelToggle.focus({preventScroll:true});
    }
  }
  function keyboard(event) {
    if (!event.isTrusted) return;
    if (event.altKey && event.shiftKey && event.code === 'KeyG') { event.preventDefault(); disable(true); }
    else if(event.altKey && event.shiftKey && !event.ctrlKey && !event.metaKey && !event.isComposing && event.code==='KeyR' && event.target===editor) {
      event.preventDefault();openSelectionReview();
    }
    // Keyboard users enable exactly the field they are in.
    else if (event.altKey && event.shiftKey && !event.ctrlKey && !event.metaKey && !event.isComposing && event.code === 'KeyE' &&
        !event.composedPath().includes(root) && fieldLike(event.target)) {
      event.preventDefault(); enableField(event.target);
    }
    else if (!event.isComposing && event.key === 'Escape' && event.composedPath().includes(root) && !panel.hidden) {
      event.preventDefault(); event.stopPropagation(); setPanelVisible(false);
    }
  }
  function visibility() { if (document.hidden) { wipeField(); if (status) status.textContent = 'Paused. Enable the field again when ready.'; } }
  function pagehide() { disable(true); }
  function schedule() { clearTimeout(timer); if (!composing) timer = setTimeout(analyzeChanged, 250); }
  function analyzeChanged() {
    if (!session || composing) return;
    const text = read();
    if (text === null) { wipeField(); status.textContent = 'Field unavailable. No text was sent.'; return; }
    // A response received during preedit stays memory-only until committed text
    // can be checked. Never read preedit or publish it as document authority.
    const deferred = deferredInstant; deferredInstant = null;
    if (deferred && deferred.owner === session && text === session.text)
      session.publish(deferred.request, deferred.suggestions);
    let request = session.update(text);
    // After a discarded or failed reply, or expiry, check every line again,
    // not only the last change.
    if (reanalyzeAll) { reanalyzeAll = false; request = session.full() || request; }
    if (!request) { render(); pump(); return; }
    pending = request; pump();
  }
  async function pump() {
    if (inflight || !pending || !session || composing) return;
    const request = pending, owner = session; pending = null; inflight = true;
    try {
      const response = await chrome.runtime.sendMessage({version:1,op:'analyze',id:String(++sequence),epoch,
        revision:request.revision,text:request.text});
      if (session !== owner || !epoch) return;
      if (response?.error === 'busy' && busyWaits < 150) {
        // A Deep request (or its cleanup) is still using the host: wait, then
        // re-check. A request that never ends does not keep the page waiting.
        busyWaits++; status.textContent = 'Waiting for the Deep request to finish…';
        reanalyzeAll = true; clearTimeout(timer); timer = setTimeout(analyzeChanged, 1000); return;
      }
      busyWaits = 0;
      if (!response || response.error) { status.textContent = 'Local engine unavailable. Disable, check host, then enable again.'; invalidate(); return; }
      if (composing && response.epoch === epoch && response.revision === request.revision) {
        deferredInstant = {owner,request,suggestions:response.suggestions};return;
      }
      if (response.epoch !== epoch || response.revision !== request.revision || read() !== owner.text) {
        // The text moved on while this reply was in flight: keep what is still
        // valid and check the whole field again.
        reanalyzeAll = true; invalidate(false); schedule(); return;
      }
      if (owner.publish(request,response.suggestions)) render();
    } catch { if (session === owner && status) status.textContent = 'Connection lost. Enable this document again.'; }
    finally { inflight = false; if (pending) pump(); }
  }
  function render() {
    if(reviewInput?.isConnected)return; // Never replace a review the user is editing.
    list.replaceChildren(); card.replaceChildren(); selectedIndex = null;
    const now = Date.now();
    session.cache = session.cache.filter(s => now < s.expires);
    if (deepRequest) status.textContent = deepSending(deepRequest.provider);
    else if (!held) status.textContent = session.cache.some(s=>s.rule.startsWith('deep:')) ? 'Deep result cached for review. Apply is explicit.' : 'Local Instant active · ' + session.cache.length + ' suggestions · no cloud';
    // A suggestion opens only when it is chosen (click, Enter or Space).
    // Moving through the list, by keyboard or pointer, never changes the card,
    // and a button opens only the suggestion it was drawn for.
    session.cache.forEach((s,index) => {
      const b = button((s.source || 'Insert') + ' → ' + s.replacement, () => {
        if (session?.cache[index] !== s) { render(); announce('The suggestion list changed. Choose the suggestion again.'); return; }
        show(index, true);
      });
      b.setAttribute('aria-label', 'Suggestion ' + (index+1) + ': ' + s.message);
      list.append(b);
    });
    // Equivalent annotation beside the field. It never wraps or mutates editor DOM.
    scheduleLayout();
    scheduleExpiry();
  }
  // Each suggestion has its own lifetime; the timer follows the earliest one.
  function scheduleExpiry(live = session?.cache ?? []) {
    clearTimeout(expiry);
    if (!live.length) return;
    expiry = setTimeout(expire, Math.max(0, Math.min(...live.map(s => s.expires)) - Date.now()));
  }
  function expire() {
    if (!session) return;
    const at = Date.now(), live = session.cache.filter(s => at < s.expires);
    const open = selectedIndex === null ? null : session.cache[selectedIndex];
    if (live.length) {
      // Only some expired: drop them. A card being edited keeps its list
      // positions until its own suggestion expires.
      if (!dirtyDraft()) { render(); return; }
      if (open && at < open.expires) { scheduleExpiry(live); return; }
    }
    reanalyzeAll = true;
    if(!dirtyDraft()) {invalidate();status.textContent=deepRequest ? deepSending(deepRequest.provider) : 'Suggestions expired. Edit the field to check it again.';return;}
    session.cache=[];list.replaceChildren();
    for(const action of card.querySelectorAll('button')) {
      if(['Accept','Apply edit','Ignore'].includes(action.textContent))action.disabled=true;
    }
    status.textContent='Suggestion expired. Edited draft kept for Copy or Dismiss; Apply is disabled.';
  }
  function dirtyDraft() {
    return editBox?.isConnected && selectedIndex!==null &&
      editBox.value!==session?.suggestion(selectedIndex)?.replacement;
  }
  function show(index, chosenByUser = false) {
    if(reviewInput?.isConnected){announce('Copy or dismiss the selected-text review first.');return;}
    const s = session?.suggestion(index);
    if (!s) return;
    if (selectedIndex === index && editBox) return;
    if (dirtyDraft() && !chosenByUser) return;
    if (chosenByUser) held = false;
    selectedIndex = index; card.replaceChildren();
    const label = document.createElement('label'); label.textContent = s.message + ' · Edit replacement';
    editBox = document.createElement('textarea'); editBox.value = s.replacement; editBox.maxLength = MAX_TEXT;
    editBox.setAttribute('aria-label','Edit replacement'); label.append(editBox); card.append(label);
    // Every action is bound to this exact suggestion, not to its list position.
    const same = () => session?.cache[index] === s;
    const stale = () => { render(); announce('The suggestion list changed. Choose the suggestion again.'); };
    card.append(button('Accept', () => apply(index, s, s.replacement)),button('Apply edit', () => apply(index, s, editBox.value)),
      button('Dismiss', () => { if (!same()) return stale(); session.dismiss(index);render();focusEditor(); }),
      button('Ignore', () => { if (!same()) return stale(); session.dismiss(index,true);render();focusEditor(); }),
      button('Copy', async () => { try { await navigator.clipboard.writeText(editBox.value); announce('Copied'); } catch { editBox.focus(); editBox.select(); announce('Press Ctrl+C to copy selected replacement'); } }));
    editBox.addEventListener('input',()=>draftVersion++);
    if (chosenByUser) editBox.focus({preventScroll:true});
  }
  function focusEditor() { if (supported(editor)) editor.focus({preventScroll:true}); }
  function updateDeepPolicy(provider) {
    if(provider!==deepProvider) {cancelDeepUI();if(session?.cache.some(s=>s.rule.startsWith('deep:'))){session.cache=[];invalidate();}}
    deepProvider=['codex','antigravity','claude'].includes(provider) ? provider : null;
    if(deepButton) {deepButton.disabled=!deepProvider;deepButton.textContent=deepProvider ? 'Send current field to '+deepProvider+' for Deep review' : 'Deep requires document permission in the popup';}
  }
  async function requestDeep() {
    if(reviewInput?.isConnected){announce('Copy or dismiss the selected-text review before sending Deep.');return;}
    if(dirtyDraft()) {status.textContent='Apply, copy, or dismiss your edited draft before requesting Deep.';return;}
    if(!deepProvider || !session || deepRequest || inflight) {status.textContent='Deep unavailable or local request still running. Try explicitly when ready.';return;}
    const text=read();if(text===null || !text.trim())return;
    clearTimeout(timer);pending=null;
    // A change the page made without an input event: close any card opened for
    // the old text before sending.
    if(session.update(text))render();
    const owner=session, target=editor, provider=deepProvider, draft=draftVersion;
    const request={version:1,op:'deep',id:String(++sequence),epoch,revision:owner.revision,text,provider};
    deepRequest=request;held=false;status.textContent=deepSending(provider);
    try {
      const bytes=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text));
      const hash=[...new Uint8Array(bytes)].map(b=>b.toString(16).padStart(2,'0')).join('');
      if(deepRequest!==request || session!==owner || read()!==text)return;
      const result=await chrome.runtime.sendMessage(request);
      if(deepRequest!==request || session!==owner || editor!==target || epoch!==request.epoch)return;
      if(result?.error) {announce('Deep unavailable: '+String(result.error).slice(0,80)+'. No retry or fallback.');return;}
      if(read()!==text || owner.revision!==request.revision || draftVersion!==draft || deepProvider!==provider ||
          result?.epoch!==request.epoch || result.revision!==request.revision || result.provider!==provider || result.source_sha256!==hash ||
          result.cleanup_complete!==true || typeof result.replacement!=='string' || result.replacement.length>MAX_TEXT) {announce('Deep result stale or invalid. No changes applied.');return;}
      // A card the user opened while waiting is never swapped for the Deep
      // one under the pointer: the result waits in the list.
      const cardOpen = selectedIndex !== null && editBox?.isConnected;
      owner.cache=[{start:0,end:text.length,source:text,replacement:result.replacement,rule:'deep:'+provider,message:'Deep '+provider,
        epoch,revision:owner.revision,expires:Date.now()+60000}];
      deepRequest=null;render();
      if (cardOpen) announce('Deep result ready: choose it in the list.'); else show(0);
    } catch {if(deepRequest===request)announce('Deep connection unavailable. No retry.');}
    finally {if(deepRequest===request)deepRequest=null;}
  }
  function apply(index, expected, replacement) {
    // The card's own suggestion, still in the same place: never a neighbour
    // that moved into its list position.
    if (!session || session.cache[index] !== expected) {
      if (session) render();
      announce('The suggestion list changed. Choose the suggestion again.');
      return;
    }
    const current = read(), mutation = current !== null ? session.replacement(index,current,replacement) : null;
    if (!mutation || composing) { resync(current); announce('Changed or unsupported field. Apply rejected.'); return; }
    commitMutation(current,mutation,()=>session?.cache[index]===expected &&
      session.replacement(index,current,replacement)!==null);
  }
  function commitMutation(current,mutation,valid,consume=()=>{},restoreFocus=true) {
    const replacement=mutation.replacement;
    const target = editor, owner = session;
    // beforeinput handlers may modify the page. Revalidate AFTER synchronous page callbacks.
    const before = new InputEvent('beforeinput',{bubbles:true,cancelable:true,inputType:'insertReplacementText',data:replacement});
    if (!target.dispatchEvent(before) || session !== owner || editor !== target || read() !== current || !valid()) {
      if (session===owner && editor===target) resync(read()); else invalidate();
      announce('Editor rejected replacement or changed. Copy only.'); return 0;
    }
    consume(); // One-shot authority is consumed before any DOM write or callback.
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
      if (!startNode || !endNode) { invalidate(); announce('Range unavailable. Copy only.'); return 0; }
      // Edit the existing text nodes in place: new nodes would push a simple
      // field past the supported node count after a few Accepts.
      if (startNode === endNode) startNode.replaceData(startOffset, endOffset - startOffset, replacement);
      else {
        startNode.replaceData(startOffset, startNode.length - startOffset, replacement);
        endNode.deleteData(0, endOffset);
        for (let node = startNode.nextSibling; node && node !== endNode;) { const next = node.nextSibling; node.remove(); node = next; }
      }
    }
    target.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertReplacementText',data:replacement}));
    const actual=read();
    if (session===owner) resync(actual); else invalidate();
    announce(actual===mutation.next ? 'Applied; content verified. Native undo is not guaranteed.' : 'Editor changed during Apply. No retry; inspect the field.');
    if(restoreFocus)focusEditor();
    return actual===mutation.next && session===owner && editor===target ? 3 : 4;
  }
  function clearDesktopReview() {
    clearTimeout(desktopReviewTimer);desktopReviewTimer=null;
    desktopReviewObserver?.disconnect();desktopReviewObserver=null;
    desktopReview?.grant.cancel();desktopReview=null;
  }
  const reviewToken=value=>typeof value==='string' && /^[a-f0-9]{8}(-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(value);
  function captureDesktopReview(id) {
    // Only the authenticated native-host worker route may call this entry.
    // It does not enable a document/field or infer permission from selection.
    if(!reviewToken(id) || desktopReview || reviewInput?.isConnected || dirtyDraft() ||
        !document.hasFocus() || document.activeElement!==editor)return {status:0,text:'',ticket:null};
    const current=read(),range=selectedRange();
    if(current===null || !range)return {status:0,text:'',ticket:null};
    session.update(current);
    const grant=SelectionReview.capture(session,range.start,range.end);
    if(!grant)return {status:0,text:'',ticket:null};
    const text=current.slice(range.start,range.end);
    if(text.includes('\0')){grant.cancel();return {status:0,text:'',ticket:null};}
    cancelDeepUI();
    // A dedicated observer preserves ABA evidence even during synchronous
    // beforeinput handlers, before MutationObserver callbacks can run.
    const ancestors=[];for(let n=editor.parentElement;n;n=n.parentElement){ancestors.push(n);if(ancestors.length>32){grant.cancel();return {status:0,text:'',ticket:null};}}
    desktopReviewObserver=new MutationObserver(clearDesktopReview);
    desktopReviewObserver.observe(editor,{attributes:true,characterData:true,childList:true,subtree:true});
    for(const ancestor of ancestors)desktopReviewObserver.observe(ancestor,{attributes:true,childList:true});
    const ticket=crypto.randomUUID();desktopReview={id,ticket,grant};
    desktopReviewTimer=setTimeout(clearDesktopReview,30000);
    announce('Selected text captured for desktop review. Replacement requires Accept.');
    return {status:2,text,ticket};
  }
  function applyDesktopReview(id,ticket,text) {
    const heldReview=desktopReview;
    if(!heldReview || heldReview.id!==id || heldReview.ticket!==ticket)return {status:0};
    if(desktopReviewObserver?.takeRecords().length){clearDesktopReview();return {status:0};}
    const grant=heldReview.grant,current=read(),range=selectedRange();
    const mutation=current!==null && range ? grant.replacement(current,range.start,range.end,text) : null;
    if(!mutation){clearDesktopReview();return {status:0};}
    try {
      const status=commitMutation(current,mutation,()=>{
        const latest=selectedRange();
        return desktopReview===heldReview && !desktopReviewObserver?.takeRecords().length && latest && !!grant.replacement(read(),latest.start,latest.end,text);
      },clearDesktopReview,false);
      clearDesktopReview();return {status};
    } catch {clearDesktopReview();return {status:4};} // Never retry an uncertain write.
  }
  function clearReview() {
    clearTimeout(reviewTimer);reviewTimer=null;selectionReview?.cancel();selectionReview=null;
    reviewInput=reviewAccept=null;
    reviewRange=null;reviewOwnsFocus=false;
  }
  function revokeReview(message) {
    clearTimeout(reviewTimer);reviewTimer=null;selectionReview?.cancel();
    if(reviewAccept)reviewAccept.disabled=true;
    if(reviewInput?.isConnected)announce(message);
  }
  function selectedRange() {
    if(!session?.active || !supported(editor) || composing || document.hidden)return null;
    if(editor.tagName==='TEXTAREA')return {start:editor.selectionStart,end:editor.selectionEnd};
    const selection=document.getSelection();if(!selection || selection.rangeCount!==1)return null;
    return rangeOffsets(selection.getRangeAt(0));
  }
  function rangeOffsets(range) {
    if(!range || !editor?.isConnected)return null;
    const nodes=[...editor.childNodes];
    const offset=(node,n)=>{
      if(!Number.isInteger(n)||n<0)return null;
      if(node===editor)return n<=nodes.length?nodes.slice(0,n).reduce((a,b)=>a+(b.length||0),0):null;
      const index=nodes.indexOf(node);return index>=0&&node.nodeType===Node.TEXT_NODE&&n<=node.length ? nodes.slice(0,index).reduce((a,b)=>a+(b.length||0),0)+n:null;
    };
    const start=offset(range.startContainer,range.startOffset),end=offset(range.endContainer,range.endOffset);
    return start===null||end===null?null:{start,end};
  }
  function reviewFocusStart(event) {
    if(!event.isTrusted || !selectionReview?.active || reviewOwnsFocus || !event.composedPath().includes(reviewInput))return;
    const range=selectedRange();
    if(range?.start===selectionReview.start && range?.end===selectionReview.end)reviewOwnsFocus=true;
    else revokeReview('Selection changed before entering review. Select again.');
  }
  function reviewFocusLeaving(event) {
    if(!event.isTrusted || !selectionReview?.active || event.target!==editor || event.relatedTarget!==root)return;
    const range=selectedRange();
    if(range?.start===selectionReview.start && range?.end===selectionReview.end)reviewOwnsFocus=true;
    else revokeReview('Selection changed before leaving the field. Select again.');
  }
  function currentReviewRange() {
    const actual=selectedRange();if(actual)return actual;
    if(!reviewOwnsFocus || !reviewRange || ![reviewInput,reviewAccept].includes(shadow?.activeElement))return null;
    // Chromium retargets a shadow textarea selection to the shadow host's
    // boundary. Only that exact review UI selection may use the captured Range;
    // a page selection in another node/field is never a focus exemption.
    const selection=document.getSelection();if(!selection || selection.rangeCount!==1)return null;
    const ownPoint=(node,offset)=>node===root || node?.getRootNode()===shadow ||
      (node===root.parentNode && [Array.prototype.indexOf.call(node.childNodes,root),Array.prototype.indexOf.call(node.childNodes,root)+1].includes(offset));
    if(!ownPoint(selection.anchorNode,selection.anchorOffset)||!ownPoint(selection.focusNode,selection.focusOffset))return null;
    return rangeOffsets(reviewRange);
  }
  function selectionChanged() {
    if(desktopReview) {
      const range=selectedRange(),grant=desktopReview.grant;
      if(!range || range.start!==grant.start || range.end!==grant.end)clearDesktopReview();
    }
    if(!selectionReview?.active)return;
    const range=currentReviewRange();
    if(!range || range.start!==selectionReview.start || range.end!==selectionReview.end)
      revokeReview('Selection changed. Copy or dismiss the draft, then select again.');
  }
  function openSelectionReview() {
    clearDesktopReview();
    if(reviewInput?.isConnected || dirtyDraft()){announce('Copy or dismiss the current draft first.');return;}
    const current=read(),range=selectedRange();
    if(current===null || !range){announce('Select text in the enabled field first.');return;}
    session.update(current);
    const grant=SelectionReview.capture(session,range.start,range.end);
    if(!grant){announce('Select a non-empty complete text range first.');return;}
    cancelDeepUI();draftVersion++;clearReview();selectionReview=grant;selectedIndex=null;editBox=null;
    if(editor.tagName!=='TEXTAREA')reviewRange=document.getSelection().getRangeAt(0).cloneRange();
    card.replaceChildren();list.replaceChildren();setPanelVisible(true);
    const source=document.createElement('pre');source.textContent=current.slice(range.start,range.end);source.setAttribute('aria-label','Selected source');source.style.whiteSpace='pre-wrap';
    reviewInput=document.createElement('textarea');reviewInput.value=grant.draft;reviewInput.maxLength=MAX_TEXT;reviewInput.setAttribute('aria-label','Reviewed selection replacement');
    reviewAccept=button('Accept selection',()=>{
      const text=read(),selected=currentReviewRange(),edited=reviewInput.value;
      const mutation=text!==null&&selected?grant.replacement(text,selected.start,selected.end,edited):null;
      if(!mutation){revokeReview('Review changed or expired. No replacement applied.');return;}
      commitMutation(text,mutation,()=>{
        const latest=currentReviewRange();return selectionReview===grant&&latest&&!!grant.replacement(read(),latest.start,latest.end,edited);
      },clearReview);
    });
    card.append(source,reviewInput,reviewAccept,button('Copy selection draft',async()=>{
      try{await navigator.clipboard.writeText(reviewInput.value);announce('Copied');}catch{reviewInput.focus();reviewInput.select();announce('Press Ctrl+C to copy the draft');}
    }),button('Dismiss selection review',()=>{clearReview();render();focusEditor();}));
    reviewTimer=setTimeout(()=>revokeReview('Review expired. Copy or dismiss the draft.'),30000);
    announce('Review selected text, then Accept. Cached local suggestions only; no new Deep request.');
    reviewInput.focus({preventScroll:true});
  }
  function enableField(target) {
    wipeField();
    if (!supported(target)) {status.textContent='Unsupported or sensitive field. No text read.';return;}
    choose(target);
    editor=target;session=new DocumentSession(epoch);status.textContent='Local Instant active';schedule();
  }
  function enable(newEpoch) {
    disable(); epoch=newEpoch; chosen=fieldLike(document.activeElement) ? document.activeElement : null;
    root=document.createElement('div');root.id='grammar-local-assist';
    root.style.cssText='position:fixed;right:12px;top:12px;z-index:2147483647;width:340px;max-width:calc(100vw - 16px);pointer-events:none;';
    shadow=root.attachShadow({mode:'open'});
    const style=document.createElement('style');style.textContent=':host{all:initial}section,button{pointer-events:auto}section[hidden]{display:none}section{box-sizing:border-box;font:13px system-ui;color:#15202b;background:#fff;border:2px solid #18684b;border-radius:10px;padding:10px;box-shadow:0 4px 16px #0003;max-height:50vh;overflow:auto}button{font:inherit;margin:3px;padding:6px;border:1px solid #779;border-radius:4px;background:#f4f8f6;color:#15202b;cursor:pointer}button:focus-visible,textarea:focus-visible{outline:3px solid #2065cd}textarea{box-sizing:border-box;width:100%;min-height:55px}p{margin:5px 0}';
    panel=document.createElement('section');panel.id='grammar-panel';panel.setAttribute('aria-label','Grammar local writing assist');
    panelToggle=button('Hide Grammar panel',()=>setPanelVisible(panel.hidden));
    panelToggle.setAttribute('aria-controls','grammar-panel');panelToggle.setAttribute('aria-expanded','true');
    status=document.createElement('p');status.setAttribute('role','status');status.textContent='Document enabled. Select a non-sensitive field, then enable it.';
    list=document.createElement('div');list.setAttribute('aria-label','Cached suggestions');card=document.createElement('div');
    deepButton=button('Deep requires document permission in the popup',requestDeep);deepButton.disabled=true;
    enableTarget=document.createElement('p');choose(chosen);
    const selectionHint=document.createElement('p');selectionHint.textContent='Select text in the enabled field and press Alt+Shift+R to review and Accept a replacement.';
    panel.append(status,deepButton,button('Cancel Deep',()=>{cancelDeepUI();status.textContent='Deep cancelled; cleanup receipt pending in host.';}),enableTarget,button('Enable this field',()=>enableField(chosen)),
      button('Pause field',()=>{wipeField();status.textContent='Paused; cache cleared';}),button('Disable document',()=>disable(true)),selectionHint,list,card);
    shadow.append(style,panelToggle,panel);document.documentElement.append(root);
    layoutObserver=new ResizeObserver(scheduleLayout);layoutObserver.observe(root);
    window.addEventListener('resize',scheduleLayout,{passive:true});
    window.addEventListener('scroll',scheduleLayout,{capture:true,passive:true});scheduleLayout();
    document.addEventListener('focusin',focus);document.addEventListener('input',input,true);
    document.addEventListener('compositionstart',compositionStart,true);document.addEventListener('compositionend',compositionEnd,true);
    document.addEventListener('keydown',keyboard,true);document.addEventListener('visibilitychange',visibility);window.addEventListener('pagehide',pagehide);
    document.addEventListener('selectionchange',selectionChanged);document.addEventListener('select',selectionChanged,true);
    document.addEventListener('pointerdown',reviewFocusStart,true);
    document.addEventListener('focusout',reviewFocusLeaving,true);
    observer=new MutationObserver(mutations=>{
      if (!root?.isConnected) {disable(true);return;}
      if (editor && !supported(editor)) {wipeField();status.textContent='Field changed capability. Disabled.';}
      else if (editor && editor.tagName!=='TEXTAREA' && mutations.some(m=>m.target===editor || editor.contains(m.target))) {cancelDeepUI();invalidate(false);schedule();}
    });
    observer.observe(document.documentElement,{subtree:true,childList:true,attributes:true,characterData:true,
      attributeFilter:['type','readonly','disabled','hidden','inert','contenteditable','autocomplete','data-sensitive','data-grammar-sensitive','aria-hidden','aria-label','aria-labelledby','aria-disabled','aria-readonly','id','name','style','class']});
    heartbeat=setInterval(async()=>{
      const activeEpoch=epoch;
      try {const response=await chrome.runtime.sendMessage({op:'heartbeat',epoch});if (epoch===activeEpoch) {if(!response?.ok)disable();else updateDeepPolicy(response.deepProvider);}}
      catch {if (epoch===activeEpoch) disable();}
      // Bounded reconciliation catches programmatic textarea value changes without key logging.
      if (session && !composing && !document.hidden) {const current=read();if(current===null){wipeField();}else if(current!==session.text){cancelDeepUI();invalidate(false);schedule();}}
    },1000);
  }
  chrome.runtime.onMessage.addListener((message,sender,respond)=>{
    if(sender.id!==chrome.runtime.id)return;
    if(message.op==='enable'&&typeof message.epoch==='string'){enable(message.epoch);respond({ready:true});}
    else if(message.op==='deep-policy'&&message.epoch===epoch){updateDeepPolicy(message.provider);respond({ok:true});}
    else if(message.op==='disable'&&message.epoch===epoch){disable();respond({ok:true});}
    else if(message.op==='desktop-capture'&&epoch && message.epoch===epoch){respond(captureDesktopReview(message.id));}
    else if(message.op==='desktop-apply'&&epoch && message.epoch===epoch){respond(applyDesktopReview(message.id,message.ticket,message.text));}
    else if(message.op==='desktop-cancel'&&epoch && message.epoch===epoch){
      if(desktopReview?.id===message.id)clearDesktopReview();respond({ok:true});
    }
  });
})();
