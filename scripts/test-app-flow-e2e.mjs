// Built-app user flow on an owned GitHub-hosted Windows desktop:
// select -> global shortcut -> local Instant -> edit -> explicit Copy,
// plus cancel, reselect and a stale session that must not touch a new document.
// Grammar never changes text inside another application (review findings R1/R2;
// the separate Apply button was merged into Copy): after every Copy the
// document is reread unchanged, the result is on the clipboard and the editor's
// counters show no state-changing message from another thread.
// The editor is the task-owned synthetic native Edit harness; no Provider,
// account, user document or real profile is used. The primary shortcut is a
// toggle (documented widget behavior): pressed while the widget is visible it
// hides the widget and cancels the capture, so every capture starts hidden.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {spawn, execFileSync} from 'node:child_process';
import {createInterface} from 'node:readline';
import {mkdir, writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
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
assert.ok(process.env.GRAMMAR_APP_EXE, 'GRAMMAR_APP_EXE names the built app under test');
const appExe = resolve(process.env.GRAMMAR_APP_EXE);
const harnessScript = resolve('src-tauri/tests/native_edit_harness.ps1');
const cdpPort = 9361;
const out = join(evidence, 'app-flow');
await mkdir(out, {recursive: true});
// Deep runs only through the task-owned synthetic executable in place of the
// Claude CLI (no Provider, account or network). Its behaviour per step comes
// from a task-owned mode file.
const syntheticProvider = resolve(evidence, 'runtime/child.exe');
assert.ok(existsSync(syntheticProvider), 'the synthetic Provider executable was built');
const providerModeFile = join(out, 'provider-mode.txt');
const setProviderMode = (mode) => writeFile(providerModeFile, mode);
await setProviderMode('success');

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
  async select(index, start, end) {
    await this.command(`SELECT ${index} ${start} ${end}`);
    assert.equal(await this.command(`SEL ${index}`), `SEL ${start} ${end}`);
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
  async counts(index) {
    const reply = await this.command(`COUNTS ${index}`);
    const values = reply.split(' ').slice(1).map(Number);
    assert.equal(values.length, 13, reply);
    return values;
  }
  async resetCounts() {
    assert.equal(await this.command('RESET'), 'OK');
  }
}

// Synthetic editor counters: 0-2 are reads; 3-9 are state-changing messages sent
// from another thread (EM_REPLACESEL, EM_SETREADONLY lock/unlock, WM_SETTEXT,
// EM_UNDO, EM_SETSEL, EM_SETMODIFY); 10 counts change notifications they raised
// and 12 every change notification from any source (posted messages or input).
// Text and selection are also reread, so a change the counters miss still fails.
const STATE_CHANGING_KINDS = [3, 4, 5, 6, 7, 8, 9, 10, 12];
const EDITORS = [0, 5];
const editorMessages = [];

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
let page;
let widgetState = null;
// Receipts never carry text values: the first line of a failure, quoted values removed.
const scrub = (error) => String(error?.message ?? error).split('\n')[0].replace(/(['"`]).*?\1/g, '<value>').slice(0, 300);
async function editorSnapshot() {
  const state = [];
  for (const index of EDITORS) state.push({text: await editor.text(index), selection: await editor.command(`SEL ${index}`)});
  return state;
}
// Resets the counters and returns the text and selection of both editors.
async function beginUntouched() {
  await editor.resetCounts();
  return editorSnapshot();
}
async function assertEditorsUntouched(step, before) {
  assert.ok(JSON.stringify(await editorSnapshot()) === JSON.stringify(before), `${step}: editor text or selection changed`);
  for (const index of EDITORS) {
    const values = await editor.counts(index);
    const stateChanging = STATE_CHANGING_KINDS.reduce((sum, kind) => sum + values[kind], 0);
    editorMessages.push({step, editor: index, reads: values[0] + values[1] + values[2], state_changing: stateChanging});
    for (const kind of STATE_CHANGING_KINDS) {
      assert.equal(values[kind], 0, `${step}: editor ${index} received state-changing kind ${kind} (${values.join(' ')})`);
    }
  }
}
try {
  await editor.ready();
  pass('owned synthetic native editor ready');

  // Starts the built app with a WebView2 debugging port and connects to it.
  async function launchApp() {
    app = spawn(appExe, [], {
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
        CODEX_PENCIL_CLAUDE_BIN: syntheticProvider,
        P01_FIXTURE_MODE_FILE: providerModeFile,
      },
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
    // The debugging endpoint answers before the widget document commits, so wait
    // for the app origin itself and for Tauri's injected IPC bridge.
    const appPage = (candidate) => /^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)\//.test(candidate.url());
    page = await until('widget page', async () =>
      browser.contexts().flatMap((context) => context.pages()).find(appPage), 60000);
    await waitForBridge();
  }
  // Polling is by interval, not animation frames, because the widget may be hidden.
  async function waitForBridge() {
    await page.waitForFunction(() => typeof window.__TAURI_INTERNALS__?.transformCallback === 'function', null, {
      polling: 100,
      timeout: 60000,
    });
    // Record capture tokens exactly as the app's own listener receives them.
    await page.evaluate(() => {
      window.__grammarCaptures = [];
      const internals = window.__TAURI_INTERNALS__;
      const handler = internals.transformCallback((event) => window.__grammarCaptures.push(event.payload));
      return internals.invoke('plugin:event|listen', {event: 'selection-captured', target: {kind: 'Any'}, handler});
    });
  }
  await launchApp();
  pass('built app started with its widget WebView and IPC bridge');

  const captures = () => page.evaluate(() => window.__grammarCaptures.slice());
  const widgetVisible = () =>
    page.evaluate(() => window.__TAURI_INTERNALS__.invoke('plugin:window|is_visible', {label: 'main'}));

  const closeButton = page.getByRole('button', {name: 'Close', exact: true});
  async function closeWidget() {
    await closeButton.click();
    await until('widget hidden', async () => !(await widgetVisible()));
  }

  // A fresh profile shows first-run onboarding in the visible widget. The stored
  // settings (read-only command) decide which pane the widget must settle on.
  const loadSettings = () => page.evaluate(() => window.__TAURI_INTERNALS__.invoke('load_settings'));
  const onboarding = page.getByRole('button', {name: 'Continue', exact: true});
  if ((await loadSettings()).onboardingVersion < 1) {
    await until('onboarding shown', async () => (await onboarding.count()) > 0 && (await widgetVisible()), 30000);
    await onboarding.click();
    await until('onboarding closed', async () => (await onboarding.count()) === 0);
    assert.ok((await loadSettings()).onboardingVersion >= 1, 'onboarding completion persisted');
  }
  const draft = page.getByLabel('Editable rewrite result text', {exact: true});
  const status = page.locator('.status-pill');
  const copy = page.getByTestId('copy-result');
  const review = page.getByRole('dialog', {name: 'AI 클라우드 처리 안내'});
  const errorRow = page.locator('.error-row');
  const draftValue = () => draft.inputValue({timeout: 2000}).catch(() => '');
  // Without a capture the result pane is not rendered while Codex is signed out.
  const copyEnabled = async () => (await copy.count()) > 0 && (await copy.isEnabled());

  // Idle, the widget shows the main pane, or the Codex sign-in pane when Codex is
  // not signed in (as on this runner). A capture always opens the main pane, so
  // local Instant and Copy never depend on a cloud account.
  const signIn = page.getByRole('heading', {name: 'Sign in with ChatGPT', exact: true});
  const idlePane = await until('idle widget pane rendered', async () =>
    ((await copy.count()) > 0 && 'main') || ((await signIn.count()) > 0 && 'codex-sign-in'), 30000);
  if (await widgetVisible()) await closeWidget();
  pass(`first-run onboarding completed and the widget closed to the tray (idle pane: ${idlePane})`);

  async function prepareSelection(index, text, selected) {
    await editor.set(index, text);
    const [start, end] = range(text, selected);
    await editor.select(index, start, end);
    await editor.focus(index);
  }

  async function captureAndWaitInstant(index, text, selected, expectedCandidate) {
    assert.equal(await widgetVisible(), false, 'each capture starts with the widget hidden');
    const before = (await captures()).length;
    await prepareSelection(index, text, selected);
    await editor.hotkey();
    const token = await until('selection captured', async () => (await captures())[before], 15000);
    assert.equal(token.selectedText, selected);
    await until('widget shown for the capture', widgetVisible);
    await until('Instant candidate', async () => (await draftValue()) === expectedCandidate, 15000);
    return token;
  }

  // Auto rewrite is on by default and cloud consent was never given: each capture
  // offers the Deep review beside the local Instant draft. Declining sends nothing.
  async function declineDeep() {
    await until('Deep consent review shown beside Instant', async () => (await review.count()) === 1);
    await review.getByRole('button', {name: '취소', exact: true}).click();
    await until('Deep declined', async () =>
      (await review.count()) === 0 && (await status.textContent()) === 'Deep not sent · Instant stays local');
  }

  // Copy on a captured native editor: the result goes to the clipboard, the
  // widget stays open with the draft, and the editor gets no state-changing
  // message. The normal case shows no warning; Copy stays disabled until the
  // next capture.
  async function copyResult(step, expectedClipboard, expectedDraft = expectedClipboard) {
    await until('Copy enabled', copyEnabled);
    const before = await beginUntouched();
    await copy.click();
    await until('Copy outcome', async () => (await status.textContent()) === 'Copied — paste it into the field');
    assert.equal(await errorRow.count(), 0, 'a normal Copy shows no warning');
    await until('result on the clipboard', async () => (await editor.clipboard()) === expectedClipboard);
    assert.equal(await widgetVisible(), true, 'the widget stays open after Copy');
    assert.equal(await draftValue(), expectedDraft, 'the copied draft stays visible');
    assert.equal(await copyEnabled(), false, 'the capture ended with the Copy');
    await assertEditorsUntouched(step, before);
  }

  // 1. Instant-only draft, edited by the user, then copied.
  const doc = 'Line one stays.\r\nPlease seperate these items.\r\nLine three stays.';
  const token1 = await captureAndWaitInstant(0, doc, 'Please seperate these items.', 'Please separate these items.');
  await declineDeep();
  assert.equal(await draftValue(), 'Please separate these items.');
  pass('shortcut captured the exact selection; local Instant is usable without cloud consent and declining Deep sends nothing');
  await draft.fill('Please separate these items carefully.');
  await copyResult('copy-edited-draft', 'Please separate these items carefully.');
  assert.equal(await editor.text(0), doc);
  await closeWidget();
  pass('Copy of the edited draft: exact draft on the clipboard, document reread unchanged, no state-changing editor message');

  // 2. Copy of the unedited Instant draft: the same single button, and a second
  //    Copy of the ended capture is not possible. Dismiss still closes the
  //    copied draft.
  const copyDoc = 'Copy source line.\r\nthe results is final.';
  await captureAndWaitInstant(0, copyDoc, 'the results is final.', 'the results are final.');
  await declineDeep();
  await copyResult('copy-instant-draft', 'the results are final.');
  assert.equal(await editor.text(0), copyDoc);
  assert.equal(await copy.isDisabled(), true, 'Copy stays disabled until the next capture');
  const dismissResult = page.getByTestId('dismiss-result');
  assert.equal(await dismissResult.isEnabled(), true, 'Dismiss stays available for the copied draft');
  await dismissResult.click();
  await until('widget hidden by Dismiss', async () => !(await widgetVisible()));
  pass('Copy of the unedited Instant draft wrote only the clipboard, never changed the document and ended the capture; Dismiss closed the copied draft');

  // 3. Cancel: dismissing the capture invalidates it; its token cannot Copy later.
  const cancelDoc = 'Cancel check: please seperate nothing here.';
  const cancelToken = await captureAndWaitInstant(0, cancelDoc, 'please seperate nothing', 'please separate nothing');
  await declineDeep();
  await closeWidget();
  await until('capture dismissed', async () => !(await copyEnabled()));
  // The Copy button's backend command (its IPC name predates the merge).
  const invokeCopy = (token, replacement) => page.evaluate(({token, replacement}) =>
    window.__TAURI_INTERNALS__.invoke('apply_replacement', {
      sessionId: token.sessionId,
      generation: token.generation,
      replacement,
      mode: 'grammar',
      targetLanguage: null,
      autoReferenceLanguage: null,
      instantDraft: null,
    }), {token, replacement});
  const beforeCancelled = await beginUntouched();
  const clipboardBeforeCancelled = await editor.clipboard();
  assert.deepEqual(await invokeCopy(cancelToken, 'CANCELLED-SHOULD-NOT-COPY'), {
    status: 'failed',
    reason: 'invalid_session_state',
  });
  assert.equal(await editor.text(0), cancelDoc);
  assert.equal(await editor.clipboard(), clipboardBeforeCancelled, 'a rejected token never writes the clipboard');
  await assertEditorsUntouched('cancelled-token', beforeCancelled);
  pass('cancel invalidated the capture and its token was rejected without a clipboard write or mutation');

  // 4. Reselect: the user selects another range while the widget is open. The
  //    first shortcut press closes the widget and cancels the old capture, the
  //    second captures the new range; Copy delivers only the new range's result.
  const reselectDoc = 'First: please seperate this.\r\nSecond: this are wrong.';
  const staleToken = await captureAndWaitInstant(0, reselectDoc, 'please seperate this.', 'please separate this.');
  const beforeToggle = (await captures()).length;
  const [secondStart, secondEnd] = range(reselectDoc, 'this are wrong.');
  await editor.select(0, secondStart, secondEnd);
  await editor.focus(0);
  await editor.hotkey();
  await until('shortcut closed the open widget', async () => !(await widgetVisible()));
  assert.equal((await captures()).length, beforeToggle, 'closing press captures nothing');
  const clipboardBeforeClosed = await editor.clipboard();
  assert.deepEqual(await invokeCopy(staleToken, 'CLOSED-SHOULD-NOT-COPY'), {
    status: 'failed',
    reason: 'invalid_session_state',
  });
  await sleep(300);
  const reselectToken = await captureAndWaitInstant(0, reselectDoc, 'this are wrong.', 'this is wrong.');
  assert.deepEqual(await invokeCopy(staleToken, 'STALE-SHOULD-NOT-COPY'), {status: 'rejected_stale'});
  assert.equal(await editor.text(0), reselectDoc);
  assert.equal(await editor.clipboard(), clipboardBeforeClosed, 'closed and stale tokens never write the clipboard');
  await declineDeep();
  await copyResult('copy-reselected', 'this is wrong.');
  assert.equal(await editor.text(0), reselectDoc);
  await closeWidget();
  assert.notEqual(reselectToken.sessionId, staleToken.sessionId);
  pass('shortcut toggle closed and cancelled the old capture; reselection bound Copy to the new range only');

  // 5. A stale session never changes a newer document in another window.
  const otherDoc = 'Other window: please seperate me.';
  const otherToken = await captureAndWaitInstant(5, otherDoc, 'please seperate me.', 'please separate me.');
  await declineDeep();
  const beforeStale = await beginUntouched();
  const clipboardBeforeStale = await editor.clipboard();
  for (const token of [staleToken, token1, reselectToken]) {
    assert.deepEqual(await invokeCopy(token, 'STALE-SHOULD-NOT-COPY'), {status: 'rejected_stale'});
  }
  assert.equal(await editor.text(5), otherDoc);
  assert.equal(await editor.text(0), reselectDoc);
  assert.equal(await editor.clipboard(), clipboardBeforeStale, 'stale tokens never write the clipboard');
  await assertEditorsUntouched('stale-tokens', beforeStale);
  await copyResult('copy-other-window', 'please separate me.');
  assert.equal(await editor.text(5), otherDoc);
  assert.equal(await editor.text(0), reselectDoc);
  assert.notEqual(otherToken.sessionId, staleToken.sessionId);
  pass('stale sessions were rejected without a clipboard write and the current capture was copied; neither document changed');

  await closeWidget();

  // Steps 6-13 use Deep through the synthetic executable. The Provider choice
  // and consent are saved the way the consent dialog saves them; the widget
  // reloads to pick them up.
  const saved = await loadSettings();
  await page.evaluate((settings) => window.__TAURI_INTERNALS__.invoke('save_settings', {settings}), {
    ...saved,
    activeProvider: 'claude',
    claudeCloudAcknowledgementVersion: 1,
    translation: {...saved.translation, targetLanguage: 'en', applyFormat: 'source_with_translation'},
  });
  await page.reload();
  await waitForBridge();
  const cancelDeep = page.getByTestId('cancel-rewrite');
  const runDeep = page.getByTestId('run-deep');
  const cancelDeepIfRunning = async () => {
    await sleep(300);
    if ((await cancelDeep.count()) > 0 && (await cancelDeep.isEnabled())) {
      await cancelDeep.click();
      await until('Deep cancelled', async () => (await status.textContent()) === 'Deep cancelled · the draft is still available');
    }
  };
  const errorText = async () => ((await errorRow.count()) > 0 ? (await errorRow.textContent()) ?? '' : '');

  // 6. A Deep result that arrives after the Instant draft never replaces it;
  //    switching to Translate shows the new Deep translation, and Copy joins
  //    the exact captured source and the translation locally.
  await setProviderMode('slow-success');
  const translateDoc = 'Translate me: please seperate this line.';
  await captureAndWaitInstant(0, translateDoc, 'please seperate this line.', 'please separate this line.');
  await until('the Deep result of the capture arrived', async () => (await status.textContent()) === 'Replacement ready', 20000);
  assert.equal(await draftValue(), 'please separate this line.', 'a later Deep result never replaces the shown draft');
  await page.getByRole('button', {name: 'Translate', exact: true}).click();
  await until('Deep translation shown after the mode change', async () => (await draftValue()) === 'Synthetic.', 20000);
  await copyResult('copy-translation-with-source', 'please seperate this line. (Synthetic.)', 'Synthetic.');
  assert.equal(await editor.text(0), translateDoc);
  await page.getByRole('button', {name: 'Grammar', exact: true}).click();
  await closeWidget();
  pass('after a mode change the new Deep translation appeared; Copy joined source and translation; the document never changed');

  // 7. Cancel stops only Deep: no error, the draft stays and can be copied,
  //    even after Deep is started again at once.
  await setProviderMode('slow');
  const cancelDeepDoc = 'Cancel Deep: this are wrong.';
  await captureAndWaitInstant(0, cancelDeepDoc, 'this are wrong.', 'this is wrong.');
  await until('Deep running', async () => (await cancelDeep.count()) > 0 && (await cancelDeep.isEnabled()));
  await cancelDeep.click();
  await until('Deep cancelled', async () => (await status.textContent()) === 'Deep cancelled · the draft is still available');
  await sleep(1000);
  assert.equal(await errorText(), '', 'Cancel shows no error and no raw code');
  assert.equal(await draftValue(), 'this is wrong.');
  // An earlier request may still be finishing: either Deep starts, or it says
  // so. The capture must stay usable either way.
  if (await runDeep.isEnabled()) {
    await runDeep.click();
    await cancelDeepIfRunning();
  }
  assert.doesNotMatch(await errorText(), /invalid_session_state|no longer active/);
  await copyResult('copy-after-cancel', 'this is wrong.');
  await closeWidget();
  pass('Cancel stopped only Deep; the Instant draft stayed and was copied');

  // 8. The shortcut toggle ends the capture in the backend and tells the widget.
  const hideDoc = 'Hide check: please seperate here.';
  const hideToken = await captureAndWaitInstant(0, hideDoc, 'please seperate here.', 'please separate here.');
  await cancelDeepIfRunning();
  await editor.hotkey();
  await until('shortcut hid the widget', async () => !(await widgetVisible()));
  await until('widget told the capture ended', async () => (await status.textContent()) === 'Capture closed');
  assert.equal(await draftValue(), '', 'the ended capture left no draft behind');
  assert.equal(await copyEnabled(), false);
  assert.deepEqual(await invokeCopy(hideToken, 'ENDED-SHOULD-NOT-COPY'), {status: 'failed', reason: 'invalid_session_state'});
  pass('the shortcut toggle ended the capture and the widget no longer offered Copy for it');

  // 9. Another program holds the clipboard: Copy fails without changing
  //    anything, says so, and a retry after the clipboard is free copies once.
  const lockDoc = 'Lock check: please seperate it.';
  await captureAndWaitInstant(0, lockDoc, 'please seperate it.', 'please separate it.');
  await cancelDeepIfRunning();
  const beforeLock = await beginUntouched();
  assert.equal(await editor.command('CLIPLOCK'), 'OK');
  try {
    await copy.click();
    await until('Copy failed while the clipboard was held', async () => (await status.textContent()) === 'Copy failed');
    assert.match(await errorText(), /clipboard could not be written/);
    assert.equal(await copyEnabled(), true, 'Copy stays available for a retry');
  } finally {
    assert.equal(await editor.command('CLIPUNLOCK'), 'OK');
  }
  await assertEditorsUntouched('clipboard-held', beforeLock);
  await copyResult('copy-after-clipboard-held', 'please separate it.');
  await closeWidget();
  pass('a held clipboard failed Copy safely with a clear message; the retry copied once');

  // 10. Korean, emoji and CRLF: the capture is exact, and an edited Unicode
  //     draft is copied exactly.
  const unicodeDoc = '첫 줄은 그대로.\r\n둘째 줄 😀 please seperate this.\r\n셋째 줄.';
  const unicodeSelection = '둘째 줄 😀 please seperate this.\r\n셋째';
  assert.equal(await widgetVisible(), false, 'each capture starts with the widget hidden');
  const beforeUnicode = (await captures()).length;
  await prepareSelection(0, unicodeDoc, unicodeSelection);
  await editor.hotkey();
  const unicodeToken = await until('Unicode selection captured', async () => (await captures())[beforeUnicode], 15000);
  assert.equal(unicodeToken.selectedText, unicodeSelection, 'the capture is exact');
  await until('widget shown for the Unicode capture', widgetVisible);
  await until('a local Instant draft to edit', async () => (await draftValue()).length > 0, 15000);
  await cancelDeepIfRunning();
  const unicodeDraft = '한국어 초안을 고쳤습니다 ✍️ 확인 😀';
  await draft.fill(unicodeDraft);
  await copyResult('copy-unicode-draft', unicodeDraft);
  assert.equal(await editor.text(0), unicodeDoc);
  await closeWidget();
  pass('Korean, emoji and CRLF were captured exactly and an edited Unicode draft was copied exactly');

  // 11. A password field is refused before any text is read.
  const beforePassword = (await captures()).length;
  await editor.set(1, 'synthetic-secret-value');
  await editor.select(1, 0, 9);
  await editor.focus(1);
  await editor.resetCounts();
  await editor.hotkey();
  await until('password field refused', async () => /Password and credential fields are never read/.test(await errorText()));
  assert.doesNotMatch(await errorText(), /native_sensitive_editor/, 'no raw code');
  assert.equal((await captures()).length, beforePassword, 'no capture from a password field');
  const passwordCounts = await editor.counts(1);
  assert.equal(passwordCounts[0] + passwordCounts[1], 0, 'no text or length was read from the password field');
  if (await widgetVisible()) await closeWidget();

  // 12. No selection: a clear message and no capture.
  await editor.set(0, 'Nothing is selected here.');
  await editor.select(0, 3, 3);
  await editor.focus(0);
  await editor.hotkey();
  await until('empty selection reported', async () => /No text selected/.test(await errorText()));
  assert.equal((await captures()).length, beforePassword, 'no capture without a selection');
  if (await widgetVisible()) await closeWidget();
  pass('a password field and an empty selection were refused with a clear message and no capture');

  // 13. Restart: settings persist; no capture, draft or selection comes back.
  const settingsBeforeRestart = await loadSettings();
  await browser.close();
  killTree(app.pid);
  await until('app exited for the restart', async () => !alive(app.pid), 20000);
  await launchApp();
  // Locators belong to a page; the restarted app has a new one.
  const restartedCopy = page.getByTestId('copy-result');
  const restartedDraft = page.getByLabel('Editable rewrite result text', {exact: true});
  await until('widget pane rendered after the restart', async () =>
    (await restartedCopy.count()) > 0 || (await page.getByRole('heading', {name: 'Sign in with ChatGPT', exact: true}).count()) > 0, 30000);
  assert.equal(await page.getByRole('button', {name: 'Continue', exact: true}).count(), 0, 'onboarding does not return');
  assert.deepEqual(await loadSettings(), settingsBeforeRestart, 'settings persisted across the restart');
  assert.equal(await restartedDraft.inputValue({timeout: 2000}).catch(() => ''), '', 'no draft after the restart');
  assert.equal((await restartedCopy.count()) > 0 && (await restartedCopy.isEnabled()), false, 'nothing to copy after the restart');
  assert.deepEqual(await invokeCopy(unicodeToken, 'OLD-SHOULD-NOT-COPY'), {status: 'rejected_stale'});
  pass('after a restart no capture, draft or selection came back and settings persisted');
} catch (error) {
  failure = error;
  console.error('FAIL', error?.stack ?? error);
  // App-generated status and error strings only (the documents are synthetic).
  widgetState = await page
    ?.evaluate(() => ({
      status: document.querySelector('.status-pill')?.textContent ?? null,
      error: document.querySelector('.error-row')?.textContent ?? null,
    }))
    .catch(() => null);
  if (widgetState) console.error('WIDGET_STATE', JSON.stringify(widgetState));
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
    app_kind: process.env.GRAMMAR_APP_KIND ?? 'unspecified',
    result_delivery: 'COPY_ONLY',
    checks,
    // Message counts only (no text): reads and state-changing messages per step.
    editor_messages: editorMessages,
    failure: failure ? scrub(failure) : null,
    widget_state: widgetState,
    not_qualified: ['physical keyboard and IME', 'live Provider Deep', 'user profiles and real documents'],
  };
  await writeFile(join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  process.exitCode = failure ? 1 : 0;
}
