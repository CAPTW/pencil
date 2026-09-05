export type TerminologyLanguage = "any" | "ko" | "en" | "ja" | "zh-Hans" | "zh-Hant";
export type TerminologyEntryType = "translation" | "preferred" | "protected";
export type TerminologyEntryStatus = "approved" | "suggested" | "disabled";
export type TerminologyEntrySort =
  | "source_text"
  | "priority"
  | "recently_updated"
  | "usage_count";
export type TerminologyWarningCode =
  | "protected_missing"
  | "preferred_missing"
  | "terminology_usage_unverified"
  | "terminology_match_truncated"
  | "terminology_conflict";
export type TerminologySuggestionReason =
  | "translation_candidate"
  | "preferred_expression"
  | "repeated_pair";

export type TerminologyProfile = Readonly<{
  id: string;
  name: string;
  enabled: boolean;
  createdAtMs: number;
  updatedAtMs: number;
}>;

export type TerminologyEntry = Readonly<{
  id: string;
  profileId: string;
  type: TerminologyEntryType;
  status: TerminologyEntryStatus;
  sourceText: string;
  preferredText: string | null;
  sourceLanguage: TerminologyLanguage;
  targetLanguage: TerminologyLanguage;
  aliases: string[];
  matchMode: "whole_phrase";
  caseSensitive: boolean;
  priority: number;
  usageCount: number;
  occurrenceCount: number;
  note: string | null;
  createdAtMs: number;
  updatedAtMs: number;
}>;

export type TerminologyEntryDraft = Readonly<{
  profileId: string;
  type: TerminologyEntryType;
  status: TerminologyEntryStatus;
  sourceText: string;
  preferredText: string | null;
  sourceLanguage: TerminologyLanguage;
  targetLanguage: TerminologyLanguage;
  aliases: string[];
  matchMode: "whole_phrase";
  caseSensitive: boolean;
  priority: number;
  usageCount: number;
  occurrenceCount: number;
  note: string | null;
}>;

export type TerminologyStore = Readonly<{
  schemaVersion: 1;
  revision: number;
  profiles: TerminologyProfile[];
  entries: TerminologyEntry[];
}>;

export type TerminologyRuntimeSnapshot =
  | Readonly<{
      status: "ready";
      store: TerminologyStore;
      recovery: "backup_recovered" | null;
    }>
  | Readonly<{
      status: "unrecoverable" | "uninitialized";
      reason: string;
    }>;

export type TerminologyWarning = Readonly<{
  code: TerminologyWarningCode;
  entryIds: string[];
}>;

export type TerminologySuggestion = Readonly<{
  type: "translation" | "preferred";
  sourceText: string;
  preferredText: string;
  sourceLanguage: TerminologyLanguage;
  targetLanguage: TerminologyLanguage;
  reason: TerminologySuggestionReason;
}>;

export type TerminologyRewriteResult = Readonly<{
  replacement: string;
  changed: boolean;
  summary: string;
  edits: Array<Readonly<{ before: string; after: string; reason: string }>>;
  confidence: number;
  mode: "grammar" | "natural" | "concise" | "polite" | "translate";
  usedTerminologyIds: string[];
  terminologySuggestions: TerminologySuggestion[];
  terminologyMatchCount: number;
  terminologyWarnings: TerminologyWarning[];
  providerUsed: "codex" | "antigravity" | "claude";
}>;

export type ImportConflict = Readonly<{
  kind: "id_conflict" | "semantic_conflict" | "unknown_profile" | "invalid_row";
  incomingId: string | null;
  existingId: string | null;
  rowNumber: number | null;
}>;

export type ImportReport = Readonly<{
  newProfiles: number;
  newEntries: number;
  identicalDuplicates: number;
  idConflicts: number;
  semanticKeyConflicts: number;
  invalidRows: number;
  skippedRows: number;
  conflicts: ImportConflict[];
}>;

export type ImportPlanPreview = Readonly<{
  planId: string;
  baseRevision: number;
  expiresAtMs: number;
  report: ImportReport;
}>;

const LANGUAGES = new Set<TerminologyLanguage>(["any", "ko", "en", "ja", "zh-Hans", "zh-Hant"]);
const ENTRY_TYPES = new Set<TerminologyEntryType>(["translation", "preferred", "protected"]);
const ENTRY_STATUSES = new Set<TerminologyEntryStatus>(["approved", "suggested", "disabled"]);
const MODES = new Set(["grammar", "natural", "concise", "polite", "translate"] as const);
const WARNING_CODES = new Set<TerminologyWarningCode>([
  "protected_missing",
  "preferred_missing",
  "terminology_usage_unverified",
  "terminology_match_truncated",
  "terminology_conflict",
]);
const SUGGESTION_REASONS = new Set<TerminologySuggestionReason>([
  "translation_candidate",
  "preferred_expression",
  "repeated_pair",
]);
const CONFLICT_KINDS = new Set([
  "id_conflict",
  "semantic_conflict",
  "unknown_profile",
  "invalid_row",
] as const);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  const actual = Object.keys(value).sort();
  const sortedExpected = [...expected].sort();
  return actual.length === sortedExpected.length && actual.every((key, index) => key === sortedExpected[index]);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isId(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 128 && /^[A-Za-z0-9_.-]+$/.test(value);
}

function isSingleLineTerm(value: unknown, maximum: number): value is string {
  return typeof value === "string" && value.trim().length > 0 &&
    value === value.trim() && Array.from(value).length <= maximum && !/[\r\n]/.test(value);
}

function normalizedKey(value: string, caseSensitive = false): string {
  const collapsed = value.normalize("NFC").trim().split(/\s+/u).join(" ");
  return caseSensitive ? collapsed : collapsed.toLowerCase();
}

function parseProfile(value: unknown): TerminologyProfile | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["id", "name", "enabled", "createdAtMs", "updatedAtMs"]) ||
    !isId(value.id) ||
    typeof value.name !== "string" ||
    value.name !== value.name.trim() ||
    value.name.length === 0 ||
    Array.from(value.name).length > 128 ||
    /[\r\n]/.test(value.name) ||
    typeof value.enabled !== "boolean" ||
    !isCount(value.createdAtMs) ||
    !isCount(value.updatedAtMs) ||
    value.createdAtMs > value.updatedAtMs
  ) {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    enabled: value.enabled,
    createdAtMs: value.createdAtMs,
    updatedAtMs: value.updatedAtMs,
  };
}

function parseEntry(value: unknown): TerminologyEntry | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "id", "profileId", "type", "status", "sourceText", "preferredText",
      "sourceLanguage", "targetLanguage", "aliases", "matchMode", "caseSensitive",
      "priority", "usageCount", "occurrenceCount", "note", "createdAtMs", "updatedAtMs",
    ]) ||
    !isId(value.id) ||
    !isId(value.profileId) ||
    typeof value.type !== "string" ||
    !ENTRY_TYPES.has(value.type as TerminologyEntryType) ||
    typeof value.status !== "string" ||
    !ENTRY_STATUSES.has(value.status as TerminologyEntryStatus) ||
    !isSingleLineTerm(value.sourceText, 256) ||
    !(typeof value.preferredText === "string" || value.preferredText === null) ||
    typeof value.sourceLanguage !== "string" ||
    !LANGUAGES.has(value.sourceLanguage as TerminologyLanguage) ||
    typeof value.targetLanguage !== "string" ||
    !LANGUAGES.has(value.targetLanguage as TerminologyLanguage) ||
    !Array.isArray(value.aliases) ||
    value.aliases.length > 32 ||
    !value.aliases.every((alias) => isSingleLineTerm(alias, 256)) ||
    value.matchMode !== "whole_phrase" ||
    typeof value.caseSensitive !== "boolean" ||
    !isCount(value.priority) ||
    value.priority > 1000 ||
    !isCount(value.usageCount) ||
    !isCount(value.occurrenceCount) ||
    !(typeof value.note === "string" || value.note === null) ||
    (typeof value.note === "string" && Array.from(value.note).length > 1024) ||
    !isCount(value.createdAtMs) ||
    !isCount(value.updatedAtMs) ||
    value.createdAtMs > value.updatedAtMs
  ) {
    return null;
  }
  if (
    (value.type === "protected" && (value.preferredText !== null || value.caseSensitive !== true)) ||
    (value.type !== "protected" && !isSingleLineTerm(value.preferredText, 512))
  ) return null;
  const aliasKeys = value.aliases.map((alias) => normalizedKey(alias, value.caseSensitive as boolean));
  if (new Set(aliasKeys).size !== aliasKeys.length) return null;
  return value as TerminologyEntry;
}

export function parseTerminologyRuntimeSnapshot(value: unknown): TerminologyRuntimeSnapshot | null {
  if (!isRecord(value) || typeof value.status !== "string") return null;
  if (value.status === "unrecoverable" || value.status === "uninitialized") {
    return hasExactKeys(value, ["status", "reason"]) && typeof value.reason === "string"
      ? { status: value.status, reason: value.reason }
      : null;
  }
  if (
    value.status !== "ready" ||
    !hasExactKeys(value, ["status", "store", "recovery"]) ||
    !(value.recovery === null || value.recovery === "backup_recovered") ||
    !isRecord(value.store) ||
    !hasExactKeys(value.store, ["schemaVersion", "revision", "profiles", "entries"]) ||
    value.store.schemaVersion !== 1 ||
    !isCount(value.store.revision) ||
    !Array.isArray(value.store.profiles) ||
    value.store.profiles.length > 64 ||
    !Array.isArray(value.store.entries) ||
    value.store.entries.length > 10_000
  ) return null;
  const profiles = value.store.profiles.map(parseProfile);
  const entries = value.store.entries.map(parseEntry);
  if (profiles.some((profile) => !profile) || entries.some((entry) => !entry)) return null;
  const parsedProfiles = profiles as TerminologyProfile[];
  const parsedEntries = entries as TerminologyEntry[];
  const profileIds = new Set(parsedProfiles.map((profile) => profile.id));
  const profileNames = new Set(parsedProfiles.map((profile) => normalizedKey(profile.name)));
  const entryIds = new Set(parsedEntries.map((entry) => entry.id));
  if (
    profileIds.size !== parsedProfiles.length ||
    profileNames.size !== parsedProfiles.length ||
    entryIds.size !== parsedEntries.length ||
    parsedEntries.some((entry) => profileIds.has(entry.id) || !profileIds.has(entry.profileId)) ||
    !parsedProfiles.some((profile) => profile.id === "global" && profile.enabled) ||
    !parsedProfiles.some((profile) => profile.id === "general")
  ) return null;
  return {
    status: "ready",
    recovery: value.recovery,
    store: {
      schemaVersion: 1,
      revision: value.store.revision,
      profiles: parsedProfiles,
      entries: parsedEntries,
    },
  };
}

function parseSuggestion(value: unknown): TerminologySuggestion | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["type", "sourceText", "preferredText", "sourceLanguage", "targetLanguage", "reason"]) ||
    !(value.type === "translation" || value.type === "preferred") ||
    !isSingleLineTerm(value.sourceText, 256) ||
    !isSingleLineTerm(value.preferredText, 512) ||
    typeof value.sourceLanguage !== "string" ||
    !LANGUAGES.has(value.sourceLanguage as TerminologyLanguage) ||
    typeof value.targetLanguage !== "string" ||
    !LANGUAGES.has(value.targetLanguage as TerminologyLanguage) ||
    typeof value.reason !== "string" ||
    !SUGGESTION_REASONS.has(value.reason as TerminologySuggestionReason)
  ) return null;
  return value as TerminologySuggestion;
}

export function parseTerminologyRewriteResult(value: unknown): TerminologyRewriteResult | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "replacement", "changed", "summary", "edits", "confidence", "mode",
      "usedTerminologyIds", "terminologySuggestions", "terminologyMatchCount", "terminologyWarnings",
      "providerUsed",
    ]) ||
    typeof value.replacement !== "string" ||
    value.replacement.length === 0 ||
    typeof value.changed !== "boolean" ||
    typeof value.summary !== "string" ||
    !Array.isArray(value.edits) ||
    value.edits.length > 8 ||
    !value.edits.every((edit) =>
      isRecord(edit) && hasExactKeys(edit, ["before", "after", "reason"]) &&
      typeof edit.before === "string" && typeof edit.after === "string" && typeof edit.reason === "string") ||
    typeof value.confidence !== "number" ||
    !Number.isFinite(value.confidence) || value.confidence < 0 || value.confidence > 1 ||
    typeof value.mode !== "string" || !MODES.has(value.mode as never) ||
    !Array.isArray(value.usedTerminologyIds) || value.usedTerminologyIds.length > 50 ||
    !value.usedTerminologyIds.every(isId) ||
    new Set(value.usedTerminologyIds).size !== value.usedTerminologyIds.length ||
    !Array.isArray(value.terminologySuggestions) || value.terminologySuggestions.length > 5 ||
    !isCount(value.terminologyMatchCount) || value.terminologyMatchCount > 50 ||
    !Array.isArray(value.terminologyWarnings) || value.terminologyWarnings.length > 128 ||
    (value.providerUsed !== "codex" && value.providerUsed !== "antigravity" && value.providerUsed !== "claude")
  ) return null;
  const suggestions = value.terminologySuggestions.map(parseSuggestion);
  const warnings = value.terminologyWarnings.map((warning): TerminologyWarning | null => {
    if (!isRecord(warning) || !hasExactKeys(warning, ["code", "entryIds"]) ||
      typeof warning.code !== "string" || !WARNING_CODES.has(warning.code as TerminologyWarningCode) ||
      !Array.isArray(warning.entryIds) || warning.entryIds.length > 50 || !warning.entryIds.every(isId)) return null;
    return { code: warning.code as TerminologyWarningCode, entryIds: [...warning.entryIds] };
  });
  if (suggestions.some((suggestion) => !suggestion) || warnings.some((warning) => !warning)) return null;
  return {
    replacement: value.replacement,
    changed: value.changed,
    summary: value.summary,
    edits: value.edits as TerminologyRewriteResult["edits"],
    confidence: value.confidence,
    mode: value.mode as TerminologyRewriteResult["mode"],
    usedTerminologyIds: [...value.usedTerminologyIds],
    terminologySuggestions: suggestions as TerminologySuggestion[],
    terminologyMatchCount: value.terminologyMatchCount,
    terminologyWarnings: warnings as TerminologyWarning[],
    providerUsed: value.providerUsed,
  };
}

export function parseImportReport(value: unknown): ImportReport | null {
  if (!isRecord(value) || !hasExactKeys(value, [
    "newProfiles", "newEntries", "identicalDuplicates", "idConflicts",
    "semanticKeyConflicts", "invalidRows", "skippedRows", "conflicts",
  ])) return null;
  for (const field of ["newProfiles", "newEntries", "identicalDuplicates", "idConflicts", "semanticKeyConflicts", "invalidRows", "skippedRows"] as const) {
    if (!isCount(value[field])) return null;
  }
  if (!Array.isArray(value.conflicts) || value.conflicts.length > 10_064) return null;
  const conflicts = value.conflicts.map((conflict): ImportConflict | null => {
    if (!isRecord(conflict) || !hasExactKeys(conflict, ["kind", "incomingId", "existingId", "rowNumber"]) ||
      typeof conflict.kind !== "string" || !CONFLICT_KINDS.has(conflict.kind as never) ||
      !(conflict.incomingId === null || isId(conflict.incomingId)) ||
      !(conflict.existingId === null || isId(conflict.existingId)) ||
      !(conflict.rowNumber === null || isCount(conflict.rowNumber))) return null;
    return conflict as ImportConflict;
  });
  if (conflicts.some((conflict) => !conflict)) return null;
  return { ...(value as Omit<ImportReport, "conflicts">), conflicts: conflicts as ImportConflict[] };
}

export function parseImportPlanPreview(value: unknown): ImportPlanPreview | null {
  if (!isRecord(value) || !hasExactKeys(value, ["planId", "baseRevision", "expiresAtMs", "report"]) ||
    !isId(value.planId) || !isCount(value.baseRevision) || !isCount(value.expiresAtMs)) return null;
  const report = parseImportReport(value.report);
  return report ? { planId: value.planId, baseRevision: value.baseRevision, expiresAtMs: value.expiresAtMs, report } : null;
}

export function entryAffectsRequests(entry: TerminologyEntry): boolean {
  return entry.status === "approved";
}
