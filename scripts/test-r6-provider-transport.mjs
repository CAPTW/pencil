import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (relative) => readFileSync(join(root, relative), "utf8");

const agy = read("src-tauri/src/provider/antigravity.rs");
const claude = read("src-tauri/src/provider/claude.rs");
const manager = read("src-tauri/src/provider/manager.rs");
const cli = read("src-tauri/src/provider/cli.rs");
const types = read("src-tauri/src/provider/types.rs");
const docs = read("docs/PROVIDER_RUNTIME_TRANSPORT.md");

assert.match(manager, /antigravity::rewrite\(/);
assert.match(manager, /SELF_TEST_SOURCE/);
assert.match(agy, /command_line_too_long/);
assert.match(agy, /provider_external_service|ExternalService/);
assert.match(cli, /command_line_too_long/);
assert.match(types, /InputTooLarge/);
assert.doesNotMatch(claude, /--permission-prompts none/);
assert.match(claude, /--permission-mode/);
assert.match(docs, /agy\.exe/);
assert.match(docs, /app-server/);
console.log("PASS r6 provider-transport wiring");
