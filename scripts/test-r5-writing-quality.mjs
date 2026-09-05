import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (relative) => readFileSync(join(root, relative), "utf8");

const contract = read("src-tauri/src/writing_contract.rs");
const app = read("src/App.tsx");
const review = read("src/resultReview.ts");
const rules = read("src-tauri/src/instant_selection/rules.rs");
const docs = read("docs/WRITING_QUALITY_CONTRACT.md");
const claude = read("src-tauri/src/provider/claude.rs");
const agy = read("src-tauri/src/provider/antigravity.rs");
const main = read("src-tauri/src/main.rs");

assert.match(contract, /WRITING_CONTRACT_VERSION: u32 = 1/);
assert.match(contract, /Preserve LNG, ESD, CBHS/);
assert.match(contract, /untrusted data/);
assert.match(docs, /Writing quality contract/);
assert.match(claude, /writing_contract::extract_json_object/);
assert.match(agy, /writing_contract::extract_json_object/);
assert.match(main, /cancel_rewrite/);
assert.match(rules, /EN_SUBJECT_VERB_THIS_ARE/);
assert.match(rules, /EN_DEMONSTRATIVE_THESE_VESSEL/);
assert.match(rules, /EN_DUPLICATE_WORD/);
assert.match(review, /export function reviewDiff/);
assert.match(app, /data-testid="apply-result"/);
assert.match(app, /data-testid="copy-result"/);
assert.match(app, /data-testid="run-deep"/);
assert.match(app, /data-testid="cancel-rewrite"/);
assert.match(app, /data-testid="dismiss-result"/);
assert.match(app, /result-review/);
assert.doesNotMatch(app, /--permission-prompts none/);
console.log("PASS r5 writing-quality wiring");
