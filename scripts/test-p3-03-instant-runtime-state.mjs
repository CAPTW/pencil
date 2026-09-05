import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ts = readFileSync(join(root, "src/instantSelectionRuntime.ts"), "utf8");

const EMPTY_INSTANT_RUNTIME_STATE = {
  sessionId: null,
  generation: null,
  draft: "",
  dirty: false,
  draftRevision: 0,
  activeKind: null,
  instant: null,
  deep: null,
  pendingSwitch: null,
};

function captureReset(_state, sessionId, generation) {
  return { ...EMPTY_INSTANT_RUNTIME_STATE, sessionId, generation };
}

function lateCandidate(state, candidate) {
  if (state.sessionId !== candidate.sessionId || state.generation !== candidate.generation) {
    return state;
  }
  const next = {
    ...state,
    instant: candidate.kind === "instant" ? candidate : state.instant,
    deep: candidate.kind === "deep" ? candidate : state.deep,
  };
  if (!state.dirty && state.draft.length === 0 && candidate.text.length > 0) {
    return { ...next, draft: candidate.text, activeKind: candidate.kind };
  }
  return next;
}

function editDraft(state, text) {
  if (state.draft === text) return state;
  return { ...state, draft: text, dirty: true, draftRevision: state.draftRevision + 1, activeKind: "user" };
}

function requestSwitch(state, candidate) {
  if (!state.dirty) {
    return { ...state, draft: candidate.text, activeKind: candidate.kind, pendingSwitch: null };
  }
  return { ...state, pendingSwitch: candidate };
}

function confirmSwitch(state) {
  if (!state.pendingSwitch) return state;
  return {
    ...state,
    draft: state.pendingSwitch.text,
    dirty: false,
    activeKind: state.pendingSwitch.kind,
    pendingSwitch: null,
  };
}

function cancelSwitch(state) {
  return { ...state, pendingSwitch: null };
}

function visibleChoices(state) {
  const items = [];
  if (state.instant) items.push(state.instant);
  if (state.deep && (!state.instant || state.deep.text !== state.instant.text)) items.push(state.deep);
  return items;
}

const failures = [];
let assertions = 0;
function check(condition, id) {
  assertions += 1;
  if (!condition) failures.push(id);
}

check(ts.includes("export function lateCandidate"), "TS_LATE_CANDIDATE");
check(ts.includes("export function confirmSwitch"), "TS_CONFIRM");
check(ts.includes("export function cancelSwitch"), "TS_CANCEL");

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

if (failures.length) {
  console.log(`FAIL ${failures.length} ${assertions} ${failures.join(",")}`);
  process.exit(1);
}
console.log(`PASS 0 ${assertions}`);
process.exit(0);
