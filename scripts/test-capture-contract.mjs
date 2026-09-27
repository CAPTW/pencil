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

// Native editors are never mutated (review findings R1/R2): Copy-only, and the
// message must not claim a verified Apply or that a mutation was prevented.
assert.deepEqual(
  contract.parseApplyOutcome({ status: "copied_fallback", reason: "target_mutation_disabled" }),
  { status: "copied_fallback", reason: "target_mutation_disabled" },
);
assert.match(contract.copiedFallbackMessage("target_mutation_disabled"), /does not change text inside other apps/);
assert.match(contract.copiedFallbackMessage("target_mutation_disabled"), /clipboard/);
// The removed native mutation outcomes are no longer part of the contract.
for (const reason of ["target_editor_changed", "target_source_changed", "target_selection_changed"]) {
  assert.equal(contract.parseApplyOutcome({ status: "copied_fallback", reason }), null);
}
for (const reason of ["target_mutation_unverified", "editor_lock_not_released"]) {
  assert.equal(contract.parseApplyOutcome({ status: "failed", reason }), null);
}
for (const reason of ["target_missing", "target_not_foreground", "target_mutation_disabled"]) {
  assert.doesNotMatch(contract.copiedFallbackMessage(reason), /nothing was replaced/);
}
assert.equal(contract.applyFailureEndsCapture("clipboard_write_failed"), false);
// Input that may have partly reached the target is uncertain, never "failed safely".
assert.equal(contract.applyFailureIsUncertain("input_injection_failed"), true);
assert.equal(contract.applyFailureStatus("input_injection_failed"), "Apply result uncertain");
assert.match(contract.applyFailureMessage("input_injection_failed"), /may have reached the field/);
for (const reason of ["empty_replacement", "invalid_session_state", "widget_hide_failed", "clipboard_write_failed", "clipboard_ownership_lost"]) {
  assert.equal(contract.applyFailureIsUncertain(reason), false);
  assert.equal(contract.applyFailureStatus(reason), "Apply failed safely");
}
assert.equal(
  contract.parseApplyOutcome({ status: "copied_fallback", reason: "target_unknown_reason" }),
  null,
);
console.log("capture contract: PASS");
