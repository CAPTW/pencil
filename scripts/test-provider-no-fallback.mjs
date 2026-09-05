import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const manager = readFileSync(join(root, "src-tauri/src/provider/manager.rs"), "utf8");
const app = readFileSync(join(root, "src/App.tsx"), "utf8");
assert.match(manager, /SilentFallbackRejected/);
assert.match(manager, /ProviderKind::Codex/);
assert.match(manager, /ProviderKind::Antigravity/);
assert.match(manager, /ProviderKind::Claude/);
assert.match(app, /activeProvider: item\.kind/);
assert.doesNotMatch(app, /fallbackProvider|silentFallback/);
assert.match(app, /Google Antigravity/);
assert.match(app, /Claude/);
assert.match(app, /Instant local candidate/);
console.log("PASS provider no-fallback wiring");
