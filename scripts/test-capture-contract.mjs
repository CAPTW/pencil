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
