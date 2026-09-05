import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const sourceUrl = new URL("../src/providerContract.ts", import.meta.url);
const source = await readFile(sourceUrl, "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2020,
  },
  fileName: sourceUrl.pathname,
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`;
const contract = await import(moduleUrl);

const snapshot = contract.parseProviderSnapshot({
  active: "codex",
  busyKind: null,
  statuses: ["codex", "antigravity", "claude"].map((kind) => ({
    kind,
    displayName: contract.providerDisplayName(kind),
    state: kind === "codex" ? "ready" : "signed_out",
    available: true,
    executablePath: "C:\\fake",
    version: "1",
    accountLabel: kind === "codex" ? "Codex account" : null,
    reason: null,
    setupRequirement: null,
    capabilities: {
      writing: true,
      officialSignIn: kind !== "antigravity",
      officialSignOut: kind === "claude",
      cancellation: true,
    },
  })),
});
assert.ok(snapshot);
assert.equal(snapshot.active, "codex");
assert.equal(snapshot.statuses.length, 3);
assert.equal(contract.parseProviderSnapshot({ active: "gemini", busyKind: null, statuses: [] }), null);
assert.equal(contract.providerDisplayName("antigravity"), "Google Antigravity");
console.log("PASS provider contract");
