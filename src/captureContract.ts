export type CaptureToken = Readonly<{
  sessionId: string;
  generation: number;
}>;

export type SelectionCapturedPayload = CaptureToken &
  Readonly<{
    selectedText: string;
  }>;

export type ApplyFallbackReason =
  | "target_missing"
  | "target_process_changed"
  | "target_not_foreground"
  | "target_changed_before_paste";

export type ApplyFailureReason =
  | "empty_replacement"
  | "invalid_session_state"
  | "widget_hide_failed"
  | "clipboard_write_failed"
  | "clipboard_ownership_lost"
  | "input_injection_failed";

export type ApplyOutcome =
  | Readonly<{ status: "applied" }>
  | Readonly<{ status: "copied_fallback"; reason: ApplyFallbackReason }>
  | Readonly<{ status: "rejected_stale" }>
  | Readonly<{ status: "failed"; reason: ApplyFailureReason }>;

const FALLBACK_REASONS = new Set<ApplyFallbackReason>([
  "target_missing",
  "target_process_changed",
  "target_not_foreground",
  "target_changed_before_paste",
]);

const FAILURE_REASONS = new Set<ApplyFailureReason>([
  "empty_replacement",
  "invalid_session_state",
  "widget_hide_failed",
  "clipboard_write_failed",
  "clipboard_ownership_lost",
  "input_injection_failed",
]);

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

export function parseSelectionCaptured(value: unknown): SelectionCapturedPayload | null {
  if (!isRecord(value) || !hasExactKeys(value, ["sessionId", "generation", "selectedText"])) {
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
  if (value.status === "applied" || value.status === "rejected_stale") {
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
