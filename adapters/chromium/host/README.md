# Local Instant native host

Run the preparation/removal scripts with PowerShell 7 (`pwsh -NoProfile -File ...`); do not launch Windows PowerShell with an inherited incompatible module path.

Build with `cargo build --manifest-path src-tauri/Cargo.toml --locked --offline --bin grammar-chromium-host`.
Run binary unit tests with the equivalent `cargo test ... --bin grammar-chromium-host`.
Run `python adapters/chromium/host/test_host.py <built-executable>` for the synthetic executable protocol fixture.

The host accepts Chromium's origin argument only when it exactly matches the single extension origin in `grammar-chromium-host.origin.json` beside its executable. Messages have a four-byte little-endian length and UTF-8 JSON. Requests are `{version:1, op:"analyze", id, epoch, revision, text}`; id/epoch are nonempty and at most 128 UTF-8 bytes. Frame limit is 128 KiB, text limit 8192 UTF-16 code units, response limit 256 KiB. Suggestion start/end are UTF-16 offsets. Unknown JSON fields, malformed/truncated/oversized input and invalid origins close the channel without document-bearing errors. `op:"deep"` returns `error:"deep_unavailable"`; no Provider is called.

No text is persisted and no subprocess or network connection is created. The existing library Instant engine is used directly. The browser owns each native port/process; clean stdin EOF ends the process. Input waits follow port lifetime, with no detached reader/cleanup worker. The extension must keep one port and at most one in-flight request and disconnect on disable/navigation. This executable fixture proves neither browser transport nor live Provider acceptance.

`Prepare-Host.ps1 -Executable <exe> -ExtensionId <32 letters a-p> -OutputDirectory <new-task-owned-dir>` prepares a unique host manifest, copied executable, origin sidecar and hash receipt. It does not register by default. The optional `-Register` creates only a unique HKCU Chrome native host entry; use only with coordinator-admitted task-owned browser testing. It never overwrites an existing key/package. Use the returned host name in extension configuration. `Remove-Registration.ps1 -PackageDirectory <dir>` removes only that exact unchanged registration and preserves files. Normal user-profile installation remains outside this fixture workflow.
