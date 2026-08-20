import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const sourceUrl = new URL("../src/mvpContract.ts", import.meta.url);
const source = await readFile(sourceUrl, "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
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

assert.equal(contract.CLOUD_PROCESSING_DISCLOSURE_VERSION, 1);

const boundaries = [
  ["source", 12_000, 48 * 1024],
  ["model_replacement", 24_000, 96 * 1024],
  ["final_apply", 36_000, 144 * 1024],
];
for (const [kind, maxScalars, maxBytes] of boundaries) {
  assert.equal(contract.validateFrontendTextLimit(kind, "a".repeat(maxScalars)), null);
  assert.deepEqual(contract.validateFrontendTextLimit(kind, "a".repeat(maxScalars + 1)), {
    kind,
    currentScalars: maxScalars + 1,
    maxScalars,
    currentBytes: maxScalars + 1,
    maxBytes,
  });
}

assert.equal(contract.validateFrontendTextLimit("source", "가".repeat(12_000)), null);
assert.equal(contract.validateFrontendTextLimit("source", "😀".repeat(12_000)), null);
assert.equal(contract.validateFrontendTextLimit("source", "\r\n".repeat(6_000)), null);
assert.equal(
  contract.validateFrontendTextLimit("source", `${"😀".repeat(12_000)}😀`)?.currentScalars,
  12_001,
);

assert.equal(
  contract.isBackendContentLimitError(
    "source_content_limit_exceeded:current_scalars=12001:max_scalars=12000:current_bytes=12001:max_bytes=49152",
  ),
  true,
);
assert.equal(contract.isBackendContentLimitError("synthetic selected text"), false);

const message = contract.contentLimitMessage({
  kind: "final_apply",
  currentScalars: 36_001,
  maxScalars: 36_000,
  currentBytes: 36_001,
  maxBytes: 147_456,
});
assert.ok(message.includes("36,001/36,000"));
assert.ok(!message.includes("synthetic selected text"));

assert.equal(
  contract.userFacingRuntimeErrorMessage("unsupported_non_text_clipboard"),
  "The clipboard contains non-text data. Copy the selected text once, then press the shortcut again.",
);
assert.equal(
  contract.userFacingRuntimeErrorMessage(new Error("shortcut_modifiers_still_pressed")),
  "Release the shortcut keys, then try again.",
);
assert.equal(
  contract.userFacingRuntimeErrorMessage("rewrite_line_structure_changed"),
  "The rewrite changed the selection's line structure, so it was rejected. Select the exact text again or use a single-line selection.",
);
assert.equal(
  contract.userFacingRuntimeErrorMessage("stable_content_free_backend_error"),
  "stable_content_free_backend_error",
);

for (const label of [
  "Terminology type filter",
  "Terminology status filter",
  "Terminology profile filter",
  "Terminology source-language filter",
  "Terminology target-language filter",
  "Terminology sort order",
  "Terminology entry profile",
  "Terminology entry type",
  "Terminology entry status",
  "Terminology entry source language",
  "Terminology entry target language",
]) {
  assert.ok(
    appSource.includes(`aria-label="${label}"`),
    `missing terminology control label: ${label}`,
  );
}

const entryListIndex = appSource.indexOf('<div className="entry-list">');
const entryFormIndex = appSource.indexOf('<div className="entry-form">');
assert.ok(entryListIndex >= 0, "saved terminology entry list must exist");
assert.ok(entryFormIndex >= 0, "terminology entry form must exist");
assert.ok(
  entryListIndex < entryFormIndex,
  "saved terminology entries must appear before the add/edit form",
);
assert.ok(
  appSource.includes("Saved entries ({filteredTerminologyEntries.length})"),
  "saved terminology list must expose its filtered count",
);
