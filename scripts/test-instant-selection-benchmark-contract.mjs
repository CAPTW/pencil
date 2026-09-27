import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve, sep } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const authorizedPaths = [
  "docs/P3_01_INSTANT_SELECTION_ENGINE_ARCHITECTURE_PRIVACY_BENCHMARK_BASELINE.md",
  "benchmarks/instant-selection/README.md",
  "benchmarks/instant-selection/baseline-contract.v1.json",
  "benchmarks/instant-selection/corpus.schema.v1.json",
  "benchmarks/instant-selection/predictions.schema.v1.json",
  "benchmarks/instant-selection/corpus.synthetic.v1.jsonl",
  "benchmarks/instant-selection/corpus-manifest.v1.json",
  "scripts/generate-instant-selection-corpus.mjs",
  "scripts/evaluate-instant-selection-benchmark.mjs",
  "scripts/test-instant-selection-benchmark-contract.mjs",
  // Added later by the authorized P3-02 engine foundation (fda1d08) in the same
  // namespace. The set stays closed: any other new file here still fails.
  "scripts/run-instant-selection-product-benchmark.ps1",
  "scripts/test-instant-selection-engine-contract.mjs",
];

const requiredBaselineFiles = [
  ["MISSING_ARCHITECTURE_DOCUMENT", authorizedPaths[0]],
  ["MISSING_BASELINE_CONTRACT", authorizedPaths[2]],
  ["MISSING_CORPUS_SCHEMA", authorizedPaths[3]],
  ["MISSING_PREDICTIONS_SCHEMA", authorizedPaths[4]],
  ["MISSING_GENERATOR", authorizedPaths[7]],
  ["MISSING_CORPUS", authorizedPaths[5]],
  ["MISSING_CORPUS_MANIFEST", authorizedPaths[6]],
  ["MISSING_EVALUATOR", authorizedPaths[8]],
  ["MISSING_BENCHMARK_README", authorizedPaths[1]],
];

const assertions = [];
function check(id, condition, detail = null) {
  assertions.push({ id, passed: Boolean(condition), detail: condition ? null : detail });
}

function absolute(relativePath) {
  return join(repositoryRoot, ...relativePath.split("/"));
}

function bytes(relativePath) {
  return readFileSync(absolute(relativePath));
}

function text(relativePath) {
  return bytes(relativePath).toString("utf8");
}

function json(relativePath) {
  return JSON.parse(text(relativePath));
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function containsPersonalLocation(value) {
  const usersDirectory = ["Us", "ers"].join("");
  const profileVariable = ["USER", "PROFILE"].join("");
  const cloudFolder = ["One", "Drive"].join("");
  const pattern = new RegExp(`(?:[A-Za-z]:[\\\\/]${usersDirectory}[\\\\/]|/${usersDirectory}/|${profileVariable}|${cloudFolder}[\\\\/])`, "i");
  return pattern.test(value);
}

function stableStringify(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
}

function utf16BoundaryValid(source, offset) {
  if (!Number.isInteger(offset) || offset < 0 || offset > source.length) return false;
  if (offset === 0 || offset === source.length) return true;
  const before = source.charCodeAt(offset - 1);
  const after = source.charCodeAt(offset);
  return !(before >= 0xd800 && before <= 0xdbff && after >= 0xdc00 && after <= 0xdfff);
}

function applySuggestions(source, suggestions) {
  let output = source;
  for (const suggestion of [...suggestions].sort((left, right) => right.startUtf16 - left.startUtf16)) {
    output = `${output.slice(0, suggestion.startUtf16)}${suggestion.replacement}${output.slice(suggestion.endUtf16)}`;
  }
  return output;
}

function listRegularFiles(root) {
  if (!existsSync(root)) return [];
  const result = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const full = join(root, entry.name);
    if (entry.isDirectory()) result.push(...listRegularFiles(full));
    else if (entry.isFile()) result.push(relative(repositoryRoot, full).split(sep).join("/"));
  }
  return result;
}

for (const [id, relativePath] of requiredBaselineFiles) {
  check(id, existsSync(absolute(relativePath)), relativePath);
}

const evaluatorAvailable = existsSync(absolute(authorizedPaths[8]));
check("MISSING_PERFECT_ORACLE_SELF_TEST", evaluatorAvailable, authorizedPaths[8]);

const missingRequired = assertions.some((assertion) => !assertion.passed);
if (missingRequired) {
  const summary = {
    result: "RED",
    assertionCount: assertions.length,
    failureCount: assertions.filter((assertion) => !assertion.passed).length,
    failureIds: assertions.filter((assertion) => !assertion.passed).map((assertion) => assertion.id),
    syntaxOrParserError: false,
  };
  process.stdout.write(`${JSON.stringify(summary)}\n`);
  process.exitCode = 1;
} else {
  const workRoot = process.env.P3_01_TEST_ROOT
    ? resolve(process.env.P3_01_TEST_ROOT)
    : mkdtempSync(join(tmpdir(), "codex-pencil-p3-01-contract-"));
  const ownsWorkRoot = !process.env.P3_01_TEST_ROOT;
  const runRoot = mkdtempSync(join(workRoot, "contract-"));
  try {
    const benchmarkFiles = listRegularFiles(absolute("benchmarks/instant-selection"));
    const p3Docs = listRegularFiles(absolute("docs")).filter((path) => path.includes("P3_01_INSTANT_SELECTION"));
    const p3Scripts = listRegularFiles(absolute("scripts")).filter((path) => path.includes("instant-selection"));
    const observedP3 = [...benchmarkFiles, ...p3Docs, ...p3Scripts].sort();
    check("AUTHORIZED_FILE_SET", stableStringify(observedP3) === stableStringify([...authorizedPaths].sort()), observedP3);

    for (const relativePath of authorizedPaths) {
      const value = bytes(relativePath);
      const body = value.toString("utf8");
      const id = relativePath.replace(/[^A-Za-z0-9]+/g, "_").toUpperCase();
      check(`REGULAR_FILE_${id}`, statSync(absolute(relativePath)).isFile(), relativePath);
      check(`UTF8_NO_BOM_${id}`, !(value[0] === 0xef && value[1] === 0xbb && value[2] === 0xbf), relativePath);
      check(`LF_ONLY_${id}`, !body.includes("\r"), relativePath);
      check(`FINAL_LF_${id}`, body.endsWith("\n"), relativePath);
      check(`NO_USER_PATH_${id}`, !containsPersonalLocation(body), relativePath);
    }

    for (const relativePath of authorizedPaths.filter((path) => path.endsWith(".mjs"))) {
      const imports = [...text(relativePath).matchAll(/from\s+["']([^"']+)["']/g)].map((match) => match[1]);
      check(`NODE_BUILTINS_ONLY_${relativePath}`, imports.every((specifier) => specifier.startsWith("node:")), imports);
    }

    const contract = json(authorizedPaths[2]);
    const requiredTopLevelKeys = [
      "schemaVersion", "taskId", "currentImplementationAuthority", "currentProductAcceptanceAuthority",
      "threeCommitProductAcceptance", "p2_02CurrentImplementationPresence", "p2_02CurrentAcceptance",
      "historicalP2_02Disposition", "projectSourceStatus", "triggerContract", "modeCapabilityMatrix",
      "rangeContract", "sourceIdentityContract", "suggestionContract", "cacheContract",
      "reconciliationContract", "userEditProtectionContract", "privacyContract", "networkContract",
      "loggingContract", "benchmarkContract", "provisionalThresholds", "engineCandidateMatrix",
      "recommendedP3_02Strategy", "provisionalNextTask", "productEngineBenchmarkExecuted", "automaticNextTask",
    ];
    check("BASELINE_TOP_LEVEL_KEYS", stableStringify(Object.keys(contract).sort()) === stableStringify(requiredTopLevelKeys.sort()), Object.keys(contract));
    check("IMPLEMENTATION_AUTHORITY", contract.currentImplementationAuthority === "fb988fe810bbb09804a59e40161e6d1aef1bca8b");
    check("PRODUCT_AUTHORITY", contract.currentProductAcceptanceAuthority === "PASS_GRAMMAR_P1_03_PERSONAL_MVP_ACCEPTANCE");
    check("THREE_COMMIT_ACCEPTANCE", contract.threeCommitProductAcceptance === "NOT_INFERRED");
    check("P2_02_PRESENCE", contract.p2_02CurrentImplementationPresence === "COMMITTED_POST_CLOSURE");
    check("P2_02_ACCEPTANCE", contract.p2_02CurrentAcceptance === "NOT_INDEPENDENTLY_ACCEPTED");
    check("HISTORICAL_P2_02", contract.historicalP2_02Disposition === "DEFERRED_NOT_ACCEPTED_NOT_COMMITTED");
    check("PROJECT_SOURCE_STALE", contract.projectSourceStatus === "HISTORICALLY_VALID_BUT_STALE_FOR_CURRENT_IMPLEMENTATION");
    check("TRIGGER_SELECTION_SHORTCUT_ONLY", contract.triggerContract.userSelectedTextOnly === true && contract.triggerContract.explicitShortcutOnly === true);
    check("NO_CONTINUOUS_MONITORING", contract.triggerContract.continuousMonitoring === false && contract.privacyContract.continuousCollection === false);
    check("MODE_CAPABILITY_MATRIX", stableStringify(contract.modeCapabilityMatrix) === stableStringify({ correction: "INSTANT_PLUS_DEEP", natural: "DEEP_ONLY", professional: "DEEP_ONLY", concise: "DEEP_ONLY", summary: "DEEP_ONLY", translation: "DEEP_ONLY" }));
    check("UTF16_HALF_OPEN_RANGE", contract.rangeContract.unit === "UTF16_CODE_UNIT" && contract.rangeContract.interval === "HALF_OPEN" && contract.rangeContract.fields.start === "startUtf16" && contract.rangeContract.fields.end === "endUtf16");
    check("SOURCE_IDENTITY_FIELDS", stableStringify(contract.sourceIdentityContract.fields) === stableStringify(["sessionId", "generation", "intentRevision", "terminologyRevision", "sourceSha256", "analyzerId", "analyzerVersion", "mode"]));
    check("SUGGESTION_KINDS", stableStringify(contract.suggestionContract.allowedInitialKinds) === stableStringify(["spelling", "spacing", "basic_grammar", "punctuation"]));
    check("CACHE_MEMORY_SESSION_ONLY", contract.cacheContract.persistence === "MEMORY_ONLY" && contract.cacheContract.scope === "ACTIVE_CAPTURE_SESSION_ONLY" && contract.cacheContract.maxActiveSourceIdentities === 1);
    check("NO_NETWORK", contract.networkContract.instantAnalyzerNetworkAllowed === false);
    check("NO_PERSISTENCE", contract.cacheContract.diskPersistence === false && contract.loggingContract.contentPersistenceAllowed === false);
    check("INLINE_ASSIST_DEFERRED", contract.triggerContract.inlineAssistAuthorized === false);
    check("EDITOR_ADAPTER_DEFERRED", contract.triggerContract.editorAdaptersAuthorized === false);
    check("NO_SILENT_DEEP_OVERWRITE", contract.reconciliationContract.deepMaySilentlyReplaceInstant === false);
    check("USER_EDIT_PROTECTION", contract.userEditProtectionContract.deepMayOverwriteUserEdit === false && contract.userEditProtectionContract.deepAfterUserEdit === "OPTIONAL_ALTERNATE_ONLY");

    const corpusText = text(authorizedPaths[5]);
    const corpus = corpusText.trimEnd().split("\n").map((line) => JSON.parse(line));
    const expectedCategoryCounts = {
      KO_SPACING: 16, KO_TYPO: 12, KO_BASIC_GRAMMAR: 12, EN_SPELLING: 12,
      EN_BASIC_GRAMMAR: 12, MIXED_LANGUAGE: 8, PROTECTED_TERMINOLOGY: 12,
      STRUCTURE_PRESERVATION: 12, NO_CHANGE: 12, ADVERSARIAL_PROMPT_LIKE: 12,
    };
    const actualCategoryCounts = Object.fromEntries(Object.keys(expectedCategoryCounts).map((category) => [category, corpus.filter((item) => item.category === category).length]));
    check("CORPUS_CASE_COUNT", corpus.length === 120, corpus.length);
    check("CORPUS_CATEGORY_COUNTS", stableStringify(actualCategoryCounts) === stableStringify(expectedCategoryCounts), actualCategoryCounts);
    check("CORPUS_UNIQUE_IDS", new Set(corpus.map((item) => item.caseId)).size === corpus.length);
    check("CORPUS_MODE_CORRECTION", corpus.every((item) => item.mode === "correction"));
    check("CORPUS_SOURCE_HASHES", corpus.every((item) => item.sourceSha256 === sha256(Buffer.from(item.source, "utf8"))));
    check("CORPUS_NO_CHANGE_ZERO", corpus.filter((item) => item.category === "NO_CHANGE").every((item) => item.expectedSuggestions.length === 0));
    check("CORPUS_RANGES_VALID", corpus.every((item) => item.expectedSuggestions.every((suggestion) => suggestion.startUtf16 >= 0 && suggestion.startUtf16 < suggestion.endUtf16 && suggestion.endUtf16 <= item.source.length && utf16BoundaryValid(item.source, suggestion.startUtf16) && utf16BoundaryValid(item.source, suggestion.endUtf16))));
    check("CORPUS_RANGES_NONOVERLAP", corpus.every((item) => item.expectedSuggestions.every((left, index) => item.expectedSuggestions.every((right, other) => index === other || left.endUtf16 <= right.startUtf16 || right.endUtf16 <= left.startUtf16))));
    check("CORPUS_OUTPUT_RECONSTRUCTION", corpus.every((item) => item.expectedOutputs.includes(applySuggestions(item.source, item.expectedSuggestions))));
    check("CORPUS_PROTECTED_SPANS", corpus.every((item) => item.expectedSuggestions.every((suggestion) => item.protectedSpansUtf16.every((span) => suggestion.endUtf16 <= span.startUtf16 || span.endUtf16 <= suggestion.startUtf16))));
    check("CORPUS_PROMPT_LIKE_DATA", corpus.filter((item) => item.category === "ADVERSARIAL_PROMPT_LIKE").every((item) => item.tags.includes("treat_as_data")));

    const generatedA = join(runRoot, "corpus-a.jsonl");
    const generatedB = join(runRoot, "corpus-b.jsonl");
    for (const output of [generatedA, generatedB]) {
      const generated = spawnSync(process.execPath, [absolute(authorizedPaths[7]), "--out", output], { encoding: "utf8" });
      check(`GENERATOR_EXIT_${output.endsWith("a.jsonl") ? "A" : "B"}`, generated.status === 0, generated.stderr);
    }
    check("GENERATOR_REPRODUCIBLE", readFileSync(generatedA).equals(readFileSync(generatedB)) && readFileSync(generatedA).equals(bytes(authorizedPaths[5])));

    const manifest = json(authorizedPaths[6]);
    check("CORPUS_MANIFEST_COUNT", manifest.caseCount === 120 && stableStringify(manifest.categoryCounts) === stableStringify(expectedCategoryCounts));
    check("CORPUS_MANIFEST_HASH", manifest.corpusBytes === bytes(authorizedPaths[5]).length && manifest.corpusSha256 === sha256(bytes(authorizedPaths[5])));
    check("CORPUS_MANIFEST_PROVENANCE", manifest.syntheticOnly === true && manifest.externalSourceCount === 0 && manifest.containsUserContent === false && manifest.containsProductionContent === false && manifest.containsClipboardContent === false);
    check("CORPUS_MANIFEST_ENCODING", manifest.lineEnding === "LF" && manifest.bom === false && manifest.finalLf === true);
    check("CORPUS_SCHEMA", json(authorizedPaths[3]).$id === "codex-pencil://instant-selection/corpus.schema.v1" && json(authorizedPaths[3]).additionalProperties === false);
    check("PREDICTIONS_SCHEMA", json(authorizedPaths[4]).$id === "codex-pencil://instant-selection/predictions.schema.v1" && json(authorizedPaths[4]).additionalProperties === false);

    function runSelfTest(name, suffix) {
      const output = join(runRoot, `${suffix}.json`);
      const run = spawnSync(process.execPath, [absolute(authorizedPaths[8]), "--contract", absolute(authorizedPaths[2]), "--corpus", absolute(authorizedPaths[5]), "--self-test", name, "--out", output, "--print-summary"], { encoding: "utf8" });
      return { output, run, report: run.status === 0 && existsSync(output) ? JSON.parse(readFileSync(output, "utf8")) : null };
    }

    const perfectRuns = [runSelfTest("perfect", "perfect-1"), runSelfTest("perfect", "perfect-2"), runSelfTest("perfect", "perfect-3")];
    check("PERFECT_SELF_TEST_EXIT", perfectRuns.every(({ run }) => run.status === 0), perfectRuns.map(({ run }) => run.stderr));
    const perfect = perfectRuns[0].report;
    check("PERFECT_SELF_TEST_METRICS", perfect?.metrics.exactPrecision === 1 && perfect?.metrics.exactRecall === 1 && perfect?.metrics.exactF1 === 1 && perfect?.metrics.caseOutputExactRate === 1 && perfect?.metrics.noChangeFalsePositiveRate === 0 && perfect?.metrics.safetyGatePassed === true && perfect?.metrics.qualityGatePassed === true);
    check("DETERMINISTIC_THREE_RUN_REPORT", perfectRuns.every(({ output }) => readFileSync(output).equals(readFileSync(perfectRuns[0].output))));

    const empty = runSelfTest("empty", "empty");
    check("EMPTY_SELF_TEST_EXIT", empty.run.status === 0, empty.run.stderr);
    check("EMPTY_SELF_TEST_METRICS", empty.report?.metrics.predictedSuggestionCount === 0 && empty.report?.metrics.exactTruePositive === 0 && empty.report?.metrics.falsePositive === 0 && empty.report?.metrics.exactRecall === 0 && empty.report?.metrics.noChangeFalsePositiveRate === 0 && empty.report?.metrics.safetyGatePassed === true && empty.report?.metrics.qualityGatePassed === false && empty.report?.metrics.overallQualified === false);

    const invalid = runSelfTest("invalid", "invalid");
    check("INVALID_SELF_TEST_EXIT", invalid.run.status === 0, invalid.run.stderr);
    check("INVALID_FIXTURES_REJECTED", invalid.report?.invalidFixtureCount === 17 && invalid.report?.rejectedFixtureCount === 17, invalid.report);

    const alternative = runSelfTest("allowed-alternative", "allowed-alternative");
    check("ALLOWED_ALTERNATIVE", alternative.run.status === 0 && alternative.report?.metrics.caseOutputExactRate === 1 && alternative.report?.metrics.exactPrecision === 1 && alternative.report?.metrics.exactRecall === 1, alternative.run.stderr);

    check("SAFETY_THRESHOLDS", contract.provisionalThresholds.safety.invalidSuggestionCount === 0 && contract.provisionalThresholds.safety.protectedSpanViolationCount === 0 && contract.provisionalThresholds.safety.networkUsed === false && contract.provisionalThresholds.safety.persistentCacheUsed === false);
    check("QUALITY_THRESHOLDS", contract.provisionalThresholds.quality.exactPrecisionMin === 0.98 && contract.provisionalThresholds.quality.exactRecallMin === 0.60 && contract.provisionalThresholds.quality.exactF1Min === 0.74 && contract.provisionalThresholds.quality.caseOutputExactRateMin === 0.70 && contract.provisionalThresholds.quality.noChangeFalsePositiveRateMax === 0.01);
    check("LATENCY_THRESHOLDS", contract.provisionalThresholds.latency.utf16_1_512_p95MsMax === 250 && contract.provisionalThresholds.latency.utf16_513_2048_p95MsMax === 500 && contract.provisionalThresholds.latency.utf16_2049_12000_p95MsMax === 750 && contract.provisionalThresholds.latency.coldStartMsMax === 1000);
    check("RESOURCE_THRESHOLDS", contract.provisionalThresholds.resources.peakRssMiBMax === 384 && contract.provisionalThresholds.resources.additionalArtifactBytesMax === 300 * 1024 * 1024);
    check("PRODUCT_ENGINE_NOT_EXECUTED", contract.productEngineBenchmarkExecuted === false && contract.benchmarkContract.productEngineBenchmark === "NOT_EXECUTED");
    check("ONE_RECOMMENDED_STRATEGY", ["DETERMINISTIC_RULE_ENGINE_FIRST", "WINDOWS_SPELLCHECK_API_FIRST", "HYBRID_RULE_AND_WINDOWS_API", "SMALL_LOCAL_MODEL_SPIKE"].includes(contract.recommendedP3_02Strategy));
    check("PROVISIONAL_NEXT_TASK", contract.provisionalNextTask === "GRAMMAR-P3-02-INSTANT-SELECTION-ENGINE-FOUNDATION-IMPLEMENTATION-AND-QUALIFICATION");
    check("AUTOMATIC_NEXT_TASK_NULL", contract.automaticNextTask === null);

    const architecture = text(authorizedPaths[0]);
    check("ARCHITECTURE_MAJOR_SECTIONS", Array.from({ length: 30 }, (_, index) => `${index + 1}. `).every((prefix) => architecture.includes(`## ${prefix}`)));
    check("ARCHITECTURE_SEAM_VERDICT", architecture.includes("CURRENT_CAPTURE_AND_APPLY_PATH_REUSED_WITHOUT_REDESIGN"));
    check("ARCHITECTURE_INLINE_TOKEN", architecture.includes("INLINE_ASSIST_DEFERRED_NOT_AUTHORIZED_BY_P3_01"));
    check("ARCHITECTURE_PRODUCT_STATUS", architecture.includes("PRODUCT_ENGINE_BENCHMARK_NOT_EXECUTED"));

    const failures = assertions.filter((assertion) => !assertion.passed);
    const summary = {
      result: failures.length === 0 ? "GREEN" : "FAIL",
      assertionCount: assertions.length,
      failureCount: failures.length,
      failureIds: failures.map((assertion) => assertion.id),
      caseCount: corpus.length,
      categoryCounts: actualCategoryCounts,
      perfectReportSha256: perfectRuns[0].report ? sha256(readFileSync(perfectRuns[0].output)) : null,
      syntaxOrParserError: false,
    };
    process.stdout.write(`${JSON.stringify(summary)}\n`);
    if (failures.length > 0) process.exitCode = 1;
  } catch (error) {
    const summary = {
      result: "FAIL",
      assertionCount: assertions.length,
      failureCount: assertions.filter((assertion) => !assertion.passed).length + 1,
      failureIds: [...assertions.filter((assertion) => !assertion.passed).map((assertion) => assertion.id), "UNEXPECTED_TEST_EXCEPTION"],
      syntaxOrParserError: false,
      exceptionName: error instanceof Error ? error.name : "UnknownError",
    };
    process.stdout.write(`${JSON.stringify(summary)}\n`);
    process.exitCode = 1;
  } finally {
    rmSync(runRoot, { recursive: true, force: true });
    if (ownsWorkRoot) rmSync(workRoot, { recursive: true, force: true });
  }
}
