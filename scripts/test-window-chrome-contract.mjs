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

// Copy is the only result action: there is no branch that reports a change
// inside another application, so nothing hides the widget as if it had applied.
assert.doesNotMatch(
  appSource,
  /outcome\.status === "applied"/,
  "no Apply success branch may exist; the result is only copied",
);

const copiedFallbackBranch = appSource.match(
  /if \(outcome\.status === "copied_fallback"\) \{(?<body>[\s\S]*?)\n\s*\}/,
);
assert.ok(copiedFallbackBranch?.groups?.body, "copied branch must exist");
assert.doesNotMatch(
  copiedFallbackBranch.groups.body,
  /dismiss\(/,
  "after Copy the widget must remain visible so the status and the copied draft cannot be lost",
);
