export type CaptureToken = Readonly<{
  sessionId: string;
  generation: number;
}>;

export type SelectionCapturedPayload = CaptureToken &
  Readonly<{
    selectedText: string;
    directApply?: boolean;
    acceptExpiresAtMs?: number;
  }>;

// Copy and explicitly qualified target-side Apply have distinct outcomes.
// No uncertain Apply is converted into an automatic clipboard fallback.
export type ApplyFallbackReason =
  | "target_selection_unverified"
  | "target_missing"
  | "target_process_changed"
  | "target_mutation_disabled";

export type ApplyFailureReason =
  | "empty_replacement"
  | "invalid_session_state"
  | "draft_outdated"
  | "clipboard_write_failed";

export type ApplyOutcome =
  | Readonly<{ status: "copied_fallback"; reason: ApplyFallbackReason }>
  | Readonly<{ status: "rejected_stale" }>
  | Readonly<{ status: "applied" }>
  | Readonly<{ status: "rejected_target" }>
  | Readonly<{ status: "indeterminate" }>
  | Readonly<{ status: "failed"; reason: ApplyFailureReason }>;

const FALLBACK_REASONS = new Set<ApplyFallbackReason>([
  "target_selection_unverified",
  "target_missing",
  "target_process_changed",
  "target_mutation_disabled",
]);

const FAILURE_REASONS = new Set<ApplyFailureReason>([
  "empty_replacement",
  "invalid_session_state",
  "draft_outdated",
  "clipboard_write_failed",
]);

/**
 * Note shown with a successful Copy. The normal case needs none: the result is
 * on the clipboard and the user pastes it. The other reasons only add context.
 */
export function copiedNotice(reason: ApplyFallbackReason): string | null {
  switch (reason) {
    // The captured window is still there with the same process.
    case "target_mutation_disabled":
    case "target_selection_unverified":
      return null;
    case "target_missing":
      return "The captured window is gone. The result is on the clipboard; paste it where you need it.";
    case "target_process_changed":
      return "The captured window changed. The result is on the clipboard; paste it where you need it.";
  }
}

/** True when the capture is no longer usable; copying again needs a new capture. */
export function applyFailureEndsCapture(reason: ApplyFailureReason): boolean {
  return reason === "invalid_session_state";
}

export function applyFailureMessage(reason: ApplyFailureReason): string {
  switch (reason) {
    case "clipboard_write_failed":
      return "The clipboard could not be written. Nothing changed; try Copy again.";
    case "invalid_session_state":
      return "This capture is no longer active. Capture the selection again.";
    case "draft_outdated":
      return "A setting changed after this draft was made, so it cannot be copied. Run Deep, or capture the selection again.";
    default:
      return "There is nothing to copy.";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  const actual = Object.keys(value).sort();
  const sortedExpected = [...expected].sort();
  return (
    actual.length === sortedExpected.length &&
    actual.every((key, index) => key === sortedExpected[index])
  );
}

// Identity only: recovery must never fetch or restore a previous document/draft.
export function parseCaptureToken(value: unknown): CaptureToken | null {
  if (!isRecord(value) || !hasExactKeys(value, ["sessionId", "generation"]) ||
    typeof value.sessionId !== "string" || value.sessionId.length === 0 ||
    typeof value.generation !== "number" || !Number.isSafeInteger(value.generation) || value.generation <= 0) return null;
  return { sessionId: value.sessionId, generation: value.generation };
}

export function parseSelectionCaptured(value: unknown): SelectionCapturedPayload | null {
  if (!isRecord(value) || !(hasExactKeys(value, ["sessionId", "generation", "selectedText"]) ||
    (hasExactKeys(value, ["sessionId", "generation", "selectedText", "directApply"]) && typeof value.directApply === "boolean") ||
    (hasExactKeys(value, ["sessionId", "generation", "selectedText", "directApply", "acceptExpiresAtMs"]) && typeof value.directApply === "boolean" && Number.isSafeInteger(value.acceptExpiresAtMs) && (value.acceptExpiresAtMs as number) > 0))) {
    return null;
  }
  if (
    typeof value.sessionId !== "string" ||
    value.sessionId.length === 0 ||
    typeof value.generation !== "number" ||
    !Number.isSafeInteger(value.generation) ||
    value.generation <= 0 ||
    typeof value.selectedText !== "string"
  ) {
    return null;
  }
  return {
    sessionId: value.sessionId,
    generation: value.generation,
    selectedText: value.selectedText,
    ...(typeof value.acceptExpiresAtMs === "number" ? { acceptExpiresAtMs: value.acceptExpiresAtMs } : {}),
    ...(typeof value.directApply === "boolean" ? { directApply: value.directApply } : {}),
  };
}

export function sameCaptureToken(
  left: CaptureToken | null | undefined,
  right: CaptureToken | null | undefined,
): boolean {
  return Boolean(
    left &&
      right &&
      left.sessionId === right.sessionId &&
      left.generation === right.generation,
  );
}

export function parseApplyOutcome(value: unknown): ApplyOutcome | null {
  if (!isRecord(value) || typeof value.status !== "string") {
    return null;
  }
  if (value.status === "applied" || value.status === "rejected_target" || value.status === "indeterminate") {
    return hasExactKeys(value, ["status"]) ? { status: value.status } : null;
  }
  if (value.status === "rejected_stale") {
    return hasExactKeys(value, ["status"]) ? { status: value.status } : null;
  }
  if (value.status === "copied_fallback") {
    return hasExactKeys(value, ["status", "reason"]) &&
      typeof value.reason === "string" &&
      FALLBACK_REASONS.has(value.reason as ApplyFallbackReason)
      ? { status: value.status, reason: value.reason as ApplyFallbackReason }
      : null;
  }
  if (value.status === "failed") {
    return hasExactKeys(value, ["status", "reason"]) &&
      typeof value.reason === "string" &&
      FAILURE_REASONS.has(value.reason as ApplyFailureReason)
      ? { status: value.status, reason: value.reason as ApplyFailureReason }
      : null;
  }
  return null;
}

/** A late backend cancellation must never clear a newer capture. */
export function captureEndedMatches(value: unknown, current: CaptureToken | null): boolean {
  if (!isRecord(value) || !hasExactKeys(value, ["sessionId", "generation", "reason"]) ||
      typeof value.reason !== "string") return false;
  const token = parseSelectionCaptured({sessionId: value.sessionId, generation: value.generation, selectedText: ""});
  return sameCaptureToken(token, current);
}
