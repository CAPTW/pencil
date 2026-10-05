import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const sourceUrl = new URL("../src/windowChromeContract.ts", import.meta.url);
let source;
try {
  source = await readFile(sourceUrl, "utf8");
} catch (error) {
  assert.fail(`window chrome contract module is required: ${error.code ?? "read_failed"}`);
}

const { outputText } = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2020,
  },
  fileName: sourceUrl.pathname,
  reportDiagnostics: true,
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`;
const contract = await import(moduleUrl);

const draggableTarget = {
  closest() {
    return null;
  },
};
const buttonTarget = {
  closest(selector) {
    return selector.includes("button") ? { tagName: "BUTTON" } : null;
  },
};

let dragStarts = 0;
const windowApi = {
  async startDragging() {
    dragStarts += 1;
  },
};

assert.equal(await contract.startWindowDrag({ button: 0, target: draggableTarget }, windowApi), true);
assert.equal(dragStarts, 1);

assert.equal(await contract.startWindowDrag({ button: 0, target: buttonTarget }, windowApi), false);
assert.equal(await contract.startWindowDrag({ button: 2, target: draggableTarget }, windowApi), false);
assert.equal(await contract.startWindowDrag({ button: 0, target: null }, windowApi), false);
assert.equal(dragStarts, 1);

const tauriConfig = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
);
const mainWindow = tauriConfig.app.windows.find((window) => window.label === "main");
assert.ok(mainWindow, "main window configuration must exist");
assert.equal(mainWindow.resizable, true, "main window must allow native resizing");
assert.ok(
  Number.isFinite(mainWindow.minWidth) && mainWindow.minWidth >= 320 && mainWindow.minWidth <= mainWindow.width,
  "main window must retain a usable minimum width",
);
assert.ok(
  Number.isFinite(mainWindow.minHeight) &&
    mainWindow.minHeight >= 280 &&
    mainWindow.minHeight <= mainWindow.height,
  "main window must retain a usable minimum height",
);

const capability = JSON.parse(
  await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"),
);
assert.ok(
  capability.permissions.includes("core:window:allow-start-dragging"),
  "main window capability must allow only the native drag command used by the title bar",
);

const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.ok(appSource.includes("onMouseDown={beginWindowDrag}"), "top bar must invoke the drag handler");
assert.ok(
  appSource.includes("startWindowDrag(event, getCurrentWindow())"),
  "drag handler must call the tested policy with the current Tauri window",
);

// Reviewed Apply is now supported. Preserve the original visibility requirement
// for both outcomes, and for refusals; inspect the complete function, not a
// regex ending at the first nested closing brace.
const appAst = ts.createSourceFile("App.tsx", appSource, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
let resultAction;
function findAction(node) {
  if (ts.isFunctionDeclaration(node) && node.name?.text === "copyResult") resultAction = node;
  ts.forEachChild(node, findAction);
}
findAction(appAst);
assert.ok(resultAction?.body, "session-bound result action must exist");
const actionSource = resultAction.body.getText(appAst);
assert.match(actionSource, /directApply && !selection\?\.directApply/, "Apply requires the captured capability");
assert.match(actionSource, /sameCaptureToken\(currentTokenRef\.current, token\)/, "stale completions must be rejected");
assert.match(actionSource, /parseApplyOutcome\(rawOutcome\)/, "backend outcomes must be validated");
const branches = [];
function inspectAction(node) {
  if (ts.isCallExpression(node)) {
    assert.doesNotMatch(node.expression.getText(appAst), /(?:^|\.)(?:dismiss|hide|close|setDraft)$/, "result handling must preserve the visible draft");
  }
  if (ts.isIfStatement(node)) branches.push(node);
  ts.forEachChild(node, inspectAction);
}
inspectAction(resultAction.body);
const success = branches.find(node => {
  const condition = node.expression.getText(appAst);
  return condition.includes('outcome.status === "copied_fallback"') && condition.includes('outcome.status === "applied"');
});
assert.ok(success, "confirmed Copy and Apply must share the visible completion path");
assert.match(success.thenStatement.getText(appAst), /dismissButtonRef\.current\?\.focus\(\)/, "completion must offer explicit Dismiss");
const refusal = branches.find(node => node.expression.getText(appAst).includes('outcome.status === "indeterminate"'));
assert.ok(refusal, "uncertain Apply must have its own outcome path");
assert.doesNotMatch(refusal.thenStatement.getText(appAst), /Applied to the captured field/, "uncertainty cannot be presented as success");
console.log("Window chrome and reviewed-result visibility contract PASS");
