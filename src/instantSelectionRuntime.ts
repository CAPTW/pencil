export type CandidateKind = "instant" | "deep" | "user";

export interface RuntimeCandidate {
  kind: CandidateKind;
  text: string;
  sessionId: string;
  generation: number;
}

export interface InstantRuntimeState {
  sessionId: string | null;
  generation: number | null;
  draft: string;
  dirty: boolean;
  draftRevision: number;
  activeKind: CandidateKind | null;
  draftOrigin: "instant" | "deep" | null;
  instant: RuntimeCandidate | null;
  deep: RuntimeCandidate | null;
  pendingSwitch: RuntimeCandidate | null;
}

export const EMPTY_INSTANT_RUNTIME_STATE: InstantRuntimeState = {
  sessionId: null,
  generation: null,
  draft: "",
  dirty: false,
  draftRevision: 0,
  activeKind: null,
  draftOrigin: null,
  instant: null,
  deep: null,
  pendingSwitch: null,
};

export function captureReset(
  state: InstantRuntimeState,
  sessionId: string,
  generation: number,
): InstantRuntimeState {
  return {
    ...EMPTY_INSTANT_RUNTIME_STATE,
    sessionId,
    generation,
  };
}

export function lateCandidate(
  state: InstantRuntimeState,
  candidate: RuntimeCandidate,
): InstantRuntimeState {
  if (state.sessionId !== candidate.sessionId || state.generation !== candidate.generation) {
    return state;
  }
  const next = {
    ...state,
    instant: candidate.kind === "instant" ? candidate : state.instant,
    deep: candidate.kind === "deep" ? candidate : state.deep,
  };
  if (next.instant && next.deep && next.instant.text === next.deep.text) {
    next.deep = { ...next.deep, kind: "deep" };
  }
  if (!state.dirty && state.draft.length === 0 && candidate.text.length > 0) {
    return {
      ...next,
      draft: candidate.text,
      activeKind: candidate.kind,
      draftOrigin: candidate.kind === "user" ? null : candidate.kind,
    };
  }
  return next;
}

export function editDraft(state: InstantRuntimeState, text: string): InstantRuntimeState {
  if (state.draft === text) {
    return state;
  }
  return {
    ...state,
    draft: text,
    dirty: true,
    draftRevision: state.draftRevision + 1,
    activeKind: "user",
  };
}

export function requestSwitch(
  state: InstantRuntimeState,
  candidate: RuntimeCandidate,
): InstantRuntimeState {
  if (state.sessionId !== candidate.sessionId || state.generation !== candidate.generation) return state;
  if (!state.dirty) {
    return {
      ...state,
      draft: candidate.text,
      activeKind: candidate.kind,
      draftOrigin: candidate.kind === "user" ? null : candidate.kind,
      pendingSwitch: null,
    };
  }
  return { ...state, pendingSwitch: candidate };
}

export function confirmSwitch(state: InstantRuntimeState): InstantRuntimeState {
  if (!state.pendingSwitch) {
    return state;
  }
  return {
    ...state,
    draft: state.pendingSwitch.text,
    dirty: false,
    activeKind: state.pendingSwitch.kind,
    draftOrigin: state.pendingSwitch.kind === "user" ? null : state.pendingSwitch.kind,
    pendingSwitch: null,
  };
}

export function cancelSwitch(state: InstantRuntimeState): InstantRuntimeState {
  return { ...state, pendingSwitch: null };
}

export function visibleChoices(state: InstantRuntimeState): RuntimeCandidate[] {
  const items: RuntimeCandidate[] = [];
  if (state.instant) {
    items.push(state.instant);
  }
  if (state.deep && (!state.instant || state.deep.text !== state.instant.text)) {
    items.push(state.deep);
  }
  return items;
}

// The immutable candidate and capture remain the authority for an edited draft.
export function instantDraftProof(state: InstantRuntimeState, token: { sessionId: string; generation: number } | null, source: string, draft: string) {
  const candidate = state.instant;
  if (!token || state.sessionId !== token.sessionId || state.generation !== token.generation ||
      state.draftOrigin !== "instant" || !candidate || candidate.sessionId !== token.sessionId ||
      candidate.generation !== token.generation || draft !== state.draft || !draft ||
      !Number.isSafeInteger(state.draftRevision) || state.draftRevision < 0) return null;
  return { sessionId: token.sessionId, generation: token.generation, source,
    candidate: candidate.text, draftRevision: state.draftRevision, userEdited: state.dirty };
}
