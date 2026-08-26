# Instant Selection synthetic benchmark v1

This directory defines a deterministic, synthetic-only qualification baseline for a future Codex Pencil Instant Analyzer. It does not contain a production analyzer, a local model, user text, clipboard text, production-store values, or model output.

## Files

- `baseline-contract.v1.json` freezes architecture, privacy, reconciliation, candidate, and provisional qualification contracts.
- `corpus.schema.v1.json` defines one JSONL corpus case.
- `predictions.schema.v1.json` defines candidate-engine predictions.
- `corpus.synthetic.v1.jsonl` contains exactly 120 deterministic synthetic cases.
- `corpus-manifest.v1.json` binds the checked-in corpus to its generator and schema.

## Reproduce the corpus

Run from the repository root with an output path outside the repository:

```powershell
node scripts/generate-instant-selection-corpus.mjs --out D:\owned-temp\corpus.synthetic.v1.jsonl
```

The generator refuses to overwrite an existing path unless `--force` is explicit. Repeated runs produce byte-identical UTF-8, no-BOM, LF-only output with a final LF. `--stdout` emits the same bytes without creating a file.

## Evaluate predictions

```powershell
node scripts/evaluate-instant-selection-benchmark.mjs --contract benchmarks/instant-selection/baseline-contract.v1.json --corpus benchmarks/instant-selection/corpus.synthetic.v1.jsonl --predictions D:\owned-temp\predictions.json --out D:\owned-temp\report.json --print-summary
```

The evaluator uses Node built-ins only. It rejects missing, duplicate, unknown, stale-source, malformed-range, surrogate-splitting, overlapping, duplicate-semantic, no-op, networked, and persistent-cache predictions before scoring. Suggestions are applied in descending `startUtf16` order.

Self-tests are deterministic and are not product-engine evidence:

```powershell
node scripts/evaluate-instant-selection-benchmark.mjs --contract benchmarks/instant-selection/baseline-contract.v1.json --corpus benchmarks/instant-selection/corpus.synthetic.v1.jsonl --self-test perfect --out D:\owned-temp\perfect.json
node scripts/evaluate-instant-selection-benchmark.mjs --contract benchmarks/instant-selection/baseline-contract.v1.json --corpus benchmarks/instant-selection/corpus.synthetic.v1.jsonl --self-test empty --out D:\owned-temp\empty.json
node scripts/evaluate-instant-selection-benchmark.mjs --contract benchmarks/instant-selection/baseline-contract.v1.json --corpus benchmarks/instant-selection/corpus.synthetic.v1.jsonl --self-test invalid --out D:\owned-temp\invalid.json
```

## Interpretation

Safety gates are zero-tolerance. Quality targets intentionally favor precision over recall. Latency is bucketed by source UTF-16 length. Resource fields are supplied by the candidate-engine harness and are validated, not independently measured by this evaluator.

`PRODUCT_ENGINE_BENCHMARK_NOT_EXECUTED`

The current cloud Deep Analyzer remains the accepted analysis path. This baseline only prepares a future, correction-only, memory-only, non-networked Instant Analyzer qualification Gate.
