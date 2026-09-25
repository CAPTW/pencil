import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import typescript from "typescript";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = readFileSync(join(root, "src/instantSelectionRuntime.ts"), "utf8");
const compiled = typescript.transpileModule(source, {
  compilerOptions: { target: typescript.ScriptTarget.ES2022, module: typescript.ModuleKind.ES2022 },
});
// Execute the production exports; a changed reducer must change this test's result.
const { EMPTY_INSTANT_RUNTIME_STATE, captureReset, lateCandidate, editDraft,
  requestSwitch, confirmSwitch, cancelSwitch, visibleChoices, instantDraftProof } =
  await import(`data:text/javascript;base64,${Buffer.from(compiled.outputText).toString("base64")}`);

const failures = [];
let assertions = 0;
function check(condition, id) {
  assertions += 1;
  if (!condition) failures.push(id);
}


let state = captureReset(EMPTY_INSTANT_RUNTIME_STATE, "s", 1);
const instant = { kind: "instant", text: "instant-text", sessionId: "s", generation: 1 };
const deep = { kind: "deep", text: "deep-text", sessionId: "s", generation: 1 };
state = lateCandidate(state, instant);
check(state.draft === "instant-text", "FIRST_CANDIDATE_INIT");
state = editDraft(state, "user-edit");
state = lateCandidate(state, deep);
check(state.draft === "user-edit", "LATE_DEEP_NO_OVERWRITE");
check(state.deep && state.deep.text === "deep-text", "DEEP_ALTERNATE");
state = requestSwitch(state, deep);
check(state.pendingSwitch && state.draft === "user-edit", "DIRTY_SWITCH_PENDING");
state = cancelSwitch(state);
check(state.draft === "user-edit" && state.pendingSwitch === null, "CANCEL_KEEPS_DRAFT");
state = requestSwitch(state, deep);
state = confirmSwitch(state);
check(state.draft === "deep-text" && state.dirty === false, "CONFIRM_SWITCH");
const sameGenInstant = { kind: "instant", text: "instant-text", sessionId: "s", generation: 2 };
const dupState = lateCandidate(
  lateCandidate(captureReset(EMPTY_INSTANT_RUNTIME_STATE, "s", 2), sameGenInstant),
  { ...sameGenInstant, kind: "deep" },
);
check(visibleChoices(dupState).length === 1, "DEDUP_IDENTICAL");
const stale = lateCandidate(dupState, { ...deep, sessionId: "other", generation: 9 });
check(stale.draft === dupState.draft, "STALE_IGNORED");

const token = { sessionId: "s", generation: 1 };
let edited = lateCandidate(captureReset(EMPTY_INSTANT_RUNTIME_STATE, "s", 1), instant);
check(editDraft(edited, edited.draft) === edited, "UNCHANGED_EDIT_IS_NOOP");
const untouchedProof = instantDraftProof(edited, token, "original source", edited.draft);
check(untouchedProof?.userEdited === false && untouchedProof.draftRevision === 0, "INSTANT_UNEDITED_PROOF");
edited = editDraft(edited, "edited Instant");
const proof = instantDraftProof(edited, token, "original source", edited.draft);
check(proof?.candidate === instant.text && proof.source === "original source", "EDIT_RETAINS_SOURCE_CANDIDATE");
check(proof?.draftRevision === 1 && proof.userEdited === true, "EDIT_REVISION_PROOF");
check(instantDraftProof(edited, null, "original source", edited.draft) === null, "PROOF_NEEDS_CAPTURE");
check(instantDraftProof(edited, {...token, generation: 2}, "original source", edited.draft) === null, "PROOF_REJECTS_STALE_GENERATION");
check(instantDraftProof(edited, {...token, sessionId: "other"}, "original source", edited.draft) === null, "PROOF_REJECTS_STALE_SESSION");
check(instantDraftProof(edited, token, "original source", "untracked text") === null, "PROOF_REJECTS_UNTRACKED_EDIT");
check(instantDraftProof({...edited, draftRevision: Number.MAX_SAFE_INTEGER + 1}, token, "original source", edited.draft) === null, "PROOF_REJECTS_UNSAFE_REVISION");
check(instantDraftProof({...edited, instant: {...instant, generation: 2}}, token, "original source", edited.draft) === null, "PROOF_REJECTS_STALE_CANDIDATE");
check(instantDraftProof(editDraft(edited, ""), token, "original source", "") === null, "PROOF_REJECTS_EMPTY_DRAFT");
const withDeep = lateCandidate(edited, deep);
check(withDeep.draft === edited.draft && withDeep.draftOrigin === "instant", "LATE_DEEP_PRESERVES_EDIT_PROVENANCE");
check(instantDraftProof(withDeep, token, "original source", edited.draft)?.userEdited === true, "LATE_DEEP_KEEPS_INSTANT_PROOF");
check(requestSwitch(edited, {...deep, generation: 2}) === edited, "STALE_SWITCH_IGNORED");
const switched = confirmSwitch(requestSwitch(edited, deep));
check(switched.draftOrigin === "deep" && instantDraftProof(switched, token, "original source", switched.draft) === null, "DEEP_SWITCH_HAS_NO_INSTANT_PROOF");
const fresh = captureReset(requestSwitch(edited, deep), "new", 2);
check(fresh.pendingSwitch === null && fresh.draft === "" && fresh.instant === null && fresh.deep === null, "RECAPTURE_WIPES_DRAFT_AND_CANDIDATES");
check(instantDraftProof(fresh, token, "original source", edited.draft) === null, "RECAPTURE_INVALIDATES_PROOF");
check(confirmSwitch(fresh) === fresh, "CONFIRM_WITHOUT_PENDING_NOOP");
check(lateCandidate(fresh, deep) === fresh, "OLD_CAPTURE_COMPLETION_NOOP");

if (failures.length) {
  console.log(`FAIL ${failures.length} ${assertions} ${failures.join(",")}`);
  process.exit(1);
}
console.log(`PASS 0 ${assertions}`);
process.exit(0);
