import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];
let assertions = 0;

function check(condition, id) {
  assertions += 1;
  if (!condition) failures.push(id);
}

function read(relativePath) {
  const path = join(root, relativePath);
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

const runtimePath = "src-tauri/src/p3_03_runtime.rs";
const runtime = read(runtimePath);
const mainSource = read("src-tauri/src/main.rs");
const app = read("src/App.tsx");
const reducer = read("src/instantSelectionRuntime.ts");
const doc = read("docs/P3_03_INSTANT_SELECTION_RUNTIME_WIRING.md");

check(existsSync(join(root, runtimePath)), "RUNTIME_MODULE_MISSING");
check(existsSync(join(root, "src/instantSelectionRuntime.ts")), "TS_REDUCER_MISSING");
check(existsSync(join(root, "docs/P3_03_INSTANT_SELECTION_RUNTIME_WIRING.md")), "DOC_MISSING");
check(mainSource.includes("mod p3_03_runtime;"), "MAIN_MOD_MISSING");
// P3-02 prohibited all engine references before runtime integration existed.
// Main now legitimately chooses AnalysisMode for Gemma. Keep the actual boundary:
// analysis runs through p3_03_runtime, not a directly constructed/called engine.
// This is a structural guard; production Rust/reducer tests cover behavior.
function bypassesRuntime(source) {
  const withoutModes = source.replace(/(?:codex_pencil::)?instant_selection::AnalysisMode::[A-Za-z]+/g, "MODE");
  return /mod\s+instant_selection\b|\binstant_selection\s*::/.test(withoutModes);
}
check(!bypassesRuntime(mainSource), "MAIN_ANALYSIS_RUNTIME_BOUNDARY");
check(!bypassesRuntime("codex_pencil::instant_selection::AnalysisMode::Translation"), "GUARD_ALLOWS_MODE");
check(bypassesRuntime("instant_selection::DefaultEngine::default().analyze(&request)"), "GUARD_REJECTS_ENGINE");
check(bypassesRuntime("use codex_pencil::instant_selection::{DefaultEngine};"), "GUARD_REJECTS_IMPORT");
check(mainSource.includes("p3_03_runtime::analyze_correction_source_off_ui(") && mainSource.includes("p3_03_runtime::analyze_gemma_source("), "MAIN_RUNTIME_DISPATCH");
check(!/async fn rewrite_selected_text\([^)]*selected_text/.test(mainSource), "FRONTEND_SOURCE_PARAM");
check(runtime.includes("spawn_blocking"), "NON_BLOCKING_MISSING");
check(runtime.includes("InvalidationReason::Recapture"), "INVALIDATION_RECAPTURE");
check(runtime.includes("InvalidationReason::Cancel"), "INVALIDATION_CANCEL");
check(runtime.includes("InvalidationReason::Dismiss"), "INVALIDATION_DISMISS");
check(runtime.includes("InvalidationReason::SuccessfulApply"), "INVALIDATION_APPLY");
check(runtime.includes("InvalidationReason::AppShutdown"), "INVALIDATION_SHUTDOWN");
check(runtime.includes("InvalidationReason::ModeChange"), "INVALIDATION_MODE");
check(runtime.includes("InvalidationReason::LanguageChange"), "INVALIDATION_LANGUAGE");
check(runtime.includes("InvalidationReason::ProfileChange"), "INVALIDATION_PROFILE");
check(runtime.includes("InvalidationReason::TerminologyRevisionChange"), "INVALIDATION_TERMINOLOGY");
check(runtime.includes("InvalidationReason::SourceIdentityChange"), "INVALIDATION_SOURCE");
check(runtime.includes("InvalidationReason::AnalyzerIdentityChange"), "INVALIDATION_ANALYZER");
check(runtime.includes("AnalysisMode::Correction"), "CORRECTION_ONLY");
check(runtime.includes("EntryType::Protected"), "PROTECTED_SPANS");
check(app.includes("instant-candidate") || app.includes("instantCandidate"), "FRONTEND_INSTANT_EVENT");
check(reducer.includes("dirty") && reducer.includes("lateCandidate"), "REDUCER_LATE_CANDIDATE");
check(reducer.includes("confirmSwitch") && reducer.includes("cancelSwitch"), "REDUCER_SWITCH");
check(doc.includes("AFTER_SELECTION_CAPTURE_TOKEN_ESTABLISHMENT_BEFORE_DEEP_RESULT_PROMOTION"), "SEAM_DOC");

if (failures.length > 0) {
  console.log(`FAIL ${failures.length} ${assertions} ${failures.join(",")}`);
  process.exit(1);
}
console.log(`PASS 0 ${assertions}`);
process.exit(0);
