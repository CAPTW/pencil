# Provider surface discovery

Official product/client names, local probes, and the selected integration routes for this candidate.

## Codex

- Official product/client: Codex CLI / Codex app-server
- Installed executable: `codex.cmd` on PATH (`C:\Users\USER\AppData\Roaming\npm\codex.cmd`)
- Installed version: pinned product version `codex-cli 0.144.6`
- Official documentation: existing accepted Codex app-server contract in this repository
- Authentication model: externally managed ChatGPT device login through Codex app-server; no app-stored credentials
- Machine-readable invocation: `codex app-server` stdio JSON-RPC
- Request/response transport: local stdio app-server
- Cancellation support: interrupt turn + process job cleanup
- Sign-in/sign-out: official device login in-app; no in-app sign-out API
- Selected integration route: wrap the current Codex client behind `ProviderKind::Codex`
- Local live-test availability: depends on an existing managed Codex session
- Limitations: remains default after settings migration

## Google Antigravity

- Official product/client: Antigravity CLI (`agy`) and Antigravity desktop
- Installed executable: `C:\Users\USER\AppData\Local\agy\bin\agy.exe`
- Installed version: `1.1.10`
- Desktop present: `C:\Users\USER\AppData\Local\Programs\Antigravity\Antigravity.exe` (not used as the writing route)
- Official documentation: https://antigravity.google/docs/cli/headless/ and https://antigravity.google/docs/cli/install/
- Authentication model: official local keyring / Google account session used by `agy`. Gemini API key is a separate official option and is **not** used by this app (no plaintext key storage).
- Machine-readable invocation: `agy -p --output-format json --disable-slash-commands`
- Request/response transport: process stdout JSON envelope (`status`, `response`)
- Cancellation support: terminate the owned `agy` process tree
- Sign-in/sign-out: interactive `agy` session / `/logout`. In-app sign-in is not a documented non-interactive command.
- Selected integration route: official `agy` headless print mode from an empty runtime cwd, without `--dangerously-skip-permissions`
- Local live-test availability: CLI installed; a writing request still requires the official local Google session
- Limitations: Gemini CLI (`gemini`) is a different product and is not used as Antigravity

## Claude

- Official product/client: Claude Code CLI
- Installed executable: `C:\Users\USER\AppData\Roaming\npm\claude.cmd`
- Installed version: `2.1.201`
- Official documentation: https://code.claude.com/docs/en/cli-reference and https://code.claude.com/docs/en/headless
- Authentication model: official `claude auth login` / `claude auth status` / `claude auth logout` (Claude account). `--bare` is not used because it skips OAuth/keychain and would require an API key.
- Machine-readable invocation: `claude -p --output-format json --permission-mode dontAsk --permission-prompts none`
- Request/response transport: process stdout JSON (`result`)
- Cancellation support: terminate the owned Claude process tree
- Sign-in/sign-out: official CLI auth commands
- Selected integration route: official Claude Code print mode from an empty runtime cwd
- Local live-test availability: CLI installed; `claude auth status` reported a signed-in Claude account at discovery time
- Limitations: selected text is untrusted prompt data; tools/MCP/skills are not enabled by this app
