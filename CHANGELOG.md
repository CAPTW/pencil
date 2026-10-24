# Pencil changelog

Exact package identities (source commit, file hashes and checks) are in each
release's MANIFEST/BUILD_RECEIPT and VALIDATION assets on the immutable
[GitHub releases](https://github.com/CAPTW/pencil/releases). Read
[qualification and limitations](docs/PROTOTYPE.md) and the detailed
[handoff](docs/control/HANDOFF.md). Historical failed measurements remain evidence.

## Unreleased

- Align README, English overview, getting-started guide and package/adapter docs
  with Prototype 3 r4 behavior: r4 asset names, API presets and Windows Credential
  Manager key storage, 30-second Accept expiry, Copy-only single-line Edit, and
  uninstall preserving user data. Documentation only; no executable change.

## v0.1.0-prototype.3 — Windows x64 prerelease

Revisions are separate replacement ZIPs in the same release; earlier ZIPs remain
for traceability. The `v0.1.0-prototype.3` tag identifies only the original package.

Commit SHAs changed in the 2026-10-24 history rewrite; release assets cite the
"published as" SHAs. See [the commit map](docs/control/HISTORY_REWRITE_2026-10-24.md).

- **r4** (source `1bcaef0`, published as `4be5b20`, 2026-10-24) — Capturing from an external editor opens
  the result even when Pencil is in the background. Official API model catalogs
  load once after completed key entry. Claude adaptive-thinking blocks are excluded
  from final output. Single-line Edit controls without automatic horizontal scroll
  reject direct Accept (Copy-only). Documents that Accept expires at most 30 seconds
  after capture, including inference and review time.
- **r3** (source `2e51f6e`, published as `743a06c`, 2026-10-24) — API key, Provider, model and consent are
  stored per profile in Windows Credential Manager and restored after restart;
  Revoke API consent and Remove API key and permission are separate actions.
- **r2** (source `8d646dc`, published as `5511f68`, 2026-10-24) — Claude, Google Gemini, OpenRouter and xAI
  API presets with model-catalog selection (session-only keys in this revision);
  scoped RichEdit Undo repair; Codex CLI 0.164.0 compatibility pin.
- **r1** (source `ee62d58`, published as `5e18309`, 2026-10-23) — Uninstaller refuses early while the
  launcher is active, before expensive preflight.
- **Original** (source `19a352b`, published as `5d43b2b`, 2026-10-23) — Incremental request supersession
  while typing; warm Gemma retention after idle-confirmed refusals; conservative
  person/title/negative-promise guards. Also includes the 2026-10-18 checkpoint:
  visible branding standardized on Pencil (settings, executable and Native
  Messaging identifiers stay compatible); proven emphasis alignment without a
  second inference; Korean synonym/connective/nested-condition/honorific
  regression coverage with HWP/HWPX Accept, Undo and saved-file rereads; rHWP
  Native Messaging review packaging with original-file protection; test cleanup
  restricted to marked owned paths; model startup failure/retry reporting and
  Windows main/PR source checks.

## v0.1.0-prototype.2 — 2026-10-14

Shutdown repair: WebView controllers close before the event loop returns and the
app waits for its browser processes within one 30-second deadline; a timeout is
reported as failure, not confirmed cleanup.

## v0.1.0-prototype.1 — 2026-10-14

First unsigned Windows x64 executable prerelease. Its failed-cleanup evidence is
preserved in the handoff.
