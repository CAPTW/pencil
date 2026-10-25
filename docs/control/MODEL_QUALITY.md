# Pencil model quality improvement

Status: active comparison track, 2026-10-25. No weight training or new live calls
were performed for this UI change. This does not assert equal model quality.

## Current choice and trade-offs

| Model | Practical benefit | Cost / limitation | Pencil evidence |
|---|---|---|---|
| GPT-6 Luna | Hosted inference avoids a local GPU requirement | Network/account availability; CLI model availability differs from API availability | Earlier CLI account rejected the requested model; not a completed comparative baseline |
| Claude Haiku 5.5 | Hosted low-latency model; existing secure API integration works | Network, usage charges and provider data policy; writing quality must be measured | r6 desktop synthetic translation/rewrite/Accept passed; Chromium live attempt remains blocked |
| Gemma 4 12B QAT Q4_0 | Local weights, local text processing, controllable runtime | GPU/memory/storage and cold-load cost; quantization and conservative quality rejections | Existing scoped synthetic tests only; no same-corpus three-model ranking |

References checked 2026-10-25: [Luna](https://developers.openai.com/api/docs/models/gpt-6-luna),
[Haiku](https://platform.claude.com/docs/en/models/haiku-5-5/overview),
[Gemma](https://ai.google.dev/gemma/docs/core).
Vendor descriptions are not Pencil latency or language-quality measurements.

## Common acceptance standard

Use the same synthetic corpus, modes, target languages and source-bound Apply
checks for all models. Blind reviewers to model identity. Score meaning,
subject/object, negation/conditions/tense, register/honorifics, names/numbers and
naturalness separately (0 unacceptable, 1 major repair, 2 noticeable repair,
3 usable minor polish, 4 no repair). A critical semantic error always fails;
naturalness never compensates. Keep rejected outputs, failures and unavailable
models in coverage denominators. Never silently call another model or retry.

Record cold setup separately; recommendation time includes debounce, queue and
complete actionable publication. Targets stay 1s short recommendation, 3s for
3–4 sentences, 0.5s for explicit validated Accept. Compare p50/p95/max, failure
and refusal rates, and paired reviewer scores, not vendor token speed.

`scripts/evaluate-writing-quality.py` validates manually reviewed synthetic run
receipts and reports incomplete coverage instead of inventing a ranking. It
performs no inference and is not a semantic verifier. The seed corpus is
`tests/quality/writing-cases.json`; it is now a development set, not unseen holdout.
Create a fresh held-out set before selecting a prompt revision. No user text,
credentials or document history belongs in either set.

## Iteration and next actions

1. Complete the installed Chromium API path (tooling blocker retained).
2. Collect all three baselines with exact model, transport, prompt hash, package,
   hardware and cold/warm status; obtain missing account/cost authority as needed.
3. Change concise shared writing instructions or format adapters one variable at
   a time. Preserve existing guards and measure false refusals as well as errors.
4. Promote only with no critical regressions, full coverage and separate installed
   Accept evidence. Preserve failed baseline and rollback candidate.
5. Consider Gemma weight tuning only after a repeatable residual error is shown,
   with licensed synthetic training data, compute budget, held-out evaluation and
   model redistribution review. Hosted model weight tuning availability must be
   verified separately; prompt calibration is not weight fine-tuning.

This track is reviewed at each major UI/provider/release checkpoint. No automatic
background training, user-history collection or paid evaluation scheduler exists.
