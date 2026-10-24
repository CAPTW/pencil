# Codex 0.164.0 protocol candidate

Generated in the owned Windows Sandbox with `codex-0.164.0.exe app-server generate-json-schema --out <owned-output>` (no experimental flag).

- Official package: `https://registry.npmjs.org/@openai/codex/-/codex-0.164.0-win32-x64.tgz`
- npm integrity: `sha512-urWPhwh9YavuoAPIkmVYKBTtd5OtGVL4Ki0Fb28dhK/Mz6Pt4LSy+gbRrZFz5+RXrw9ubvXuXDCcd8beYcWhDg==`
- Executable SHA-256: `6f267830b029ee25d9339864aa867cd45de9f8fd4c5ea007d2e63c13a0c66d36`
- License: Apache-2.0
- Generated JSON files: 320
- Bundle SHA-256: `314f88fc41cbed69182f59196acfdc09252fb491fd8a7f077a5824b473872fe5`

Fingerprint: sort JSON paths relative to this directory, join `<file SHA-256>  <forward-slash path>\n`, hash the UTF-8 result. PROVENANCE.md is excluded.

The 0.144.6 bundle and historical P2 bundle scripts remain immutable historical evidence; they do not qualify this candidate. Current mission packaging uses scripts/mission/build-receipt.ps1 and package.ps1.

Same-Sandbox actual gpt-6-luna compatibility: strict-config with all existing disabled features/config overrides accepted; initialize/initialized, model/list, ephemeral read-only thread/start, schema-constrained turn/start and turn/interrupt worked. Translation/rewrite completed with gpt-6-luna reported, cancellation completed interrupted, no tool activity, diagnostic process exited 0. This is protocol evidence, not installed-app Accept or general language qualification. New-version device login and full production client/package integration remain to be verified. No credential data is included.
