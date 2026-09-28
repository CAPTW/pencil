export const CLOUD_PROCESSING_DISCLOSURE_VERSION = 1;

export type ContentLimitKind = "source" | "model_replacement" | "final_apply";

export type ContentLimitFailure = Readonly<{
  kind: ContentLimitKind;
  currentScalars: number;
  maxScalars: number;
  currentBytes: number;
  maxBytes: number;
}>;

const LIMITS: Readonly<Record<ContentLimitKind, Readonly<{ scalars: number; bytes: number }>>> = {
  source: { scalars: 12_000, bytes: 48 * 1024 },
  model_replacement: { scalars: 24_000, bytes: 96 * 1024 },
  final_apply: { scalars: 36_000, bytes: 144 * 1024 },
};

export function validateFrontendTextLimit(
  kind: ContentLimitKind,
  text: string,
): ContentLimitFailure | null {
  const currentScalars = Array.from(text).length;
  const currentBytes = new TextEncoder().encode(text).length;
  const limit = LIMITS[kind];
  return currentScalars <= limit.scalars && currentBytes <= limit.bytes
    ? null
    : {
        kind,
        currentScalars,
        maxScalars: limit.scalars,
        currentBytes,
        maxBytes: limit.bytes,
      };
}

export function isBackendContentLimitError(value: unknown): boolean {
  const message = value instanceof Error ? value.message : String(value);
  return /^(source|model_replacement|final_apply)_content_limit_exceeded:/.test(message);
}

export function contentLimitMessage(failure: ContentLimitFailure): string {
  return `Text is too large for ${failure.kind.replace("_", " ")} (${failure.currentScalars.toLocaleString()}/${failure.maxScalars.toLocaleString()} characters, ${failure.currentBytes.toLocaleString()}/${failure.maxBytes.toLocaleString()} bytes).`;
}

const RUNTIME_ERROR_MESSAGES: Readonly<Record<string, string>> = {
  unsupported_non_text_clipboard:
    "The clipboard contains non-text data. Copy the selected text once, then press the shortcut again.",
  shortcut_modifiers_still_pressed: "Release the shortcut keys, then try again.",
  rewrite_line_structure_changed:
    "The rewrite changed the selection's line structure, so it was rejected. Select the exact text again or use a single-line selection.",
  // Session codes the backend returns as-is: never show a raw code.
  invalid_session_state: "This capture is no longer active. Capture the selection again.",
  stale_session: "This belongs to an earlier capture. Capture the selection again.",
  stale_rewrite_intent: "The mode, language or dictionary changed during the request. Run Deep again.",
  session_token_required: "The widget lost track of the current capture. Capture the selection again.",
  rewrite_interrupted: "The Deep request was interrupted. Run Deep again.",
  cloud_processing_disclosure_required: "Review cloud processing before using Deep.",
  provider_busy: "Another Provider request is still running. Try again in a moment.",
  rewrite_result_ready: "A result is already ready for this capture. Copy it, or capture the selection again to run Deep again.",
  codex_unavailable: "Codex CLI was not found. Instant still works; Deep with Codex needs it, or choose another Provider in Settings.",
};

export function userFacingRuntimeErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  const mapped = RUNTIME_ERROR_MESSAGES[message];
  if (mapped) return mapped;
  // "native_no_selection: No text selected. ..." -> the sentence only.
  const coded = /^[a-z][a-z0-9_]*: (.+)$/s.exec(message);
  return coded ? coded[1] : message;
}
