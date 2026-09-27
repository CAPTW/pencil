// Installed-package Chromium smoke test on an owned GitHub-hosted Windows desktop.
// browser.test.mjs runs a modified isolated extension fixture (new key/ID, a
// fixture host permission, its own host-config.js and host registration). This
// test instead loads the installed extension directory exactly as the
// installer left it: package key/ID, installed permissions (activeTab, no host
// permission) and the installer-written host-config.js, with the native host
// the installer registered, in a fresh task-owned profile. Page access comes
// only from the real browser toolbar: the Extensions menu item and the popup's
// "Enable this document" button are clicked with OS mouse input after UI
// Automation locates them (ui-gesture.ps1). Worker evaluation is used only to
// observe page access and read storage; it never grants or enables anything.
// If no real gesture is possible, the result is NOT_RUN with the exact cause.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {execFile, execFileSync} from 'node:child_process';
import {promisify} from 'node:util';
import {mkdir, mkdtemp, readFile, readdir, rm, writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {join, resolve} from 'node:path';

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
const installRoot = resolve(process.env.GRAMMAR_INSTALL_ROOT);
const extensionDir = join(installRoot, 'extension');
const gestureScript = resolve('adapters/chromium/tests/ui-gesture.ps1');
const out = join(evidence, 'installed-browser');
await mkdir(out, {recursive: true});
const execFileAsync = promisify(execFile);
const PORT = 18438;
const OTHER_PORT = 18439;
const TITLE = 'Grammar installed-package fixture';
const origin = `http://127.0.0.1:${PORT}`;

const checks = [];
const gestures = [];
const observations = {};
let status = 'PASS';
let notRunCause = null;
let failure = null;
const pass = (name) => {
  checks.push(name);
  console.log('PASS', name);
};
class NotRun extends Error {}
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
async function until(label, probe, timeout = 10000) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    last = await probe();
    if (last) return last;
    await sleep(100);
  }
  throw new Error(`timeout: ${label} (last=${JSON.stringify(last)})`);
}
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

// Content hashes of the installed extension directory (a new or missing entry
// is a change too).
async function extensionState() {
  const entries = await readdir(extensionDir, {withFileTypes: true});
  const state = {};
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    state[entry.name] = entry.isFile() ? sha256(await readFile(join(extensionDir, entry.name))) : 'not-a-file';
  }
  return state;
}

// Count processes running the installed host binary (installer registration).
function installedHostProcesses(hostPath) {
  const script = `@(Get-Process -Name grammar-chromium-host -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq '${hostPath}' }).Count`;
  return Number(execFileSync('powershell.exe', ['-NoProfile', '-Command', script], {encoding: 'utf8'}).trim());
}

async function helper(command, name, {method = 'Mouse', match = 'Exact', timeoutMs = 10000} = {}) {
  const args = ['-NoProfile', '-STA', '-File', gestureScript, '-Command', command, '-WindowTitle', TITLE,
    '-Name', name, '-Method', method, '-Match', match, '-TimeoutMs', String(timeoutMs)];
  let text;
  try {
    text = (await execFileAsync('powershell.exe', args, {timeout: timeoutMs + 60000})).stdout;
  } catch (error) {
    text = String(error.stdout ?? '');
    if (!text.trim()) {
      text = JSON.stringify({status: 'helper_failed', detail: `${error.message}`.slice(0, 200) + String(error.stderr ?? '').slice(-400)});
    }
  }
  const result = JSON.parse(text.trim().split(/\r?\n/).at(-1));
  // Browser UI labels only; never page text.
  if (command === 'Click') gestures.push({name, method, status: result.status, hit: result.hit ?? null, foreground: result.foreground ?? null});
  if (!['clicked', 'found'].includes(result.status)) console.log('GESTURE', command, JSON.stringify(result));
  // A broken helper is a test failure, never a reason for NOT_RUN.
  if (result.status === 'helper_failed') throw new Error(`gesture helper failed: ${result.detail}`);
  return result;
}
const exists = async (name, options) => (await helper('Exists', name, options)).status === 'found';

// Clicks `name` and waits for `effect()`. When an OS mouse click stays covered
// by another window, finds no stable on-screen rectangle or has no visible
// effect, the rest of the run uses UI Automation Invoke and this click is retried.
let inputMethod = 'Mouse';
async function click(name, effect, {match = 'Exact', timeoutMs = 10000} = {}) {
  let result = await helper('Click', name, {method: inputMethod, match, timeoutMs});
  if (result.status === 'clicked' && (await effect())) return result;
  if (!['clicked', 'occluded', 'no_rect', 'stale'].includes(result.status)) return result;
  if (inputMethod !== 'Mouse') return {...result, status: 'no_effect'};
  inputMethod = 'Invoke';
  result = await helper('Click', name, {method: inputMethod, match, timeoutMs: 3000});
  if (result.status !== 'clicked') return {...result, status: 'no_effect'};
  return (await effect()) ? result : {...result, status: 'no_effect'};
}

// The real toolbar path: the pinned action if present, otherwise the Extensions
// menu and this extension's item. The action must grant page access (activeTab,
// observed by the probe), then the popup's own button enables the document.
async function toolbarEnable({extensionName, extensionId, page, probe}) {
  await page.bringToFront();
  const granted = async () => {
    for (const deadline = Date.now() + 5000; Date.now() < deadline; await sleep(200)) {
      if ((await probe()) === 'page_access') return true;
    }
    return false;
  };
  let path;
  if (await exists(extensionName, {match: 'Prefix', timeoutMs: 1500})) {
    path = 'pinned-toolbar-action';
    const action = await click(extensionName, granted, {match: 'Prefix'});
    if (action.status !== 'clicked') throw new Error(`pinned toolbar action granted no page access (${action.status})`);
  } else {
    path = 'extensions-menu';
    const menu = await click('Extensions', () => exists(extensionName, {match: 'Prefix', timeoutMs: 5000}), {timeoutMs: 20000});
    if (menu.status === 'window_not_found') throw new NotRun('browser window not exposed to UI Automation on this desktop');
    if (menu.status === 'not_found') throw new NotRun('Extensions toolbar button not exposed to UI Automation');
    if (menu.status !== 'clicked') throw new NotRun(`Extensions menu did not open through mouse input or UI Automation (${menu.status})`);
    const action = await click(extensionName, granted, {match: 'Prefix'});
    if (action.status !== 'clicked') throw new Error(`Extensions menu item granted no page access (${action.status})`);
  }
  const actionInput = inputMethod;
  const enabled = () => page.locator('#grammar-local-assist').waitFor({timeout: 10000}).then(() => true, () => false);
  let popupInput = inputMethod;
  const enable = await click('Enable this document', enabled);
  if (enable.status === 'not_found') {
    // Popup content not exposed to UI Automation: use Playwright's trusted input
    // on the popup the action opened, if Playwright can see it.
    const popup = page.context().pages().find((candidate) => candidate.url() === `chrome-extension://${extensionId}/popup.html`);
    if (!popup) throw new NotRun('extension popup content not exposed to UI Automation or Playwright');
    await popup.getByRole('button', {name: 'Enable this document', exact: true}).click();
    popupInput = 'playwright';
    if (!(await enabled())) throw new Error('popup Enable this document did not enable the document');
  } else if (enable.status !== 'clicked') {
    throw new Error(`popup Enable this document had no effect (${enable.status})`);
  } else {
    popupInput = inputMethod;
  }
  await page.bringToFront();
  return {path, actionInput, popupInput};
}

const html = (title) => `<!doctype html><html lang=en><meta charset=utf-8><title>${title}</title><body>
<label for=writing>Writing sample</label><textarea id=writing rows=4 cols=50>seperate</textarea>
<div id=editable contenteditable=true style="border:1px solid;width:380px;padding:10px">seperate</div></body></html>`;
const servers = [
  createServer((req, res) => { res.setHeader('Content-Type', 'text/html'); res.end(html(TITLE)); }),
  createServer((req, res) => { res.setHeader('Content-Type', 'text/html'); res.end(html(`${TITLE} (other origin)`)); }),
];
await new Promise((done) => servers[0].listen(PORT, '127.0.0.1', done));
await new Promise((done) => servers[1].listen(OTHER_PORT, '127.0.0.1', done));

let context;
let profile;
let installReceipt;
let packageManifest;
let before;
let hostPath;
const cleanup = {};
try {
  // 1. The installed extension and registration are exactly what the installer produced.
  installReceipt = JSON.parse(await readFile(join(installRoot, 'install-receipt.json'), 'utf8'));
  packageManifest = JSON.parse(await readFile(join(installRoot, 'MANIFEST.json'), 'utf8'));
  const hostName = installReceipt.nativeHost.name;
  before = await extensionState();
  const packaged = Object.fromEntries(packageManifest.files
    .filter((file) => file.path.startsWith('extension/'))
    .map((file) => [file.path.slice('extension/'.length), file.sha256]));
  assert.deepEqual(Object.keys(before).sort(), Object.keys(packaged).sort(), 'installed extension has exactly the packaged files');
  for (const [file, hash] of Object.entries(packaged)) {
    if (file !== 'host-config.js') assert.equal(before[file], hash, `installed ${file} matches the package`);
  }
  assert.equal(await readFile(join(extensionDir, 'host-config.js'), 'utf8'), `export const HOST = ${JSON.stringify(hostName)};\n`);
  const manifest = JSON.parse(await readFile(join(extensionDir, 'manifest.json'), 'utf8'));
  assert.deepEqual(manifest.permissions, ['activeTab', 'scripting', 'nativeMessaging', 'storage']);
  assert.equal(manifest.host_permissions, undefined, 'installed manifest has no host permission');
  assert.equal(manifest.content_scripts, undefined, 'installed manifest injects nothing by itself');
  const keyId = sha256(Buffer.from(manifest.key, 'base64')).slice(0, 32)
    .replace(/[0-9a-f]/g, (c) => String.fromCharCode(97 + parseInt(c, 16)));
  assert.equal(keyId, packageManifest.extensionId);
  assert.equal(installReceipt.extensionId, packageManifest.extensionId);
  const registry = execFileSync('reg.exe', ['query', `HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts\\${hostName}`, '/ve'], {encoding: 'utf8'});
  assert.equal(registry.match(/REG_SZ\s+(.+?)\s*$/m)?.[1], installReceipt.nativeHost.manifest);
  const hostManifest = JSON.parse((await readFile(installReceipt.nativeHost.manifest, 'utf8')).replace(/^\uFEFF/, ''));
  assert.deepEqual(hostManifest.allowed_origins, [`chrome-extension://${packageManifest.extensionId}/`]);
  hostPath = hostManifest.path;
  const packagedHost = packageManifest.files.find((file) => file.path === 'grammar-chromium-host.exe').sha256;
  assert.equal(sha256(await readFile(hostPath)), packagedHost);
  assert.equal(installedHostProcesses(hostPath), 0);
  pass('installed extension files, package key/ID, installed permissions, installer host-config and registration verified unmodified');

  // 2. Fresh task-owned profile; the installed directory is loaded in place.
  profile = await mkdtemp(join(evidence, 'installed-profile-'));
  context = await chromium.launchPersistentContext(profile, {
    headless: false,
    executablePath: process.env.GRAMMAR_CHROMIUM,
    // Keep the browser sandbox that Playwright disables by default.
    chromiumSandbox: true,
    viewport: null,
    args: [
      `--disable-extensions-except=${extensionDir}`,
      `--load-extension=${extensionDir}`,
      // Exposes page and popup content to UI Automation like a screen reader does.
      '--force-renderer-accessibility',
      '--window-position=0,0',
      '--window-size=1000,720',
    ],
  });
  observations.browser = context.browser()?.version() ?? null;
  const worker = context.serviceWorkers()[0] || await context.waitForEvent('serviceworker');
  assert.equal(new URL(worker.url()).host, packageManifest.extensionId);
  const page = context.pages()[0] ?? await context.newPage();
  for (const other of context.pages()) if (other !== page) await other.close();
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(`${origin}/`);
  pass('fresh profile loaded the installed extension with the package ID');

  // Page-access probe (observation only; never used to enable anything).
  const probe = () => worker.evaluate(async () => {
    const tabs = await chrome.tabs.query({});
    if (tabs.length !== 1 || !Number.isInteger(tabs[0].id)) return `tabs:${tabs.length}`;
    try {
      await chrome.scripting.executeScript({target: {tabId: tabs[0].id}, func: () => true});
      return 'page_access';
    } catch (error) {
      return /Cannot access|permission/i.test(String(error)) ? 'denied' : `error:${String(error).slice(0, 120)}`;
    }
  });
  assert.equal(await probe(), 'denied');
  assert.equal(await page.locator('#grammar-local-assist').count(), 0);
  pass('no page access and no Grammar UI before the toolbar gesture');

  // 3. Real toolbar gesture -> activeTab -> popup -> Enable this document.
  const activation = {extensionName: manifest.name, extensionId: packageManifest.extensionId, page, probe};
  observations.activation = await toolbarEnable(activation);
  pass(`toolbar gesture (${observations.activation.path}) granted page access and the popup enabled the document`);

  // 4. textarea: analysis by the installed host -> suggestion -> explicit Accept / Apply edit -> reread.
  const suggestion = page.getByRole('button', {name: /Suggestion 1:/});
  await page.locator('#writing').click();
  await page.getByRole('button', {name: 'Enable this field', exact: true}).click();
  await suggestion.waitFor({timeout: 10000});
  assert.ok(installedHostProcesses(hostPath) >= 1, 'analysis runs in the installer-registered host binary');
  await suggestion.click();
  await page.getByRole('button', {name: 'Accept', exact: true}).click();
  assert.equal(await page.locator('#writing').inputValue(), 'separate');
  await page.locator('#writing').fill('seperate');
  await suggestion.waitFor({timeout: 10000});
  await suggestion.click();
  await page.getByRole('textbox', {name: 'Edit replacement', exact: true}).fill('distinct');
  await page.getByRole('button', {name: 'Apply edit', exact: true}).click();
  assert.equal(await page.locator('#writing').inputValue(), 'distinct');
  pass('textarea: installed-host analysis, suggestion, explicit Accept and Apply edit, each reread');

  // 5. Simple contenteditable.
  await page.locator('#editable').click();
  await page.getByRole('button', {name: 'Enable this field', exact: true}).click();
  await suggestion.waitFor({timeout: 10000});
  await suggestion.click();
  await page.getByRole('button', {name: 'Accept', exact: true}).click();
  assert.equal(await page.locator('#editable').textContent(), 'separate');
  pass('simple contenteditable: suggestion and explicit Accept reread');

  // 6. Disable: UI removed, later edits never analysed, storage holds no text.
  await page.getByRole('button', {name: 'Disable document', exact: true}).click();
  await until('Grammar UI removed', async () => (await page.locator('#grammar-local-assist').count()) === 0);
  await page.locator('#writing').fill('seperate');
  await sleep(1000);
  assert.equal(await page.locator('#grammar-local-assist').count(), 0);
  assert.equal(await page.getByRole('button', {name: /Suggestion/}).count(), 0);
  const stored = await worker.evaluate(async () => {
    const local = await chrome.storage.local.get(null);
    const session = chrome.storage.session ? await chrome.storage.session.get(null) : {};
    return {keys: [...Object.keys(local), ...Object.keys(session)], serialized: JSON.stringify([local, session])};
  });
  assert.ok(stored.keys.every((key) => ['deniedOrigins', 'deepCleanupPending'].includes(key)), `storage keys: ${stored.keys}`);
  assert.ok(!/seperate|separate|distinct/.test(stored.serialized), 'extension storage holds no document text');
  pass('Disable removed the UI; later edits produce no annotation; storage holds no document text');

  // 7. Re-enable needs another toolbar gesture and a fresh field opt-in; nothing cached reappears.
  observations.secondActivation = await toolbarEnable(activation);
  await sleep(1000);
  assert.equal(await page.getByRole('button', {name: /Suggestion/}).count(), 0, 'no annotation before a fresh field opt-in');
  await page.locator('#writing').click();
  await page.getByRole('button', {name: 'Enable this field', exact: true}).click();
  await suggestion.waitFor({timeout: 10000});
  await suggestion.click();
  await page.getByRole('button', {name: 'Accept', exact: true}).click();
  assert.equal(await page.locator('#writing').inputValue(), 'separate');
  pass('second toolbar gesture re-enabled the document; field opt-in was required again and Accept reread');

  // 8. Navigation: a reload drops the opt-in; a cross-origin navigation revokes activeTab.
  await page.reload();
  assert.equal(await page.locator('#grammar-local-assist').count(), 0);
  observations.pageAccessAfterSameOriginReload = await probe();
  await page.goto(`http://127.0.0.1:${OTHER_PORT}/`);
  assert.equal(await probe(), 'denied');
  assert.equal(await page.locator('#grammar-local-assist').count(), 0);
  assert.deepEqual(errors, []);
  pass('reload removed the opt-in and cross-origin navigation revoked page access');
} catch (error) {
  if (error instanceof NotRun) {
    status = 'NOT_RUN';
    notRunCause = error.message;
    console.log('INSTALLED_BROWSER NOT_RUN', notRunCause);
  } else {
    status = 'FAIL';
    failure = String(error?.message ?? error).slice(0, 500);
    console.error('FAIL', error?.stack ?? error);
  }
} finally {
  // 9. Cleanup: browser closed, installed host exited, task-owned profile removed,
  //    installed extension unchanged. Registration and files are removed later
  //    by the uninstaller, whose residue check is a separate step.
  try {
    await context?.close();
    cleanup.browserClosed = true;
  } catch {
    cleanup.browserClosed = false;
  }
  await Promise.all(servers.map((server) => new Promise((done) => server.close(done))));
  if (hostPath) {
    try {
      await until('installed host exited', async () => installedHostProcesses(hostPath) === 0, 15000);
      cleanup.installedHostProcesses = 0;
    } catch {
      cleanup.installedHostProcesses = installedHostProcesses(hostPath);
    }
  }
  if (profile) {
    await rm(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 500}).catch(() => {});
    cleanup.profileRemoved = !existsSync(profile);
  }
  if (before) {
    const after = await extensionState().catch(() => null);
    cleanup.installedExtensionUnchanged = JSON.stringify(after) === JSON.stringify(before);
  }
  const cleanupOk = cleanup.browserClosed !== false && (cleanup.installedHostProcesses ?? 0) === 0 &&
    cleanup.profileRemoved !== false && cleanup.installedExtensionUnchanged !== false;
  if (!cleanupOk && status !== 'FAIL') {
    status = 'FAIL';
    failure = `cleanup incomplete: ${JSON.stringify(cleanup)}`;
  } else if (cleanupOk && status === 'PASS') {
    pass('browser closed, installed host exited, profile removed, installed extension unchanged');
  }
  const result = {
    status,
    classification: 'INSTALLED_PACKAGE_UNMODIFIED_EXTENSION_FRESH_PROFILE',
    source_sha: process.env.GITHUB_SHA,
    run_id: process.env.GITHUB_RUN_ID,
    package_source_commit: packageManifest?.sourceCommit ?? null,
    extension_id: packageManifest?.extensionId ?? null,
    installed_extension_sha256: before ?? null,
    installed_host_sha256: packageManifest?.files.find((file) => file.path === 'grammar-chromium-host.exe')?.sha256 ?? null,
    activation: {
      first: observations.activation ?? null,
      second: observations.secondActivation ?? null,
      input_note: 'Mouse = OS mouse click at the element located by UI Automation; Invoke = UI Automation Invoke',
      gestures,
      permission_injection: false,
      worker_used_for: 'page-access probes (observation only) and a storage read; never to enable',
    },
    page_access_after_same_origin_reload: observations.pageAccessAfterSameOriginReload ?? null,
    browser: observations.browser ?? null,
    browser_sandbox: true,
    checks,
    not_run_cause: notRunCause,
    failure,
    cleanup,
    not_qualified: ['physical keyboard and IME', 'live Provider Deep', 'user profiles and Web Store install', 'Edge'],
  };
  await writeFile(join(out, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  process.exitCode = status === 'FAIL' ? 1 : 0;
}
