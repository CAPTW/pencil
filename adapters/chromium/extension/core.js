/* Memory-only document authority. UTF-16 offsets match the native Instant engine. */
(() => {
  const MAX_TEXT = 8192, MAX_SUGGESTIONS = 64, TTL = 60000;
  const boundary = (text, n) => n === 0 || n === text.length ||
    !(text.charCodeAt(n - 1) >= 0xd800 && text.charCodeAt(n - 1) <= 0xdbff &&
      text.charCodeAt(n) >= 0xdc00 && text.charCodeAt(n) <= 0xdfff);
  function changedWindow(before, after) {
    let start = 0, oldEnd = before.length, end = after.length;
    while (start < oldEnd && start < end && before[start] === after[start]) start++;
    while (oldEnd > start && end > start && before[oldEnd - 1] === after[end - 1]) { oldEnd--; end--; }
    // Rules are line-local. Include complete changed lines; no split surrogate pairs.
    start = start === 0 ? 0 : after.lastIndexOf('\n', start - 1) + 1;
    const lineEnd = after.indexOf('\n', end);
    end = lineEnd < 0 ? after.length : lineEnd;
    return { start, end, text: after.slice(start, end) };
  }
  class DocumentSession {
    constructor(epoch) { this.epoch = epoch; this.revision = 0; this.active = true; this.text = ''; this.cache = []; this.ignored = new Set(); }
    update(text) {
      if (!this.active || typeof text !== 'string' || text.length > MAX_TEXT) { this.clear(); return null; }
      if (this.revision && text === this.text) return null;
      const window = changedWindow(this.text, text);
      const delta = text.length - this.text.length, oldEnd = window.end - delta;
      const retained = this.cache.filter(s => s.end <= window.start || s.start >= oldEnd).map(s => {
        const shift = s.start >= oldEnd ? delta : 0;
        return {...s, start:s.start+shift, end:s.end+shift, revision:this.revision+1};
      }).filter(s=>text.slice(s.start,s.end)===s.source);
      this.text = text; this.revision++; this.cache = retained;
      return { epoch: this.epoch, revision: this.revision, ...window };
    }
    // The whole field at the current revision: re-analysis after a discarded or
    // failed reply, or after the suggestions expired.
    full() { return this.active && this.revision ? { epoch: this.epoch, revision: this.revision, start: 0, end: this.text.length, text: this.text } : null; }
    publish(request, suggestions, now = Date.now()) {
      if (!this.active || request.epoch !== this.epoch || request.revision !== this.revision ||
          request.text !== this.text.slice(request.start, request.end) || !Array.isArray(suggestions)) return false;
      const accepted = [];
      for (const s of suggestions.slice(0, MAX_SUGGESTIONS)) {
        if (!Number.isInteger(s.start) || !Number.isInteger(s.end) || s.start < 0 || s.end < s.start ||
            s.end > request.text.length || !boundary(request.text, s.start) || !boundary(request.text, s.end) ||
            typeof s.replacement !== 'string' || s.replacement.length > MAX_TEXT || typeof s.rule !== 'string' || s.rule.length > 128) continue;
        const start = request.start + s.start, end = request.start + s.end;
        const source = this.text.slice(start, end), key = s.rule + '\0' + source;
        if (this.ignored.has(key) || accepted.some(a => start < a.end && end > a.start)) continue;
        accepted.push({ start, end, source, replacement: s.replacement, rule: s.rule,
          message: typeof s.message === 'string' ? s.message.slice(0, 256) : s.rule,
          epoch: this.epoch, revision: this.revision, expires: now + TTL });
      }
      // The reply is authoritative for the analyzed window: it replaces what was there.
      this.cache = [...this.cache.filter(s => now < s.expires && (s.end <= request.start || s.start >= request.end)), ...accepted]
        .sort((a,b)=>a.start-b.start).slice(0,MAX_SUGGESTIONS); return true;
    }
    suggestion(index, now = Date.now()) {
      const s = this.cache[index];
      return this.active && s && s.epoch === this.epoch && s.revision === this.revision && now < s.expires ? s : null;
    }
    replacement(index, current, edited, now = Date.now()) {
      const s = this.suggestion(index, now);
      if (!s || current !== this.text || current.slice(s.start, s.end) !== s.source ||
          typeof edited !== 'string' || edited.length > MAX_TEXT) return null;
      const next = current.slice(0, s.start) + edited + current.slice(s.end);
      return next.length <= MAX_TEXT ? { ...s, next, replacement: edited } : null;
    }
    dismiss(index, ignore = false) {
      const s = this.suggestion(index);
      if (s && ignore && this.ignored.size < MAX_SUGGESTIONS) this.ignored.add(s.rule + '\0' + s.source);
      this.cache.splice(index, 1);
    }
    clear() { this.active = false; this.text = ''; this.cache = []; this.ignored.clear(); this.revision++; }
  }
  globalThis.GrammarCore = { DocumentSession, changedWindow, boundary, MAX_TEXT, MAX_SUGGESTIONS, TTL };
})();
