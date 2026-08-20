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
};

export function userFacingRuntimeErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return RUNTIME_ERROR_MESSAGES[message] ?? message;
}
