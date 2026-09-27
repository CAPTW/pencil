import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];
const notRun = [];
let assertions = 0;

function check(condition, id) {
  assertions += 1;
  if (!condition) failures.push(id);
}

// The native-process and UTF-8 JSON self-tests need Windows PowerShell 5.1 and
// an explicit task-owned root (P3_02_NATIVE_SELF_TEST_ROOT,
// P3_02_UTF8_JSON_SELF_TEST_ROOT). Without them these checks did not run: they
// are NOT_RUN, never PASS, and the contract does not report GREEN. Windows CI
// sets both roots.
function selfTestCheck(ran, condition, id) {
  if (!ran) {
    notRun.push(id);
    return;
  }
  check(condition, id);
}

function read(relativePath) {
  const path = join(root, relativePath);
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

const required = [
  "src-tauri/src/lib.rs",
  "src-tauri/src/instant_selection/mod.rs",
  "src-tauri/src/instant_selection/cache.rs",
  "src-tauri/src/instant_selection/candidate.rs",
  "src-tauri/src/instant_selection/engine.rs",
  "src-tauri/src/instant_selection/identity.rs",
  "src-tauri/src/instant_selection/reconciliation.rs",
  "src-tauri/src/instant_selection/rules.rs",
  "src-tauri/src/instant_selection/types.rs",
  "src-tauri/src/instant_selection/utf16.rs",
  "src-tauri/tests/instant_selection/main.rs",
  "scripts/run-instant-selection-product-benchmark.ps1",
  "docs/P3_02_INSTANT_SELECTION_ENGINE_FOUNDATION.md",
];
for (const path of required) check(existsSync(join(root, path)), `REQUIRED_PATH_MISSING:${path}`);

const lib = read("src-tauri/src/lib.rs");
const moduleSources = required
  .filter((path) => path.startsWith("src-tauri/src/instant_selection/") && path.endsWith(".rs"))
  .map(read)
  .join("\n");
const tests = read("src-tauri/tests/instant_selection/main.rs");
const runner = read("scripts/run-instant-selection-product-benchmark.ps1");
const documentation = read("docs/P3_02_INSTANT_SELECTION_ENGINE_FOUNDATION.md");
const mainSource = read("src-tauri/src/main.rs");

const nativeSelfTestBase = process.env.P3_02_NATIVE_SELF_TEST_ROOT;
let nativeSelfTestRun = null;
let nativeSelfTestResult = null;
let nativeSelfTestLineCount = 0;
let nativeSelfTestWorkRoot = null;
if (nativeSelfTestBase) {
  mkdirSync(nativeSelfTestBase, { recursive: true });
  nativeSelfTestWorkRoot = mkdtempSync(join(nativeSelfTestBase, "native-process-contract-"));
  const outputRoot = join(nativeSelfTestWorkRoot, "output");
  const cargoTargetRoot = join(nativeSelfTestWorkRoot, "cargo-target");
  mkdirSync(cargoTargetRoot);
  const windowsPowerShell = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
  nativeSelfTestRun = spawnSync(windowsPowerShell, [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    join(root, "scripts", "run-instant-selection-product-benchmark.ps1"),
    "-OutputRoot",
    outputRoot,
    "-CargoTargetRoot",
    cargoTargetRoot,
    "-NativeProcessSelfTest",
  ], { cwd: root, encoding: "utf8", windowsHide: true });
  const lines = (nativeSelfTestRun.stdout ?? "").split(/\r?\n/u).filter((line) => line.length > 0);
  nativeSelfTestLineCount = lines.length;
  if (lines.length === 1) {
    try {
      nativeSelfTestResult = JSON.parse(lines[0]);
    } catch {
      nativeSelfTestResult = null;
    }
  }
}

const utf8JsonSelfTestBase = process.env.P3_02_UTF8_JSON_SELF_TEST_ROOT;
let utf8JsonSelfTestRun = null;
let utf8JsonSelfTestResult = null;
let utf8JsonSelfTestLineCount = 0;
let utf8JsonSelfTestWorkRoot = null;
if (utf8JsonSelfTestBase) {
  mkdirSync(utf8JsonSelfTestBase, { recursive: true });
  utf8JsonSelfTestWorkRoot = mkdtempSync(join(utf8JsonSelfTestBase, "utf8-json-contract-"));
  const outputRoot = join(utf8JsonSelfTestWorkRoot, "output");
  const cargoTargetRoot = join(utf8JsonSelfTestWorkRoot, "cargo-target");
  mkdirSync(cargoTargetRoot);
  const windowsPowerShell = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
  utf8JsonSelfTestRun = spawnSync(windowsPowerShell, [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    join(root, "scripts", "run-instant-selection-product-benchmark.ps1"),
    "-OutputRoot",
    outputRoot,
    "-CargoTargetRoot",
    cargoTargetRoot,
    "-Utf8JsonSelfTest",
  ], { cwd: root, encoding: "utf8", windowsHide: true });
  const lines = (utf8JsonSelfTestRun.stdout ?? "").split(/\r?\n/u).filter((line) => line.length > 0);
  utf8JsonSelfTestLineCount = lines.length;
  if (lines.length === 1) {
    try {
      utf8JsonSelfTestResult = JSON.parse(lines[0]);
    } catch {
      utf8JsonSelfTestResult = null;
    }
  }
}

const nativeRan = nativeSelfTestRun !== null;
selfTestCheck(nativeRan, nativeSelfTestRun?.status === 0 && nativeSelfTestResult?.cases?.stderrWithZeroExit === true, "native-process.stderr-with-zero-exit-is-not-failure");
selfTestCheck(nativeRan, nativeSelfTestResult?.cases?.nonzeroExitClassifiedAsFailure === true, "native-process.nonzero-exit-is-failure");
selfTestCheck(nativeRan, nativeSelfTestResult?.cases?.stdoutStderrSeparated === true, "native-process.stdout-stderr-remain-separated");
selfTestCheck(nativeRan, nativeSelfTestResult?.cases?.largeDualStreamCompleted === true, "native-process.concurrent-dual-stream-drain-does-not-deadlock");
selfTestCheck(nativeRan, nativeSelfTestLineCount === 1 && nativeSelfTestResult?.resultObjectCountExactlyOne === true, "native-process.result-object-count-is-exactly-one");
selfTestCheck(nativeRan, nativeSelfTestResult?.exitCodeRuntimeType === "System.Int32" && nativeSelfTestResult?.exitCodeIsSignedInteger === true, "native-process.exit-code-is-a-signed-integer");
selfTestCheck(nativeRan, nativeSelfTestResult?.cases?.createNewAndReuseRejected === true, "native-process.output-files-are-create-new-and-not-reused");
selfTestCheck(nativeRan, nativeSelfTestResult?.cases?.argumentWithSpacesRoundtrip === true, "native-process.argument-roundtrip-supports-spaces");
check(/\$BuildResult\s*=\s*Invoke-NativeProcess/u.test(runner), "benchmark-runner.cargo-invocation-uses-the-native-process-helper");
check(/\$WriterResult\s*=\s*Invoke-NativeProcess/u.test(runner) && /\$EvaluatorResult\s*=\s*Invoke-NativeProcess/u.test(runner) && /Assert-NativeProcessSucceeded/u.test(runner), "benchmark-runner.writer-and-evaluator-use-the-same-exit-authority");

const defaultEncodingReadForbidden = !/\bGet-Content\b/u.test(runner)
  && !/\[System\.IO\.File\]::ReadAllText/u.test(runner)
  && !/\bOut-File\b/u.test(runner)
  && !/\bSet-Content\b/u.test(runner)
  && !/\bAdd-Content\b/u.test(runner);
const predictionsUsesStrictReader = /\$Predictions\s*=\s*Read-StrictUtf8Json\s+-Path\s+\$PredictionsPath/u.test(runner);
const generatedJsonBoundariesUseStrictReader = predictionsUsesStrictReader
  && /\$CorpusManifest\s*=\s*Read-StrictUtf8Json\s+-Path\s+\$CorpusManifestPath/u.test(runner)
  && /\$Report\s*=\s*Read-StrictUtf8Json\s+-Path\s+\$EvaluatorReportPath/u.test(runner)
  && /Read-StrictUtf8JsonLines\s+-Path\s+\$BuildStdout/u.test(runner)
  && defaultEncodingReadForbidden;
const utf8Ran = utf8JsonSelfTestRun !== null;
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.strictByteReaderRequired === true, "utf8-json.strict-byte-reader-required");
check(defaultEncodingReadForbidden, "utf8-json.default-encoding-read-forbidden");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.cases?.noBomMixedLanguageParses === true, "utf8-json.no-bom-mixed-language-parses");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.cases?.bomMixedLanguageParses === true, "utf8-json.bom-mixed-language-parses");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.cases?.bomNoBomSemanticEquality === true, "utf8-json.bom-no-bom-semantic-equality");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.cases?.malformedSequenceFailsClosed === true, "utf8-json.malformed-sequence-fails-closed");
// The runner-source half of these two checks is static and always runs; the
// self-test half needs Windows PowerShell 5.1.
check(predictionsUsesStrictReader, "utf8-json.predictions-boundary-uses-strict-reader.source");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.boundaries?.predictionsUsesStrictReader === true, "utf8-json.predictions-boundary-uses-strict-reader");
check(generatedJsonBoundariesUseStrictReader, "utf8-json.all-generated-json-boundaries-use-strict-reader.source");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.boundaries?.allGeneratedJsonUseStrictReader === true, "utf8-json.all-generated-json-boundaries-use-strict-reader");
selfTestCheck(utf8Ran, utf8JsonSelfTestRun?.status === 0 && utf8JsonSelfTestLineCount === 1 && utf8JsonSelfTestResult?.contentFreeResult === true && utf8JsonSelfTestResult?.cleanupSuccessful === true, "utf8-json.self-test-emits-one-content-free-result");
selfTestCheck(utf8Ran, utf8JsonSelfTestResult?.ps51ParserGatePassed === true && utf8JsonSelfTestResult?.parserErrorCount === 0 && Number.isInteger(utf8JsonSelfTestResult?.parserTokenCount) && utf8JsonSelfTestResult.parserTokenCount > 0, "utf8-json.ps51-parser-gate-is-required");

if (nativeSelfTestWorkRoot) rmSync(nativeSelfTestWorkRoot, { recursive: true, force: true });
if (utf8JsonSelfTestWorkRoot) rmSync(utf8JsonSelfTestWorkRoot, { recursive: true, force: true });

check(lib.includes("pub mod instant_selection"), "LIBRARY_MODULE_EXPORT_MISSING");
check(moduleSources.includes('deterministic-rule-engine'), "ENGINE_DESCRIPTOR_ID_MISSING");
check(moduleSources.includes('0.1.0'), "ENGINE_DESCRIPTOR_VERSION_MISSING");
check(moduleSources.includes("runtime_wired: false"), "RUNTIME_WIRING_FALSE_MISSING");
check(moduleSources.includes("network_used: false"), "NETWORK_FALSE_MISSING");
check(moduleSources.includes("persistent_cache_used: false"), "PERSISTENCE_FALSE_MISSING");
check(moduleSources.includes("MAX_SUGGESTIONS"), "SUGGESTION_CAP_MISSING");
check(moduleSources.includes("SourceIdentityMismatch"), "SOURCE_IDENTITY_REJECTION_MISSING");
check(moduleSources.includes("SurrogateSplit"), "SURROGATE_REJECTION_MISSING");
check(moduleSources.includes("OverlappingSuggestions"), "OVERLAP_REJECTION_MISSING");
check(moduleSources.includes("suppressed_suggestion_count"), "PROTECTED_SUPPRESSION_COUNT_MISSING");
check(moduleSources.includes("sort_by"), "DETERMINISTIC_SORT_MISSING");
check(moduleSources.includes("InvalidationReason"), "CACHE_INVALIDATION_MISSING");
check(moduleSources.includes("UserEdited"), "USER_EDIT_STATE_MISSING");
check(moduleSources.includes("recoverable_user_draft"), "USER_DRAFT_RECOVERY_MISSING");
check(!mainSource.includes("mod instant_selection") && !mainSource.includes("instant_selection::"), "RUNTIME_WIRING_PRESENT");

for (const token of ["std::fs", "std::process", "Command::", "env::", "reqwest", "tauri::", "arboard", "clipboard", "CodexClient", "TcpListener", "UdpSocket"]) {
  check(!moduleSources.includes(token), `FORBIDDEN_ENGINE_API:${token}`);
}
for (const token of [".unwrap(", ".expect(", "panic!(", "println!(", "eprintln!(", "dbg!("]) {
  check(!moduleSources.includes(token), `FORBIDDEN_ENGINE_RUNTIME_TOKEN:${token}`);
}

check(tests.includes("product_benchmark_writer_uses_the_same_rust_analysis_api"), "PRODUCT_BENCHMARK_WRITER_TEST_MISSING");
check(/engine\s*\.\s*analyze\s*\(\s*&request\s*\)/.test(tests), "PRODUCT_BENCHMARK_ENGINE_API_CALL_MISSING");
check(runner.includes("product_benchmark_writer_uses_the_same_rust_analysis_api"), "BENCHMARK_RUNNER_WRITER_CALL_MISSING");
check(runner.includes("evaluate-instant-selection-benchmark.mjs"), "IMMUTABLE_EVALUATOR_CALL_MISSING");
check(runner.includes("P3_02_CORPUS_PATH"), "BENCHMARK_CORPUS_BOUNDARY_MISSING");
check(runner.includes("semantic-output"), "SEMANTIC_OUTPUT_BOUNDARY_MISSING");

const requiredRuleIds = [
  "KO_SPACING_CONFIRM",
  "KO_TYPO_FINAL",
  "KO_TENSE_AGREEMENT",
  "EN_SPELLING_SEPARATE",
  "EN_SUBJECT_VERB_RESULTS",
  "MIXED_EN_SUCCESSFUL",
  "KO_SPACING_STABLE",
  "KO_SPACING_MODIFY",
];
for (const ruleId of requiredRuleIds) {
  check(moduleSources.includes(ruleId), `RULE_MISSING:${ruleId}`);
  check(tests.split(ruleId).length >= 6, `RULE_TEST_COVERAGE_MISSING:${ruleId}`);
  check(documentation.includes(ruleId), `RULE_DOCUMENTATION_MISSING:${ruleId}`);
}

const corpusPath = join(root, "benchmarks/instant-selection/corpus.synthetic.v1.jsonl");
if (existsSync(corpusPath)) {
  const corpus = readFileSync(corpusPath, "utf8").trimEnd().split("\n").map((line) => JSON.parse(line));
  check(corpus.length === 120, "IMMUTABLE_CORPUS_CASE_COUNT_MISMATCH");
  for (const item of corpus) {
    check(!moduleSources.includes(item.caseId), "BENCHMARK_CASE_ID_COUPLING");
    check(!moduleSources.includes(item.sourceSha256), "BENCHMARK_SOURCE_HASH_COUPLING");
    check(!moduleSources.includes(item.source), "BENCHMARK_FULL_SOURCE_COUPLING");
  }
} else {
  check(false, "IMMUTABLE_CORPUS_MISSING");
}

check(!/benchmark[s]?\s*[:=].*rule/i.test(moduleSources), "BENCHMARK_SWITCH_IN_ENGINE_SOURCE");
check(!/case[_-]?id/i.test(moduleSources), "CASE_ID_BRANCH_IN_ENGINE_SOURCE");
check(!/sourceSha256/.test(moduleSources), "CAMEL_CASE_BENCHMARK_HASH_IN_ENGINE_SOURCE");
check(!/process\.env|std::env/.test(moduleSources), "ENVIRONMENT_DEPENDENT_RULE_OUTCOME");

for (const token of [
  "IMPLEMENTED_AND_QUALIFIED_NOT_RUNTIME_WIRED",
  "RUNTIME_WIRING_NOT_AUTHORIZED_NOT_IMPLEMENTED",
  "PRODUCT_ACCEPTANCE_NOT_INFERRED",
  "P2_02_ACCEPTANCE_NOT_INFERRED",
  "UTF16_CODE_UNIT",
  "MEMORY_ONLY",
]) {
  check(documentation.includes(token), `DOCUMENTATION_TOKEN_MISSING:${token}`);
}

// The P3-02 task was not allowed to touch src-tauri/src/main.rs, so this
// contract once failed on any uncommitted main.rs edit (MAIN_RUNTIME_SOURCE_CHANGED).
// Later authorized tasks wired the engine into the app through p3_03_runtime,
// and a working-tree diff never protected committed code, so that task-scope
// rule was retired. The engine's own boundary is still checked above: no
// direct engine calls in main.rs, no network, no persistence, deterministic
// rules and no benchmark coupling.

const summary = {
  result: failures.length > 0 ? "RED" : notRun.length > 0 ? "NOT_RUN_PREREQUISITES" : "GREEN",
  assertions,
  failureCount: failures.length,
  failureIds: failures,
  notRunCount: notRun.length,
  notRunIds: notRun,
};
process.stdout.write(`${JSON.stringify(summary)}\n`);
// Failures exit 1. Checks that could not run exit 2: that is not a pass.
process.exitCode = failures.length > 0 ? 1 : notRun.length > 0 ? 2 : 0;
