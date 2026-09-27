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

// The desktop Copy button is the only delivery: every result is a copy.
for (const reason of [
  "target_selection_unverified",
  "target_missing",
  "target_process_changed",
  "target_mutation_disabled",
]) {
  assert.deepEqual(contract.parseApplyOutcome({ status: "copied_fallback", reason }), {
    status: "copied_fallback",
    reason,
  });
}
assert.deepEqual(contract.parseApplyOutcome({ status: "rejected_stale" }), {
  status: "rejected_stale",
});
for (const reason of ["empty_replacement", "invalid_session_state", "clipboard_write_failed"]) {
  assert.deepEqual(contract.parseApplyOutcome({ status: "failed", reason }), { status: "failed", reason });
}
assert.equal(
  contract.parseApplyOutcome({ status: "failed", reason: "unknown_reason" }),
  null,
);
assert.equal(
  contract.parseApplyOutcome({ status: "copied_fallback", reason: "target_unknown_reason" }),
  null,
);
assert.equal(
  contract.parseApplyOutcome({ status: "copied_fallback", reason: "target_missing", extra: true }),
  null,
);

// Nothing reports a change inside another application: the paste-era and
// native mutation outcomes are no longer part of the contract (R1/R2, legacy
// paste removed).
assert.equal(contract.parseApplyOutcome({ status: "applied" }), null);
for (const reason of [
  "target_not_foreground",
  "target_editor_changed",
  "target_source_changed",
  "target_selection_changed",
]) {
  assert.equal(contract.parseApplyOutcome({ status: "copied_fallback", reason }), null);
}
for (const reason of [
  "target_changed_before_paste",
  "widget_hide_failed",
  "clipboard_ownership_lost",
  "input_injection_failed",
  "target_mutation_unverified",
  "editor_lock_not_released",
]) {
  assert.equal(contract.parseApplyOutcome({ status: "failed", reason }), null);
}

// The normal copy needs no notice; the others only add context and never claim
// that text was replaced or that the field was changed.
assert.equal(contract.copiedNotice("target_mutation_disabled"), null);
for (const reason of ["target_selection_unverified", "target_missing", "target_process_changed"]) {
  const notice = contract.copiedNotice(reason);
  assert.match(notice, /clipboard/);
  assert.doesNotMatch(notice, /replaced|applied|pasted into/i);
}
assert.match(contract.copiedNotice("target_missing"), /gone/);

assert.equal(contract.applyFailureEndsCapture("invalid_session_state"), true);
assert.equal(contract.applyFailureEndsCapture("clipboard_write_failed"), false);
assert.equal(contract.applyFailureEndsCapture("empty_replacement"), false);
assert.match(contract.applyFailureMessage("clipboard_write_failed"), /try Copy again/);
assert.match(contract.applyFailureMessage("invalid_session_state"), /Capture the selection again/);
assert.match(contract.applyFailureMessage("empty_replacement"), /nothing to copy/);
for (const reason of ["empty_replacement", "invalid_session_state", "clipboard_write_failed"]) {
  assert.doesNotMatch(contract.applyFailureMessage(reason), /may have reached the field|Apply/);
}
console.log("capture contract: PASS");
