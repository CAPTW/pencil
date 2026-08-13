# Codex App Server schema provenance

- Supported Codex version: `codex-cli 0.144.6`
- Generation executable source: the canonical resolver's first PATH `codex.cmd`; the machine-local absolute path is intentionally not committed
- Generation command: `& $resolvedCodex app-server generate-json-schema --out src-tauri/protocol/codex-cli-0.144.6`
- Experimental schema fields: not requested (`--experimental` was not passed)
- Generated JSON files: 267
- Bundle SHA-256: `c954593823626b5194b7f687c27d542eb87fcfa63e5e11af3ca95f9b6c39c6e8`

The bundle fingerprint is the SHA-256 of a UTF-8 manifest formed by sorting all
generated `*.json` files by repository-relative path, then writing one line per
file as `<lowercase-file-sha256><two spaces><forward-slash-relative-path>`, with
a final newline. `PROVENANCE.md` is intentionally excluded from the fingerprint.

Runtime executable resolution policy:

1. Use non-empty `CODEX_PENCIL_CODEX_BIN` as the explicit executable path.
2. On Windows, otherwise select the first PATH result for `codex.cmd`.
3. If no command shim is present, select the first PATH result for `codex.exe`.
4. Do not use the PowerShell-only `codex.ps1` shim for the Tauri child process.

The checked stable schema confirms a single `initialize` request followed by an
`initialized` notification, `account/read`, managed `chatgptDeviceCode` login,
login cancellation/logout, and the `account/login/completed` and
`account/updated` notifications. It does not define the previously sent
`responsesapiClientMetadata` field on `turn/start`.
