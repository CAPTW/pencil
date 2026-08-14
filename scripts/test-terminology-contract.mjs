import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const sourceUrl = new URL("../src/terminologyContract.ts", import.meta.url);
const source = await readFile(sourceUrl, "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2020,
  },
  fileName: sourceUrl.pathname,
  reportDiagnostics: true,
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`;
const contract = await import(moduleUrl);

const profile = {
  id: "general",
  name: "General",
  enabled: true,
  createdAtMs: 1,
  updatedAtMs: 1,
};
const globalProfile = {
  ...profile,
  id: "global",
  name: "Global",
};
const entry = {
  id: "entry-synthetic",
  profileId: "general",
  type: "preferred",
  status: "suggested",
  sourceText: "synthetic candidate",
  preferredText: "synthetic preference",
  sourceLanguage: "en",
  targetLanguage: "en",
  aliases: [],
  matchMode: "whole_phrase",
  caseSensitive: false,
  priority: 100,
  usageCount: 0,
  occurrenceCount: 1,
  note: null,
  createdAtMs: 2,
  updatedAtMs: 2,
};
const snapshot = contract.parseTerminologyRuntimeSnapshot({
  status: "ready",
  store: {
    schemaVersion: 1,
    revision: 1,
    profiles: [globalProfile, profile],
    entries: [entry],
  },
  recovery: null,
});
assert.ok(snapshot);
assert.equal(snapshot.status, "ready");
assert.equal(snapshot.store.entries[0].status, "suggested");
assert.equal(contract.entryAffectsRequests(snapshot.store.entries[0]), false);
assert.equal(
  contract.parseTerminologyRuntimeSnapshot({
    ...snapshot,
    selectedText: "forbidden synthetic field",
  }),
  null,
);
assert.equal(
  contract.parseTerminologyRuntimeSnapshot({
    status: "ready",
    store: {
      schemaVersion: 1,
      revision: 1,
      profiles: [{ ...profile, id: "profile-only" }],
      entries: [],
    },
    recovery: null,
  }),
  null,
);
assert.equal(
  contract.parseTerminologyRuntimeSnapshot({
    status: "ready",
    store: {
      schemaVersion: 1,
      revision: 1,
      profiles: [
        { ...profile, id: "global", name: "Global", enabled: false },
        profile,
      ],
      entries: [],
    },
    recovery: null,
  }),
  null,
);
assert.equal(
  contract.parseTerminologyRuntimeSnapshot({
    status: "ready",
    store: {
      schemaVersion: 1,
      revision: 1,
      profiles: [
        { ...profile, id: "global", name: "Global" },
        profile,
      ],
      entries: [{ ...entry, profileId: "missing-profile" }],
    },
    recovery: null,
  }),
  null,
);
assert.equal(
  contract.parseTerminologyRuntimeSnapshot({
    status: "ready",
    store: {
      schemaVersion: 1,
      revision: 1,
      profiles: [
        { ...profile, id: "global", name: "Global" },
        profile,
      ],
      entries: [{ ...entry, aliases: ["Alias", " alias "] }],
    },
    recovery: null,
  }),
  null,
);

const rewrite = contract.parseTerminologyRewriteResult({
  replacement: "synthetic replacement",
  changed: true,
  summary: "Synthetic summary.",
  edits: [],
  confidence: 0.9,
  mode: "grammar",
  usedTerminologyIds: ["entry-synthetic"],
  terminologySuggestions: [
    {
      type: "preferred",
      sourceText: "candidate fixture",
      preferredText: "preferred fixture",
      sourceLanguage: "en",
      targetLanguage: "en",
      reason: "preferred_expression",
    },
  ],
  terminologyMatchCount: 1,
  terminologyWarnings: [
    { code: "terminology_usage_unverified", entryIds: ["entry-synthetic"] },
  ],
});
assert.ok(rewrite);
assert.equal(rewrite.terminologySuggestions.length, 1);
assert.equal(
  contract.parseTerminologyRewriteResult({
    ...rewrite,
    terminologySuggestions: Array.from({ length: 6 }, () =>
      rewrite.terminologySuggestions[0],
    ),
  }),
  null,
);
assert.equal(
  contract.parseTerminologyRewriteResult({
    ...rewrite,
    usedTerminologyIds: ["entry-synthetic", "entry-synthetic"],
  }),
  null,
);

const preview = contract.parseImportPlanPreview({
  planId: "plan-synthetic",
  baseRevision: 1,
  expiresAtMs: 100,
  report: {
    newProfiles: 0,
    newEntries: 1,
    identicalDuplicates: 0,
    idConflicts: 0,
    semanticKeyConflicts: 0,
    invalidRows: 0,
    skippedRows: 0,
    conflicts: [],
  },
});
assert.ok(preview);
assert.equal(preview.report.newEntries, 1);
assert.equal(contract.parseImportPlanPreview({ ...preview, unexpected: true }), null);
