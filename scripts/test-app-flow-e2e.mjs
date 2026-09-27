// Built-app user flow on an owned GitHub-hosted Windows desktop:
// select -> global shortcut -> local Instant -> edit -> explicit Apply/Copy,
// plus cancel, reselect and a stale session that must not touch a new document.
// The editor is the task-owned synthetic native Edit harness; no Provider,
// account, user document or real profile is used.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {spawn, execFileSync} from 'node:child_process';
import {createInterface} from 'node:readline';
import {mkdir, writeFile} from 'node:fs/promises';
import {resolve, join} from 'node:path';

const OWNED_REFS = ['refs/heads/codex/grammar-autonomous-r1', 'refs/heads/claude/eloquent-faraday-hh62qc'];
for (const [key, expected] of [
  ['GITHUB_ACTIONS', 'true'],
  ['GITHUB_REPOSITORY', 'CAPTW/pencil'],
  ['RUNNER_ENVIRONMENT', 'github-hosted'],
  ['GRAMMAR_OWNED_DESKTOP_TEST', '1'],
]) {
  assert.equal(process.env[key], expected, `owned desktop opt-in: ${key}`);
}
assert.ok(OWNED_REFS.includes(process.env.GITHUB_REF), 'owned desktop opt-in: GITHUB_REF');

const require = createRequire(import.meta.url);
const {chromium} = require(process.env.GRAMMAR_PLAYWRIGHT);
const evidence = resolve(process.env.GRAMMAR_EVIDENCE);
const appExe = resolve(process.env.GRAMMAR_APP_EXE);
const harnessScript = resolve('src-tauri/tests/native_edit_harness.ps1');
const cdpPort = 9361;
const out = join(evidence, 'app-flow');
await mkdir(out, {recursive: true});

const checks = [];
let failure = null;
const pass = (name) => {
  checks.push(name);
  console.log('PASS', name);
};
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
async function until(label, probe, timeout = 15000) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    last = await probe();
    if (last) return last;
    await sleep(100);
  }
  throw new Error(`timeout: ${label} (last=${JSON.stringify(last)})`);
}
const b64 = (text) => Buffer.from(text, 'utf8').toString('base64');
const unb64 = (text) => Buffer.from(text ?? '', 'base64').toString('utf8');
const units = (text) => Buffer.from(text, 'utf16le').length / 2;
const range = (haystack, needle) => {
  const start = units(haystack.slice(0, haystack.indexOf(needle)));
  return [start, start + units(needle)];
};

class Editor {
  constructor() {
    this.process = spawn('powershell.exe', ['-NoProfile', '-STA', '-File', harnessScript], {
      stdio: ['pipe', 'pipe', 'ignore'],
    });
    this.replies = [];
    this.waiters = [];
    createInterface({input: this.process.stdout}).on('line', (line) => {
      const waiter = this.waiters.shift();
      if (waiter) waiter(line);
      else this.replies.push(line);
    });
  }
  next(timeout = 60000) {
    const ready = this.replies.shift();
    if (ready !== undefined) return Promise.resolve(ready);
    return new Promise((done, fail) => {
      const timer = setTimeout(() => fail(new Error('synthetic editor reply timeout')), timeout);
      this.waiters.push((line) => {
        clearTimeout(timer);
        done(line);
      });
    });
  }
  async ready() {
    const line = await this.next();
    assert.ok(line.startsWith('NATIVE_READY:'), line);
  }
  async command(line) {
    this.process.stdin.write(`${line}\n`);
    const reply = await this.next(10000);
    assert.ok(!reply.startsWith('ERR'), `${line.split(' ')[0]}: ${reply}`);
    return reply;
  }
  set(index, text) {
    return this.command(`SET ${index} ${b64(text)}`);
  }
  select(index, start, end) {
    return this.command(`SELECT ${index} ${start} ${end}`);
  }
  async text(index) {
    return unb64((await this.command(`TEXT ${index}`)).split(' ')[1]);
  }
  async clipboard() {
    return unb64((await this.command('CLIPBOARD')).split(' ')[1]);
  }
  async focus(index) {
    const want = index === 5 ? 'other' : 'main';
    await until(`editor ${index} foreground`, async () => {
      await this.command(`FOCUS ${index}`);
      return (await this.command('FOREGROUND')) === `FOREGROUND ${want}`;
    }, 10000);
  }
  async hotkey() {
    assert.equal(await this.command('HOTKEY'), 'SENT 6');
  }
}

function killTree(pid) {
  try {
    execFileSync('taskkill.exe', ['/T', '/F', '/PID', String(pid)], {stdio: 'ignore'});
  } catch {
    // Already exited.
  }
}

function alive(pid) {
  const listing = execFileSync('tasklist.exe', ['/FI', `PID eq ${pid}`, '/FO', 'CSV', '/NH'], {encoding: 'utf8'});
  return listing.includes(`"${pid}"`);
}

const editor = new Editor();
let app;
let browser;
try {
  await editor.ready();
  pass('owned synthetic native editor ready');

  app = spawn(appExe, [], {
    env: {...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`},
    stdio: 'ignore',
  });
  await until('WebView2 debugging endpoint', async () => {
    try {
      return (await fetch(`http://127.0.0.1:${cdpPort}/json/version`)).ok;
    } catch {
      return false;
    }
  }, 90000);
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${cdpPort}`);
  const page = await until('widget page', async () => browser.contexts()[0]?.pages()[0], 30000);
  await page.waitForLoadState('domcontentloaded');
  pass('built app started with its widget WebView');

  // Record capture tokens exactly as the app's own listener receives them.
  await page.evaluate(() => {
    window.__grammarCaptures = [];
    const internals = window.__TAURI_INTERNALS__;
    const handler = internals.transformCallback((event) => window.__grammarCaptures.push(event.payload));
    return internals.invoke('plugin:event|listen', {event: 'selection-captured', target: {kind: 'Any'}, handler});
  });
  const captures = () => page.evaluate(() => window.__grammarCaptures.slice());

  const onboarding = page.getByRole('button', {name: 'Continue'});
  if (await onboarding.count()) {
    await onboarding.click();
    await until('onboarding closed', async () => (await onboarding.count()) === 0);
  }
  pass('first-run onboarding completed');

  const draft = page.getByLabel('Editable rewrite result text');
  const status = page.locator('.status-pill');
  const apply = page.getByTestId('apply-result');
  const copy = page.getByTestId('copy-result');
  const draftValue = () => draft.inputValue().catch(() => '');

  async function captureAndWaitInstant(index, text, selected, expectedCandidate) {
    const before = (await captures()).length;
    await editor.set(index, text);
    const [start, end] = range(text, selected);
    await editor.select(index, start, end);
    await editor.focus(index);
    await editor.hotkey();
    const token = await until('selection captured', async () => (await captures())[before], 15000);
    assert.equal(token.selectedText, selected);
    await until('Instant candidate', async () => (await draftValue()) === expectedCandidate, 15000);
    return token;
  }

  // 1. Instant-only draft, edited by the user, applied on the supported path.
  const doc = 'Line one stays.\r\nPlease seperate these items.\r\nLine three stays.';
  const token1 = await captureAndWaitInstant(0, doc, 'Please seperate these items.', 'Please separate these items.');
  // Auto rewrite is on by default: consent is requested for Deep only, while the
  // local Instant draft is already usable. Declining sends nothing.
  const review = page.getByRole('dialog', {name: 'AI 클라우드 처리 안내'});
  await until('Deep consent review shown beside Instant', async () => (await review.count()) === 1);
  await review.getByRole('button', {name: '취소'}).click();
  await until('Deep declined', async () => (await status.textContent()) === 'Deep not sent · Instant stays local');
  assert.equal(await draftValue(), 'Please separate these items.');
  pass('shortcut captured the exact selection; local Instant is usable without cloud consent and declining Deep sends nothing');
  const clipboardBefore = await editor.clipboard();
  await draft.fill('Please separate these items carefully.');
  await until('Apply enabled', async () => apply.isEnabled());
  await apply.click();
  await until('document applied', async () => (await editor.text(0)) === doc.replace('Please seperate these items.', 'Please separate these items carefully.'));
  assert.equal(await editor.clipboard(), clipboardBefore);
  pass('edited Instant-only draft applied exactly with document reread and clipboard untouched');

  // 2. Copy leaves the document unchanged and places the draft on the clipboard.
  const copyDoc = 'Copy source line.\r\nthe results is final.';
  await captureAndWaitInstant(0, copyDoc, 'the results is final.', 'the results are final.');
  await copy.click();
  await until('draft copied', async () => (await editor.clipboard()) === 'the results are final.');
  assert.equal(await editor.text(0), copyDoc);
  pass('explicit Copy wrote only the clipboard and never changed the document');

  // 3. Cancel: dismissing the capture invalidates it; its token cannot Apply later.
  const cancelDoc = 'Cancel check: please seperate nothing here.';
  const cancelToken = await captureAndWaitInstant(0, cancelDoc, 'please seperate nothing', 'please separate nothing');
  await page.getByRole('button', {name: 'Close'}).click();
  await until('capture dismissed', async () => !(await apply.isEnabled()));
  const invokeApply = (token, replacement) => page.evaluate(({token, replacement}) =>
    window.__TAURI_INTERNALS__.invoke('apply_replacement', {
      sessionId: token.sessionId,
      generation: token.generation,
      replacement,
      mode: 'grammar',
      targetLanguage: null,
      autoReferenceLanguage: null,
      restoreClipboard: false,
      instantDraft: null,
    }), {token, replacement});
  assert.deepEqual(await invokeApply(cancelToken, 'CANCELLED-SHOULD-NOT-APPLY'), {
    status: 'failed',
    reason: 'invalid_session_state',
  });
  assert.equal(await editor.text(0), cancelDoc);
  pass('cancel invalidated the capture and its token was rejected without mutation');

  // 4. Reselect: a new capture replaces the old one; only the new range changes.
  const reselectDoc = 'First: please seperate this.\r\nSecond: this are wrong.';
  const staleToken = await captureAndWaitInstant(0, reselectDoc, 'please seperate this.', 'please separate this.');
  await captureAndWaitInstant(0, reselectDoc, 'this are wrong.', 'this is wrong.');
  await apply.click();
  const reselected = reselectDoc.replace('this are wrong.', 'this is wrong.');
  await until('reselected range applied', async () => (await editor.text(0)) === reselected);
  pass('reselection bound Apply to the new selection only');

  // 5. A stale session never changes a newer document in another window.
  const otherDoc = 'Other window: please seperate me.';
  const otherToken = await captureAndWaitInstant(5, otherDoc, 'please seperate me.', 'please separate me.');
  for (const token of [staleToken, token1]) {
    assert.deepEqual(await invokeApply(token, 'STALE-SHOULD-NOT-APPLY'), {status: 'rejected_stale'});
  }
  assert.equal(await editor.text(5), otherDoc);
  assert.equal(await editor.text(0), reselected);
  await apply.click();
  await until('new document applied', async () => (await editor.text(5)) === 'Other window: please separate me.');
  assert.equal(await editor.text(0), reselected);
  assert.notEqual(otherToken.sessionId, staleToken.sessionId);
  pass('stale sessions were rejected and only the current document changed');
} catch (error) {
  failure = error;
  console.error('FAIL', error?.stack ?? error);
} finally {
  try {
    await browser?.close();
  } catch {
    // CDP connection already gone.
  }
  const pids = [app?.pid, editor.process.pid].filter(Boolean);
  for (const pid of pids) killTree(pid);
  await sleep(1500);
  const residual = pids.filter(alive);
  if (residual.length) {
    failure ??= new Error(`residual processes: ${residual.join(',')}`);
  } else {
    pass('app and synthetic editor process trees exited');
  }
  const result = {
    status: failure ? 'FAIL' : 'PASS',
    source_sha: process.env.GITHUB_SHA,
    run_id: process.env.GITHUB_RUN_ID,
    classification: 'SYNTHETIC_OWNED_WINDOWS_DESKTOP_BUILT_APP',
    checks,
    failure: failure ? String(failure.message ?? failure).slice(0, 500) : null,
    not_qualified: ['physical keyboard and IME', 'live Provider Deep', 'user profiles and real documents'],
  };
  await writeFile(join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  process.exitCode = failure ? 1 : 0;
}
