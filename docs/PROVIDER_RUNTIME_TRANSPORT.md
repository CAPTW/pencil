# Provider runtime transport

| Provider | Official client | Transport | Input | Output | Auth | Cancel | Timeout |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Codex | pinned `codex` app-server | JSON-RPC stdio | structured rewrite request | strict JSON replacement | Codex account | interrupt turn | existing app-server bounds |
| Google Antigravity | `agy.exe` print mode | argv array, no shell | `-p` canonical writing prompt | `--output-format json` envelope `status`/`response` | official `agy` keyring | kill + Job Object | `--print-timeout 2m`, wait 130s |
| Claude | `claude` CLI | argv array, no shell | `-p` canonical writing prompt | `--output-format json` | official `claude auth` | kill + Job Object | 120s |

Shared CLI writing path: isolated runtime cwd, Job Object, env key removal, stdout/stderr capture, no retry, no cross-Provider fallback.

Limitations: Windows command-line size is preflighted and fail-closed. Antigravity has no official noninteractive auth-status command. Codex remains app-server, not a CLI `-p` prompt.
