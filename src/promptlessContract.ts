import type { CaptureToken } from "./captureContract";

export type RewriteMode = "grammar" | "natural" | "concise" | "polite" | "translate";
export type TranslationTargetLanguage = "ko" | "en" | "ja" | "zh-Hans" | "zh-Hant";
export type TranslationApplyFormat = "translation_only" | "source_with_translation";

export type ShortcutCandidate = Readonly<{
  modifiers: string[];
  key: string;
}>;

export type PrimaryShortcut = ShortcutCandidate &
  Readonly<{
    display: string;
  }>;

export type AppSettings = Readonly<{
  schemaVersion: 4;
  cloudProcessingAcknowledgementVersion: number;
  mode: RewriteMode;
  restoreClipboard: boolean;
  autoRewrite: boolean;
  shortcut: Readonly<{ primary: PrimaryShortcut }>;
  translation: Readonly<{
    sourceLanguage: "auto";
    targetLanguage: TranslationTargetLanguage;
    applyFormat: TranslationApplyFormat;
  }>;
  terminology: Readonly<{
    enabled: boolean;
    activeProfileId: string;
    useApprovedTerminology: boolean;
    suggestTerminology: boolean;
    autoSaveSuggestions: false;
  }>;
}>;

export type RewriteIntentToken = CaptureToken &
  Readonly<{
    mode: RewriteMode;
    targetLanguage: TranslationTargetLanguage | null;
  }>;

export type ShortcutUpdateStatus =
  | "applied"
  | "unchanged"
  | "conflict"
  | "invalid"
  | "persistence_failed_rolled_back"
  | "failed";

export type ShortcutUpdateResponse = Readonly<{
  status: ShortcutUpdateStatus;
  active: PrimaryShortcut;
}>;

export type ShortcutKeyEvent = Readonly<{
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}>;

const MODES = new Set<RewriteMode>(["grammar", "natural", "concise", "polite", "translate"]);
const TARGET_LANGUAGES = new Set<TranslationTargetLanguage>([
  "ko",
  "en",
  "ja",
  "zh-Hans",
  "zh-Hant",
]);
const APPLY_FORMATS = new Set<TranslationApplyFormat>([
  "translation_only",
  "source_with_translation",
]);
const SHORTCUT_STATUSES = new Set<ShortcutUpdateStatus>([
  "applied",
  "unchanged",
  "conflict",
  "invalid",
  "persistence_failed_rolled_back",
  "failed",
]);
const MODIFIER_KEYS = new Set(["Alt", "AltGraph", "Control", "Meta", "Shift"]);

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

function parsePrimaryShortcut(value: unknown): PrimaryShortcut | null {
  if (!isRecord(value) || !hasExactKeys(value, ["modifiers", "key", "display"])) {
    return null;
  }
  if (
    !Array.isArray(value.modifiers) ||
    !value.modifiers.every((modifier) => typeof modifier === "string") ||
    typeof value.key !== "string" ||
    value.key.length === 0 ||
    typeof value.display !== "string" ||
    value.display.length === 0
  ) {
    return null;
  }
  return {
    modifiers: [...value.modifiers],
    key: value.key,
    display: value.display,
  };
}

export function parseAppSettings(value: unknown): AppSettings | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "schemaVersion",
      "cloudProcessingAcknowledgementVersion",
      "mode",
      "restoreClipboard",
      "autoRewrite",
      "shortcut",
      "translation",
      "terminology",
    ]) ||
    value.schemaVersion !== 4 ||
    typeof value.cloudProcessingAcknowledgementVersion !== "number" ||
    !Number.isSafeInteger(value.cloudProcessingAcknowledgementVersion) ||
    value.cloudProcessingAcknowledgementVersion < 0 ||
    value.cloudProcessingAcknowledgementVersion > 1 ||
    typeof value.mode !== "string" ||
    !MODES.has(value.mode as RewriteMode) ||
    typeof value.restoreClipboard !== "boolean" ||
    typeof value.autoRewrite !== "boolean" ||
    !isRecord(value.shortcut) ||
    !hasExactKeys(value.shortcut, ["primary"]) ||
    !isRecord(value.translation) ||
    !hasExactKeys(value.translation, ["sourceLanguage", "targetLanguage", "applyFormat"]) ||
    !isRecord(value.terminology) ||
    !hasExactKeys(value.terminology, [
      "enabled",
      "activeProfileId",
      "useApprovedTerminology",
      "suggestTerminology",
      "autoSaveSuggestions",
    ])
  ) {
    return null;
  }
  const primary = parsePrimaryShortcut(value.shortcut.primary);
  if (
    !primary ||
    value.translation.sourceLanguage !== "auto" ||
    typeof value.translation.targetLanguage !== "string" ||
    !TARGET_LANGUAGES.has(value.translation.targetLanguage as TranslationTargetLanguage) ||
    typeof value.translation.applyFormat !== "string" ||
    !APPLY_FORMATS.has(value.translation.applyFormat as TranslationApplyFormat) ||
    typeof value.terminology.enabled !== "boolean" ||
    typeof value.terminology.activeProfileId !== "string" ||
    value.terminology.activeProfileId.length === 0 ||
    value.terminology.activeProfileId.length > 128 ||
    !/^[A-Za-z0-9_.-]+$/.test(value.terminology.activeProfileId) ||
    typeof value.terminology.useApprovedTerminology !== "boolean" ||
    typeof value.terminology.suggestTerminology !== "boolean" ||
    value.terminology.autoSaveSuggestions !== false
  ) {
    return null;
  }
  return {
    schemaVersion: 4,
    cloudProcessingAcknowledgementVersion: value.cloudProcessingAcknowledgementVersion,
    mode: value.mode as RewriteMode,
    restoreClipboard: value.restoreClipboard,
    autoRewrite: value.autoRewrite,
    shortcut: { primary },
    translation: {
      sourceLanguage: "auto",
      targetLanguage: value.translation.targetLanguage as TranslationTargetLanguage,
      applyFormat: value.translation.applyFormat as TranslationApplyFormat,
    },
    terminology: {
      enabled: value.terminology.enabled,
      activeProfileId: value.terminology.activeProfileId,
      useApprovedTerminology: value.terminology.useApprovedTerminology,
      suggestTerminology: value.terminology.suggestTerminology,
      autoSaveSuggestions: false,
    },
  };
}

export function shortcutCandidateFromKeyEvent(event: ShortcutKeyEvent): ShortcutCandidate | null {
  if (MODIFIER_KEYS.has(event.key)) {
    return null;
  }
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("CTRL");
  if (event.altKey) modifiers.push("ALT");
  if (event.shiftKey) modifiers.push("SHIFT");
  if (event.metaKey) modifiers.push("WIN");

  let key = event.key === " " || event.key === "Spacebar" ? "SPACE" : event.key.toUpperCase();
  if (/^Key[A-Z]$/.test(event.code)) {
    key = event.code.slice(3);
  } else if (/^Digit[0-9]$/.test(event.code)) {
    key = event.code.slice(5);
  }
  return key.length > 0 ? { modifiers, key } : null;
}

export function sameRewriteIntent(
  left: RewriteIntentToken | null | undefined,
  right: RewriteIntentToken | null | undefined,
): boolean {
  return Boolean(
    left &&
      right &&
      left.sessionId === right.sessionId &&
      left.generation === right.generation &&
      left.mode === right.mode &&
      left.targetLanguage === right.targetLanguage,
  );
}

export function parseShortcutUpdateResponse(value: unknown): ShortcutUpdateResponse | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["status", "active"]) ||
    typeof value.status !== "string" ||
    !SHORTCUT_STATUSES.has(value.status as ShortcutUpdateStatus)
  ) {
    return null;
  }
  const active = parsePrimaryShortcut(value.active);
  return active ? { status: value.status as ShortcutUpdateStatus, active } : null;
}
