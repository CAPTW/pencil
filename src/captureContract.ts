export type CaptureToken = Readonly<{
  sessionId: string;
  generation: number;
}>;

export type SelectionCapturedPayload = CaptureToken &
  Readonly<{
    selectedText: string;
  }>;

export type ApplyFallbackReason =
  | "target_selection_unverified"
  | "target_missing"
  | "target_process_changed"
  | "target_not_foreground"
  | "target_changed_before_paste"
  | "target_editor_changed"
  | "target_source_changed"
  | "target_selection_changed";

export type ApplyFailureReason =
  | "empty_replacement"
  | "invalid_session_state"
  | "widget_hide_failed"
  | "clipboard_write_failed"
  | "clipboard_ownership_lost"
  | "input_injection_failed"
  | "target_mutation_unverified"
  | "editor_lock_not_released";

export type ApplyOutcome =
  | Readonly<{ status: "applied" }>
  | Readonly<{ status: "copied_fallback"; reason: ApplyFallbackReason }>
  | Readonly<{ status: "rejected_stale" }>
  | Readonly<{ status: "failed"; reason: ApplyFailureReason }>;

const FALLBACK_REASONS = new Set<ApplyFallbackReason>([
  "target_selection_unverified",
  "target_missing",
  "target_process_changed",
  "target_not_foreground",
  "target_changed_before_paste",
  "target_editor_changed",
  "target_source_changed",
  "target_selection_changed",
]);

const FAILURE_REASONS = new Set<ApplyFailureReason>([
  "empty_replacement",
  "invalid_session_state",
  "widget_hide_failed",
  "clipboard_write_failed",
  "clipboard_ownership_lost",
  "input_injection_failed",
  "target_mutation_unverified",
  "editor_lock_not_released",
]);

/** Copy-only explanation for each fallback; the replacement is on the clipboard. */
export function copiedFallbackMessage(reason: ApplyFallbackReason): string {
  switch (reason) {
    case "target_selection_changed":
      return "The selection changed after capture, so nothing was replaced. The result is on the clipboard; paste it manually or capture again.";
    case "target_source_changed":
      return "The text changed after capture, so nothing was replaced. The result is on the clipboard; paste it manually or capture again.";
    case "target_editor_changed":
      return "This field cannot be changed safely (read-only, closed or no longer supported). The result is on the clipboard; paste it manually.";
    case "target_missing":
      return "The captured window is gone. The result is on the clipboard; paste it manually.";
    default:
      return "The captured target could not be proven safe. The approved replacement is on the clipboard; paste it manually.";
  }
}

/** True when the backend cancelled the capture; a retry needs a new capture. */
export function applyFailureEndsCapture(reason: ApplyFailureReason): boolean {
  return (
    reason === "input_injection_failed" ||
    reason === "invalid_session_state" ||
    reason === "target_mutation_unverified" ||
    reason === "editor_lock_not_released"
  );
}

export function applyFailureMessage(reason: ApplyFailureReason): string {
  switch (reason) {
    case "clipboard_ownership_lost":
      return "The clipboard changed before paste. Nothing was pasted; review and try again.";
    case "input_injection_failed":
      return "Windows did not confirm the complete paste input. Automatic retry is disabled; capture again.";
    case "target_mutation_unverified":
      return "The field changed while applying and the result could not be verified. Check the document; nothing will be retried.";
    case "editor_lock_not_released":
      return "The field's temporary read-only lock could not be confirmed released. Check the field before typing; nothing will be retried.";
    default:
      return "Nothing was pasted. Review the target and try again.";
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
