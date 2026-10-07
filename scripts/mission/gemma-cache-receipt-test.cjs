const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
// Synthetic receipt only. Executes production cache logic; no browser mutation claim.
const receipt = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const sandbox = {};
vm.runInNewContext(fs.readFileSync('adapters/chromium/extension/core.js', 'utf8'), sandbox);
const { DocumentSession } = sandbox.GrammarCore;
const sources = ['She have two books.', 'The results is uncertain, so do not approve the release.', '이 결과는 검토가 더 필요하므로 지금 결론을 내리면 안됍니다.'];
assert.equal(receipt.scope, 'REAL_RUST_ENGINE_SYNTHETIC_NOT_INSTALLED');
assert.equal(receipt.calls.length, sources.length);
let checks = 0;
for (let i = 0; i < sources.length; i++) {
  const source = sources[i], text = receipt.calls[i].replacement;
  assert.equal(typeof text, 'string');
  assert(!text.startsWith('"') && !text.endsWith('"'));
  const session = new DocumentSession('owned-synthetic');
  const request = session.update(source);
  const suggestions = [{start:0, end:source.length, replacement:text, rule:'gemma:grammar'}];
  assert(session.publish(request, suggestions, 100)); checks++;
  const cached = session.suggestion(0, 101);
  assert.equal(cached.replacement, text); checks++;
  assert.equal(session.replacement(0, source, cached.replacement, 101).next, text); checks++;
  assert.equal(session.replacement(0, source + ' changed', text, 101), null); checks++;
  assert.equal(session.publish({...request, epoch:'other'}, suggestions, 101), false); checks++;
  session.update(source + ' changed');
  assert.equal(session.publish(request, suggestions, 101), false); checks++;
  session.clear();
  assert.equal(session.suggestion(0, 101), null); checks++;
}
console.log(JSON.stringify({scope:'REAL_GEMMA_RECEIPT_PRODUCTION_CACHE_NO_UI',checks,status:'PASS'}));
