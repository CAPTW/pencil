import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const sourceUrl = new URL("../src/captureContract.ts", import.meta.url);
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

const token = contract.parseSelectionCaptured({
  sessionId: "synthetic-session",
  generation: 7,
  selectedText: "synthetic-selected-source",
});
assert.ok(token);
assert.equal(token.sessionId, "synthetic-session");
assert.equal(token.generation, 7);

assert.equal(
  contract.parseSelectionCaptured({
    sessionId: "synthetic-session",
    generation: 0,
    selectedText: "synthetic-selected-source",
  }),
  null,
);
assert.equal(
  contract.parseSelectionCaptured({
    sessionId: "synthetic-session",
    generation: 7,
    selectedText: "synthetic-selected-source",
    unexpected: true,
  }),
  null,
);

assert.equal(
  contract.sameCaptureToken(token, {
    sessionId: "synthetic-session",
    generation: 7,
  }),
  true,
);
assert.equal(
  contract.sameCaptureToken(token, {
    sessionId: "synthetic-session",
    generation: 8,
  }),
  false,
);
assert.equal(
  contract.sameCaptureToken(token, {
    sessionId: "different-session",
    generation: 7,
  }),
  false,
);

assert.deepEqual(contract.parseApplyOutcome({ status: "applied" }), {
  status: "applied",
});
assert.deepEqual(
  contract.parseApplyOutcome({
    status: "copied_fallback",
    reason: "target_not_foreground",
  }),
  { status: "copied_fallback", reason: "target_not_foreground" },
);
assert.deepEqual(contract.parseApplyOutcome({ status: "rejected_stale" }), {
  status: "rejected_stale",
});
assert.equal(
  contract.parseApplyOutcome({ status: "failed", reason: "unknown_reason" }),
  null,
);

for (const reason of ["target_editor_changed", "target_source_changed", "target_selection_changed"]) {
  assert.deepEqual(contract.parseApplyOutcome({ status: "copied_fallback", reason }), {
    status: "copied_fallback",
    reason,
  });
  assert.match(contract.copiedFallbackMessage(reason), /nothing was replaced|cannot be changed safely/);
  assert.match(contract.copiedFallbackMessage(reason), /clipboard/);
}
for (const reason of ["target_mutation_unverified", "editor_lock_not_released"]) {
  assert.deepEqual(contract.parseApplyOutcome({ status: "failed", reason }), { status: "failed", reason });
  assert.equal(contract.applyFailureEndsCapture(reason), true);
  assert.match(contract.applyFailureMessage(reason), /nothing will be retried/);
}
assert.equal(contract.applyFailureEndsCapture("clipboard_write_failed"), false);
assert.equal(
  contract.parseApplyOutcome({ status: "copied_fallback", reason: "target_unknown_reason" }),
  null,
);
console.log("capture contract: PASS");
