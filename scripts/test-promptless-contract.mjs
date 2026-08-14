import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const sourceUrl = new URL("../src/promptlessContract.ts", import.meta.url);
const source = await readFile(sourceUrl, "utf8");
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

const settings = contract.parseAppSettings({
  schemaVersion: 2,
  mode: "translate",
  restoreClipboard: true,
  autoRewrite: true,
  shortcut: {
    primary: {
      modifiers: ["CTRL", "SHIFT"],
      key: "G",
      display: "Ctrl+Shift+G",
    },
  },
  translation: {
    sourceLanguage: "auto",
    targetLanguage: "zh-Hant",
    applyFormat: "source_with_translation",
  },
});
assert.ok(settings);
assert.equal(settings.translation.targetLanguage, "zh-Hant");
assert.equal(
  contract.parseAppSettings({
    ...settings,
    translation: { ...settings.translation, targetLanguage: "unsupported" },
  }),
  null,
);
assert.deepEqual(
  contract.shortcutCandidateFromKeyEvent({
    key: " ",
    code: "Space",
    ctrlKey: true,
    altKey: false,
    shiftKey: false,
    metaKey: false,
  }),
  { modifiers: ["CTRL"], key: "SPACE" },
);

assert.deepEqual(
  contract.shortcutCandidateFromKeyEvent({
    key: "g",
    code: "KeyG",
    ctrlKey: true,
    altKey: false,
    shiftKey: true,
    metaKey: false,
  }),
  { modifiers: ["CTRL", "SHIFT"], key: "G" },
);
assert.equal(
  contract.shortcutCandidateFromKeyEvent({
    key: "Shift",
    code: "ShiftLeft",
    ctrlKey: false,
    altKey: false,
    shiftKey: true,
    metaKey: false,
  }),
  null,
);

const currentIntent = {
  sessionId: "synthetic-session",
  generation: 3,
  mode: "translate",
  targetLanguage: "ja",
};
assert.equal(contract.sameRewriteIntent(currentIntent, { ...currentIntent }), true);
assert.equal(
  contract.sameRewriteIntent(currentIntent, { ...currentIntent, targetLanguage: "en" }),
  false,
);
assert.equal(
  contract.sameRewriteIntent(currentIntent, { ...currentIntent, generation: 4 }),
  false,
);

assert.deepEqual(
  contract.parseShortcutUpdateResponse({
    status: "conflict",
    active: settings.shortcut.primary,
  }),
  { status: "conflict", active: settings.shortcut.primary },
);
assert.equal(
  contract.parseShortcutUpdateResponse({
    status: "unknown",
    active: settings.shortcut.primary,
  }),
  null,
);
