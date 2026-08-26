import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const supportedKinds = new Set(["spelling", "spacing", "basic_grammar", "punctuation"]);

class ValidationError extends Error {
  constructor(code) {
    super(code);
    this.name = "ValidationError";
    this.code = code;
  }
}

function exactKeys(value, keys) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

function readUtf8(path) {
  const raw = readFileSync(path);
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(raw);
  } catch {
    throw new ValidationError("MALFORMED_UTF8");
  }
}

function readJson(path) {
  try {
    return JSON.parse(readUtf8(path));
  } catch (error) {
    if (error instanceof ValidationError) throw error;
    throw new ValidationError("MALFORMED_JSON");
  }
}

function readJsonl(path) {
  const body = readUtf8(path);
  if (!body.endsWith("\n") || body.includes("\r")) throw new ValidationError("CORPUS_ENCODING_INVALID");
  try {
    return body.trimEnd().split("\n").map((line) => JSON.parse(line));
  } catch {
    throw new ValidationError("MALFORMED_JSON");
  }
}

function utf16BoundaryValid(source, offset) {
  if (!Number.isInteger(offset) || offset < 0 || offset > source.length) return false;
  if (offset === 0 || offset === source.length) return true;
  const before = source.charCodeAt(offset - 1);
  const after = source.charCodeAt(offset);
  return !(before >= 0xd800 && before <= 0xdbff && after >= 0xdc00 && after <= 0xdfff);
}

function parseArguments(argv) {
  const options = { contract: null, corpus: null, predictions: null, out: null, selfTest: null, printSummary: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (["--contract", "--corpus", "--predictions", "--out", "--self-test"].includes(argument)) {
      const value = argv[index + 1];
      if (!value) throw new ValidationError(`${argument.slice(2).toUpperCase()}_MISSING`);
      const key = argument === "--self-test" ? "selfTest" : argument.slice(2);
      options[key] = argument === "--self-test" ? value : resolve(value);
      index += 1;
    } else if (argument === "--print-summary") {
      options.printSummary = true;
    } else {
      throw new ValidationError("UNKNOWN_ARGUMENT");
    }
  }
  if (!options.contract || !options.corpus || !options.out) throw new ValidationError("REQUIRED_ARGUMENT_MISSING");
  if (!options.selfTest && !options.predictions) throw new ValidationError("PREDICTIONS_REQUIRED");
  if (options.selfTest && !["perfect", "empty", "invalid", "allowed-alternative"].includes(options.selfTest)) {
    throw new ValidationError("SELF_TEST_UNSUPPORTED");
  }
  return options;
}

function validateContract(contract) {
  if (contract.schemaVersion !== 1) throw new ValidationError("CONTRACT_SCHEMA_UNSUPPORTED");
  if (contract.networkContract?.instantAnalyzerNetworkAllowed !== false) throw new ValidationError("CONTRACT_NETWORK_FORBIDDEN");
  if (contract.cacheContract?.diskPersistence !== false) throw new ValidationError("CONTRACT_PERSISTENCE_FORBIDDEN");
  return contract;
}

function validateCorpus(cases) {
  if (!Array.isArray(cases) || cases.length === 0) throw new ValidationError("CORPUS_EMPTY");
  const ids = new Set();
  for (const item of cases) {
    if (!exactKeys(item, ["schemaVersion", "caseId", "category", "language", "mode", "source", "sourceSha256", "expectedSuggestions", "allowedAlternatives", "expectedOutputs", "protectedSpansUtf16", "preserveLiterals", "tags"])) throw new ValidationError("CORPUS_CASE_SHAPE");
    if (item.schemaVersion !== 1 || item.mode !== "correction" || typeof item.caseId !== "string" || ids.has(item.caseId)) throw new ValidationError("CORPUS_CASE_ID");
    ids.add(item.caseId);
    if (typeof item.source !== "string" || !Array.isArray(item.expectedSuggestions) || !Array.isArray(item.allowedAlternatives) || !Array.isArray(item.expectedOutputs) || !Array.isArray(item.protectedSpansUtf16) || !Array.isArray(item.preserveLiterals) || !Array.isArray(item.tags)) throw new ValidationError("CORPUS_CASE_VALUE");
    let previousEnd = -1;
    for (const suggestion of [...item.expectedSuggestions].sort((left, right) => left.startUtf16 - right.startUtf16)) {
      if (!exactKeys(suggestion, ["startUtf16", "endUtf16", "replacement", "kind", "ruleCode"])) throw new ValidationError("CORPUS_SUGGESTION_SHAPE");
      if (!(suggestion.startUtf16 >= 0 && suggestion.startUtf16 < suggestion.endUtf16 && suggestion.endUtf16 <= item.source.length) || !utf16BoundaryValid(item.source, suggestion.startUtf16) || !utf16BoundaryValid(item.source, suggestion.endUtf16)) throw new ValidationError("CORPUS_RANGE_INVALID");
      if (suggestion.startUtf16 < previousEnd) throw new ValidationError("CORPUS_RANGE_OVERLAP");
      previousEnd = suggestion.endUtf16;
    }
  }
  return cases;
}

function validatePredictions(document, corpus) {
  if (!exactKeys(document, ["schemaVersion", "engine", "cases"])) throw new ValidationError("PREDICTIONS_TOP_LEVEL_SHAPE");
  if (document.schemaVersion !== 1) throw new ValidationError("UNSUPPORTED_SCHEMA_VERSION");
  if (!exactKeys(document.engine, ["id", "version", "kind", "coldStartMs", "peakRssMiB", "artifactBytes", "networkUsed", "persistentCacheUsed"])) throw new ValidationError("ENGINE_SHAPE");
  const engine = document.engine;
  if (typeof engine.id !== "string" || !engine.id || typeof engine.version !== "string" || !engine.version || typeof engine.kind !== "string") throw new ValidationError("ENGINE_IDENTITY_INVALID");
  for (const key of ["coldStartMs", "peakRssMiB", "artifactBytes"]) {
    if (typeof engine[key] !== "number" || !Number.isFinite(engine[key]) || engine[key] < 0) throw new ValidationError("ENGINE_RESOURCE_INVALID");
  }
  if (engine.networkUsed !== false) throw new ValidationError("NETWORK_USED_TRUE");
  if (engine.persistentCacheUsed !== false) throw new ValidationError("PERSISTENT_CACHE_USED_TRUE");
  if (!Array.isArray(document.cases)) throw new ValidationError("PREDICTION_CASES_INVALID");

  const corpusById = new Map(corpus.map((item) => [item.caseId, item]));
  const seenCases = new Set();
  for (const prediction of document.cases) {
    if (!exactKeys(prediction, ["caseId", "sourceSha256", "latencyMs", "suggestions"])) throw new ValidationError("PREDICTION_CASE_SHAPE");
    if (!corpusById.has(prediction.caseId)) throw new ValidationError("UNKNOWN_CASE");
    if (seenCases.has(prediction.caseId)) throw new ValidationError("DUPLICATE_CASE");
    seenCases.add(prediction.caseId);
    const item = corpusById.get(prediction.caseId);
    if (prediction.sourceSha256 !== item.sourceSha256) throw new ValidationError("SOURCE_HASH_MISMATCH");
    if (typeof prediction.latencyMs !== "number" || !Number.isFinite(prediction.latencyMs) || prediction.latencyMs < 0) throw new ValidationError("LATENCY_INVALID");
    if (!Array.isArray(prediction.suggestions)) throw new ValidationError("SUGGESTIONS_INVALID");
    const suggestionIds = new Set();
    const semantics = new Set();
    let previousEnd = -1;
    const sorted = [...prediction.suggestions].sort((left, right) => left.startUtf16 - right.startUtf16);
    for (const suggestion of sorted) {
      if (!exactKeys(suggestion, ["suggestionId", "startUtf16", "endUtf16", "replacement", "kind", "ruleCode", "engineVersion"])) throw new ValidationError("SUGGESTION_SHAPE");
      if (typeof suggestion.suggestionId !== "string" || !suggestion.suggestionId) throw new ValidationError("SUGGESTION_ID_INVALID");
      if (suggestionIds.has(suggestion.suggestionId)) throw new ValidationError("DUPLICATE_SUGGESTION_ID");
      suggestionIds.add(suggestion.suggestionId);
      if (!(suggestion.startUtf16 >= 0 && suggestion.startUtf16 < suggestion.endUtf16 && suggestion.endUtf16 <= item.source.length)) throw new ValidationError("RANGE_OUT_OF_BOUNDS_OR_ZERO");
      if (!utf16BoundaryValid(item.source, suggestion.startUtf16) || !utf16BoundaryValid(item.source, suggestion.endUtf16)) throw new ValidationError("SURROGATE_PAIR_SPLIT");
      if (suggestion.startUtf16 < previousEnd) throw new ValidationError("OVERLAPPING_SUGGESTIONS");
      previousEnd = suggestion.endUtf16;
      if (typeof suggestion.replacement !== "string" || suggestion.replacement === item.source.slice(suggestion.startUtf16, suggestion.endUtf16)) throw new ValidationError("NO_OP_REPLACEMENT");
      if (!supportedKinds.has(suggestion.kind)) throw new ValidationError("UNSUPPORTED_SUGGESTION_KIND");
      if (typeof suggestion.ruleCode !== "string" || typeof suggestion.engineVersion !== "string") throw new ValidationError("SUGGESTION_METADATA_INVALID");
      const semantic = `${suggestion.startUtf16}|${suggestion.endUtf16}|${suggestion.replacement}|${suggestion.kind}`;
      if (semantics.has(semantic)) throw new ValidationError("DUPLICATE_SEMANTIC_SUGGESTION");
      semantics.add(semantic);
    }
  }
  if (seenCases.size !== corpus.length) throw new ValidationError("MISSING_CASE");
  return document;
}

function applySuggestions(source, suggestions) {
  let output = source;
  for (const suggestion of [...suggestions].sort((left, right) => right.startUtf16 - left.startUtf16)) {
    output = `${output.slice(0, suggestion.startUtf16)}${suggestion.replacement}${output.slice(suggestion.endUtf16)}`;
  }
  return output;
}

function expectedMatch(item, expected, expectedIndex, predicted) {
  if (expected.startUtf16 !== predicted.startUtf16 || expected.endUtf16 !== predicted.endUtf16 || expected.kind !== predicted.kind) return false;
  if (expected.replacement === predicted.replacement) return true;
  return item.allowedAlternatives.some((alternative) => alternative.suggestionIndex === expectedIndex && alternative.replacement === predicted.replacement);
}

function percentile(values, probability) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * probability) - 1)];
}

function bucketFor(length) {
  if (length <= 512) return "utf16_1_512";
  if (length <= 2048) return "utf16_513_2048";
  return "utf16_2049_12000";
}

function evaluate(contract, corpus, predictions, warnings) {
  const byId = new Map(predictions.cases.map((item) => [item.caseId, item]));
  let expectedSuggestionCount = 0;
  let predictedSuggestionCount = 0;
  let exactTruePositive = 0;
  let falsePositive = 0;
  let falseNegative = 0;
  let caseOutputExactCount = 0;
  let noChangeCaseCount = 0;
  let noChangeFalsePositiveCaseCount = 0;
  let protectedSpanViolationCount = 0;
  let preserveLiteralViolationCount = 0;
  const latencies = [];
  const bucketValues = { utf16_1_512: [], utf16_513_2048: [], utf16_2049_12000: [] };

  for (const item of corpus) {
    const prediction = byId.get(item.caseId);
    expectedSuggestionCount += item.expectedSuggestions.length;
    predictedSuggestionCount += prediction.suggestions.length;
    const matchedExpected = new Set();
    for (const predicted of prediction.suggestions) {
      const expectedIndex = item.expectedSuggestions.findIndex((expected, index) => !matchedExpected.has(index) && expectedMatch(item, expected, index, predicted));
      if (expectedIndex >= 0) {
        matchedExpected.add(expectedIndex);
        exactTruePositive += 1;
      } else {
        falsePositive += 1;
      }
      for (const span of item.protectedSpansUtf16) {
        if (predicted.startUtf16 < span.endUtf16 && span.startUtf16 < predicted.endUtf16) protectedSpanViolationCount += 1;
      }
    }
    falseNegative += item.expectedSuggestions.length - matchedExpected.size;
    const output = applySuggestions(item.source, prediction.suggestions);
    if (item.expectedOutputs.includes(output)) caseOutputExactCount += 1;
    if (item.expectedSuggestions.length === 0) {
      noChangeCaseCount += 1;
      if (prediction.suggestions.length > 0) noChangeFalsePositiveCaseCount += 1;
    }
    for (const literal of item.preserveLiterals) {
      if (!output.includes(literal)) preserveLiteralViolationCount += 1;
    }
    latencies.push(prediction.latencyMs);
    bucketValues[bucketFor(item.source.length)].push(prediction.latencyMs);
  }

  const exactPrecision = predictedSuggestionCount === 0 ? 0 : exactTruePositive / predictedSuggestionCount;
  const exactRecall = expectedSuggestionCount === 0 ? 1 : exactTruePositive / expectedSuggestionCount;
  const exactF1 = exactPrecision + exactRecall === 0 ? 0 : (2 * exactPrecision * exactRecall) / (exactPrecision + exactRecall);
  const caseOutputExactRate = caseOutputExactCount / corpus.length;
  const noChangeFalsePositiveRate = noChangeCaseCount === 0 ? 0 : noChangeFalsePositiveCaseCount / noChangeCaseCount;
  const thresholds = contract.provisionalThresholds;
  const latencyByUtf16LengthBucket = Object.fromEntries(Object.entries(bucketValues).map(([name, values]) => [name, { caseCount: values.length, p50Ms: percentile(values, 0.50), p95Ms: percentile(values, 0.95), p99Ms: percentile(values, 0.99) }]));
  const safetyGatePassed = protectedSpanViolationCount === 0 && preserveLiteralViolationCount === 0 && predictions.engine.networkUsed === false && predictions.engine.persistentCacheUsed === false;
  const qualityGatePassed = exactPrecision >= thresholds.quality.exactPrecisionMin && exactRecall >= thresholds.quality.exactRecallMin && exactF1 >= thresholds.quality.exactF1Min && caseOutputExactRate >= thresholds.quality.caseOutputExactRateMin && noChangeFalsePositiveRate <= thresholds.quality.noChangeFalsePositiveRateMax;
  const latencyGatePassed = latencyByUtf16LengthBucket.utf16_1_512.p95Ms <= thresholds.latency.utf16_1_512_p95MsMax && (latencyByUtf16LengthBucket.utf16_513_2048.p95Ms === null || latencyByUtf16LengthBucket.utf16_513_2048.p95Ms <= thresholds.latency.utf16_513_2048_p95MsMax) && (latencyByUtf16LengthBucket.utf16_2049_12000.p95Ms === null || latencyByUtf16LengthBucket.utf16_2049_12000.p95Ms <= thresholds.latency.utf16_2049_12000_p95MsMax) && predictions.engine.coldStartMs <= thresholds.latency.coldStartMsMax;
  const resourceGatePassed = predictions.engine.peakRssMiB <= thresholds.resources.peakRssMiBMax && predictions.engine.artifactBytes <= thresholds.resources.additionalArtifactBytesMax;

  return {
    schemaVersion: 1,
    benchmarkId: "instant-selection-synthetic-v1",
    engine: predictions.engine,
    productEngineBenchmark: "NOT_EXECUTED",
    metrics: {
      caseCount: corpus.length,
      expectedSuggestionCount,
      predictedSuggestionCount,
      exactTruePositive,
      falsePositive,
      falseNegative,
      exactPrecision,
      exactRecall,
      exactF1,
      caseOutputExactCount,
      caseOutputExactRate,
      noChangeCaseCount,
      noChangeFalsePositiveCaseCount,
      noChangeFalsePositiveRate,
      protectedSpanViolationCount,
      preserveLiteralViolationCount,
      invalidSuggestionCount: 0,
      overlapViolationCount: 0,
      sourceIdentityMismatchCount: 0,
      latencyP50Ms: percentile(latencies, 0.50),
      latencyP95Ms: percentile(latencies, 0.95),
      latencyP99Ms: percentile(latencies, 0.99),
      latencyByUtf16LengthBucket,
      coldStartMs: predictions.engine.coldStartMs,
      peakRssMiB: predictions.engine.peakRssMiB,
      artifactBytes: predictions.engine.artifactBytes,
      networkUsed: predictions.engine.networkUsed,
      persistentCacheUsed: predictions.engine.persistentCacheUsed,
      safetyGatePassed,
      qualityGatePassed,
      latencyGatePassed,
      resourceGatePassed,
      overallQualified: safetyGatePassed && qualityGatePassed && latencyGatePassed && resourceGatePassed,
      warnings,
    },
  };
}

function predictionsFromCorpus(corpus, empty = false) {
  const engine = { id: empty ? "synthetic-empty" : "synthetic-perfect-oracle", version: "1", kind: "SELF_TEST_NOT_PRODUCT_ENGINE", coldStartMs: 100, peakRssMiB: 16, artifactBytes: 0, networkUsed: false, persistentCacheUsed: false };
  return {
    schemaVersion: 1,
    engine,
    cases: corpus.map((item) => ({
      caseId: item.caseId,
      sourceSha256: item.sourceSha256,
      latencyMs: 10 + (item.source.length % 7),
      suggestions: empty ? [] : item.expectedSuggestions.map((suggestion, index) => ({ suggestionId: `${item.caseId}-s${index + 1}`, startUtf16: suggestion.startUtf16, endUtf16: suggestion.endUtf16, replacement: suggestion.replacement, kind: suggestion.kind, ruleCode: suggestion.ruleCode, engineVersion: "1" })),
    })),
  };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function firstSuggestedCase(document) {
  return document.cases.find((item) => item.suggestions.length > 0);
}

function invalidFixtureResult(contract, corpus) {
  const base = predictionsFromCorpus(corpus);
  const fixtures = [
    ["duplicate_case", (value) => value.cases.push(clone(value.cases[0]))],
    ["unknown_case", (value) => { value.cases[0].caseId = "unknown-case"; }],
    ["missing_case", (value) => { value.cases.pop(); }],
    ["source_hash_mismatch", (value) => { value.cases[0].sourceSha256 = "0".repeat(64); }],
    ["out_of_bounds_range", (value) => { const item = firstSuggestedCase(value); item.suggestions[0].endUtf16 = 999999; }],
    ["zero_length_range", (value) => { const item = firstSuggestedCase(value); item.suggestions[0].endUtf16 = item.suggestions[0].startUtf16; }],
    ["surrogate_split", (value) => { const index = corpus.findIndex((item) => item.source.includes("😀")); const emoji = corpus[index].source.indexOf("😀"); value.cases[index].suggestions = [{ suggestionId: "surrogate", startUtf16: emoji + 1, endUtf16: emoji + 2, replacement: "x", kind: "spelling", ruleCode: "INVALID", engineVersion: "1" }]; }],
    ["overlapping_suggestions", (value) => { const item = firstSuggestedCase(value); item.suggestions.push({ ...item.suggestions[0], suggestionId: "overlap", replacement: `${item.suggestions[0].replacement}!` }); }],
    ["duplicate_suggestion_id", (value) => { const item = firstSuggestedCase(value); item.suggestions.push({ ...item.suggestions[0] }); }],
    ["duplicate_semantic_suggestion", (value) => { const item = firstSuggestedCase(value); item.suggestions.push({ ...item.suggestions[0], suggestionId: "semantic-duplicate" }); }],
    ["no_op_replacement", (value) => { const index = value.cases.findIndex((item) => item.suggestions.length); const suggestion = value.cases[index].suggestions[0]; suggestion.replacement = corpus[index].source.slice(suggestion.startUtf16, suggestion.endUtf16); }],
    ["negative_latency", (value) => { value.cases[0].latencyMs = -1; }],
    ["non_finite_latency", (value) => { value.cases[0].latencyMs = "NaN"; }],
    ["network_used", (value) => { value.engine.networkUsed = true; }],
    ["persistent_cache_used", (value) => { value.engine.persistentCacheUsed = true; }],
    ["unsupported_schema_version", (value) => { value.schemaVersion = 2; }],
  ];
  const rejected = [];
  for (const [id, mutate] of fixtures) {
    const value = clone(base);
    mutate(value);
    try {
      validatePredictions(value, corpus);
    } catch (error) {
      if (error instanceof ValidationError) rejected.push(id);
    }
  }
  let malformedJsonRejected = false;
  try {
    JSON.parse("{");
  } catch {
    malformedJsonRejected = true;
  }
  if (!malformedJsonRejected) throw new ValidationError("MALFORMED_JSON_NOT_REJECTED");
  if (rejected.length !== fixtures.length) throw new ValidationError("INVALID_FIXTURE_NOT_REJECTED");
  return {
    schemaVersion: 1,
    selfTest: "invalid",
    invalidFixtureCount: fixtures.length + 1,
    rejectedFixtureCount: rejected.length + 1,
    rejectedFixtureIds: [...rejected, "malformed_json"],
    malformedJsonRejected,
    productEngineBenchmark: "NOT_EXECUTED",
    contractAuthority: contract.currentImplementationAuthority,
  };
}

function run(options) {
  const contract = validateContract(readJson(options.contract));
  const corpus = validateCorpus(readJsonl(options.corpus));
  if (options.selfTest === "invalid") return invalidFixtureResult(contract, corpus);

  let predictions;
  let warnings;
  if (options.selfTest === "perfect" || options.selfTest === "allowed-alternative") {
    predictions = predictionsFromCorpus(corpus);
    warnings = ["SYNTHETIC_ORACLE_NOT_PRODUCT_EVIDENCE"];
    if (options.selfTest === "allowed-alternative") {
      const index = corpus.findIndex((item) => item.allowedAlternatives.length > 0);
      if (index < 0) throw new ValidationError("ALLOWED_ALTERNATIVE_MISSING");
      predictions.cases[index].suggestions[0].replacement = corpus[index].allowedAlternatives[0].replacement;
      predictions.engine.id = "synthetic-allowed-alternative";
    }
  } else if (options.selfTest === "empty") {
    predictions = predictionsFromCorpus(corpus, true);
    warnings = ["SYNTHETIC_EMPTY_ENGINE_NOT_PRODUCT_EVIDENCE"];
  } else {
    predictions = readJson(options.predictions);
    warnings = [];
  }
  validatePredictions(predictions, corpus);
  return evaluate(contract, corpus, predictions, warnings);
}

function main() {
  const options = parseArguments(process.argv.slice(2));
  const report = run(options);
  const output = `${JSON.stringify(report, null, 2)}\n`;
  writeFileSync(options.out, output, { encoding: "utf8", flag: "wx" });
  if (options.printSummary) {
    const summary = report.metrics
      ? { result: "PASS", selfTest: options.selfTest, caseCount: report.metrics.caseCount, overallQualified: report.metrics.overallQualified, productEngineBenchmark: report.productEngineBenchmark }
      : { result: "PASS", selfTest: options.selfTest, invalidFixtureCount: report.invalidFixtureCount, rejectedFixtureCount: report.rejectedFixtureCount, productEngineBenchmark: report.productEngineBenchmark };
    process.stdout.write(`${JSON.stringify(summary)}\n`);
  }
}

try {
  main();
} catch (error) {
  const code = error instanceof ValidationError ? error.code : "UNEXPECTED_EVALUATOR_ERROR";
  process.stderr.write(`${code}\n`);
  process.exitCode = 1;
}
