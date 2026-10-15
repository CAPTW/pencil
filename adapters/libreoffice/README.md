
> Current Gemma checkpoint (2026-10-15): clean e6ff394 app/host/broker plus paired
> Writer-enabled native DLL/OXT passed owned saved DOCX EN-KO/natural/polite
> Accept, exact reread, bold/RGB/alignment preservation, Undo, edited draft,
> changed-source and Disable checks. One paragraph/mixed surroundings only;
> actual D-drive Gemma, LibreOffice26.2.4.2 in owned SSD temp profile/runtime.
> Native shortcut + Owner foreground activation + trusted widget UI Accept.
> This is a separate installed candidate, not personal-install activation, Word
> support or current-Gemma qualification of all earlier paragraph-topology scopes.
> Detailed receipts, preserved failures and temporary-file cleanup limitation:
> HANDOFF.md, 2026-10-15 Current Gemma + Writer saved DOCX qualification.

# LibreOffice Writer adapter (implementation in progress)

### Preparing the paired extension from a verified package

Use `scripts/mission/build-writer-extension.py --package-root <package-or-install-root>
--library-file <same-root>/target-apply.dll --installed-library <final-absolute-path>/target-apply.dll
--output <fresh-path.oxt> --enable-saved-docx --enable-uniform-format --enable-mixed-surroundings`.
Run from a checkout containing the package's source commit. This checks actual
app/DLL hashes against both receipts, the exact local Writer executable, and all
six adapter source inputs against that commit (checkout line endings may differ).
A Writer-disabled native build is rejected: generating an OXT cannot enable it.
The command does not install/register an extension or touch a document/profile.
The output remains a candidate, with one-review Enable/Disable required in Writer.
Metadata checks establish consistency, not publisher authenticity or new runtime
qualification. Only use a trusted source/package. Without `--package-root`, the
older low-level builder remains available for development and records null binding.
Broader paragraph/cross-format flags require separate qualification; do not infer
support from successful package generation. An installed path change requires a
new paired OXT. Native runtime pinning still checks the library when it is used.

Current checkpoint: see "Uniform paragraph topology candidate" below. The opted-in
candidate OXT is installed-widget qualified only in the documented owned synthetic
scope; general Writer acceptance is incomplete. Earlier sections below are
chronological implementation/qualification history, not the current scope limit.

## Historical core and activation development

`pythonpath/grammar_writer_core.py` is the target-resident transaction core.
It is not independently registered as a UNO service and is not yet included in
an enabled personal package. The synthetic service wrapper and adversarial hooks
remain under scripts/mission/libreoffice-writer-probe; never ship those hooks.

The goal is the existing Grammar selection -> shortcut -> Instant -> reviewed
Accept experience in Writer. Activation still requires a deployment-bound,
authenticated native connection, visible per-document opt-in/disable and actual
installed-widget qualification. The core's XJob implementation is an internal
adapter interface, not an authorization boundary for arbitrary UNO callers.

Current scope is a new unsaved plain-body Writer document, one exact selected
range, selected source up to4096 cursor steps/4096 Python characters, bounded local
body snapshot up to16384 Python characters, original operation <=3s and separate
30s review authority. Hidden text, sections, tables, frames/draw objects, fields,
bookmarks and tracked changes are refused. The local body snapshot is used for
an in-memory Undo digest and discarded; only selected before/after strings belong
to the document's in-memory Undo action. No network or persistent text history.
These checks do not constitute full eligibility/formatting qualification for user
or saved documents.

Writer's native setString emits Undo events between deletion and insertion.
The core suppresses recording of these suboperations while retaining existing
Undo history, then registers one XUndoAction. Modified-state notifications are
batched over the actual replacement and the dirty flag restored. User edits from
postwrite callbacks are retained and reported UNKNOWN_CHANGED, never rolled back.
Undo/Redo validates a bounded whole-body digest and reconstructs only the exact
selected range using measured cursor steps. It never restores the whole body.
The XComponent disposal path releases retained selected strings and document.
Undo actions retain numeric steps and digests, not long-lived mutable cursors.

Native trial evidence lives at
D:/dev/grammar-evidence/grammar-writer-activation-20261005.
R7 passed41 actual GUI Writer checks, including the previous failing undo callback
and earlier/later native history plus disposal. R8 after module separation had a
soffice.bin crash (Application1000) and is FAIL. Removing the no-longer-needed
retained Undo cursor is a lifetime simplification, not a proven crash root cause.
No product activation or general Writer/DOCX support is claimed by these results.

Build the guarded test OXT with:
`python scripts/mission/build-libreoffice-writer-probe.py --output <new-path.oxt>`.
The build uses the actual core and emits per-input and archive SHA256 values.
The offline restricted-Medium Sandbox launcher must supply the documented
GRAMMAR_WRITER_FIXTURE guard and disposable profile; do not install in a normal
LibreOffice profile.

R11 passed46 checks and32 repeated Apply/Undo/Redo cycles. R13/R14 exposed a
native-channel defect: the exact replacement happened but closing the server
immediately discarded the unread success reply. R15 keeps the native terminal
poll/deadline until the peer disconnects; capture/Apply/Undo, wrong-image refusal
and actual cleanup pass. All52 GUI checks pass; launcher cleanup=true,
forced=false, both process exit codes0 and remainingProcesses0. This uses the
production core and native ABI with a synthetic trusted client, not the installed
Grammar widget. The six bridge boundary tests include the preserved red/green
terminal-reply regression. Core and application routing are implemented; product
menu/install/widget qualification remains pending.


Product-entry candidate (qualification in progress)
-------------------------------------------------
`writer.py` is a separate product-only XJobExecutor. Its menu command only opens
an initially disabled panel. The Enable button opts into the bound new document
for one review; Disable, closing the panel/document and connection completion
revoke access. No public XJob capture/accept method or adversarial hook is shipped.
`Addons.xcu` adds the Writer-only menu entry. The OXT does not start itself.

Build a paired OXT with `scripts/mission/build-writer-extension.py --library-file
<verified target-apply.dll> --installed-library <absolute installed DLL path>
--output <fresh.oxt>`. The generated config binds the installed path and SHA256;
the panel verifies it while holding a non-write/non-delete-sharing read handle.
Only use the dedicated disposable profile during qualification. No normal-profile
installer or generic Writer/DOCX support is claimed. The optional build-receipt
Writer image/path arguments bind the actual tested soffice.bin, not soffice.exe.
The default build remains disabled. Actual installed widget qualification uses
Guest-App-Writer-Product.ps1, writer-product-probe.py --widget and writer-app-e2e.mjs.
These fixtures do not invoke private document mutation helpers or live Providers.


Activation boundary after installed R23 and safety R24
-----------------------------------------------------
R23 installed10737e9 + paired OXT passed actual shortcut -> Instant -> edited
Accept -> exact Writer reread -> Undo, changed-source preservation, Disable and
natural app/Office exit with complete removal. Disable can yield the honest
"Apply outcome unconfirmed" when it closes the channel; no automatic retry occurs.
The synthetic document reread proves unchanged content in that test, not a generic
promise that every lost response means no mutation.

Follow-up review strengthened the one-review greeting with the exact Writer
window handle and liveness, keeping the Chromium zero-window contract unchanged.
The core checks actual document foreground before and after capture reads. Current
supported text is bounded default-style plain body: named character/paragraph
styles, direct font/appearance differences, hyperlinks and non-text portions are
refused through actual paragraph/portion metadata before reading selected text.
Aggregate cursor PropertyState is not sufficient evidence of absence of formatting.
R24 passed54 core/real-native checks and32 repeated cycles; the shared Windows
channel probe passed15 scenarios, including zero-window mismatch and exact bound
window acceptance. The final installed-widget recheck of these guards is pending.

Close the Writer panel before enabling another document. Close LibreOffice before
uninstalling/updating Grammar because it owns the loaded native DLL. Do not install
this candidate in a normal profile yet; current deployment qualification is the
owned offline Sandbox with exact26.2.4.2 engine/path and disposable profiles.

R25/R26 pass57 actual Writer checks including multiline replacement/Undo/Redo,
32 repeated transactions, formatting and original-deadline refusals, exact native
window greeting, disposal and cleanup. R26 additionally rejects every writable
Char property whose actual portion state is non-default, covering unsupported
appearance attributes beyond the explicit default-style value comparisons.
The old aggregate cursor state inspection is not used to qualify rich formatting.


Final installed candidate (R27)
------------------------------
The clean0fc8624 package and paired OXT passed the actual installed widget flow
with all post-review guards:6 checks, exact replacement/Undo, source-change and
Disable refusal, natural exit and complete uninstall. Evidence is
D:/dev/grammar-evidence/grammar-writer-activation-20261005/guest-r27.
This supersedes the pending statements in earlier checkpoints above. Enable the
bound new document from the Writer Grammar menu panel, select text and press
Ctrl+Shift+G, review/edit the Instant draft, then Accept. Re-enable explicitly
for another review. Only the pinned26.2.4.2 disposable-profile default-style
plain-body scope is qualified. Saved/rich documents and normal-profile use are
not qualified. No automatic Provider call is required. Close LibreOffice before
removing the paired installation.


Saved plain DOCX candidate (2026-10-05)
-------------------------------------
The core now has explicit saved-document admission, still off in ordinary OXT
builds. `build-writer-extension.py --enable-saved-docx` records the switch in the
paired extension configuration; no document or transport request can enable it.
Only local-drive writable Office Open XML Text documents passing the existing
plain-body/default-format metadata gates are admitted. No automatic file Save.
Document identity includes the bound live object/controller/window and URL;
Save/SaveAs/SaveTo/load/unload/mode events revoke the one-review epoch, including
A->B->A changes. Disable detaches event observation. Supported formatting has not
been broadened. Actual saved component and installed-widget qualification are
recorded separately in HANDOFF; no generic DOCX or rich-document claim.


## Saved plain DOCX installed qualification — 2026-10-05
Product extension source5a339b8 with explicit --enable-saved-docx is now qualified
in the owned offline pinned26.2.4.2 Sandbox with installed app/native0fc8624.
These are two separately bound artifacts, not a claim that the old app commit
contains the new Python adapter. OXT SHA256:
997ed14943ad2e8d6cce0a1dc9e583dabcbd359af8c5433c5156725f69c3551b.
The paired DLL hash remains61c50bb11ae3243efe29b2cadd74bfbc3c4e944d8dfdb2e4767167002e471376.

Core R5:73 checks +32 Apply/Undo/Redo cycles PASS, actual saved/reopened DOCX,
Save and SaveAs A-B-A revocation, Save inside Undo entry, user edits, readonly
refusal and event detach. R1 foreground failure and R3/R4 termination reply
FAIL preserved. R5 does not equate a disconnected terminate response to cleanup:
launcher independently requires actual Office exit0/processes0/profile removal.

Installed product guest-product-r1:6 widget checks PASS plus actual DOCX reopen
and unchanged disk hash (no automatic file Save). Real hotkey/Instant/edited
Accept/exact reread/Undo, source edit preservation, Disable and natural app exit.
Office/Python0, cleanuptrue/forcedfalse/processes0; uninstall complete. Guest
stopped. Default OXT builds remain saved-disabled; qualified candidate uses the
explicit packaged option and visible per-document one-review opt-in. No automatic
Save or new rich formatting support. Current F06 scope includes this bounded saved
case while preserving the previous unsaved-only scope as historical export data.

Evidence: D:/dev/grammar-evidence/grammar-writer-saved-apply-20261005.
Package3/bridge6 checks and governance PASS; no Rust changes, no full Rust rebuild
claimed. Same-integrator review only. Installed original app/native hashes checked
by the widget fixture; OXT receipt binds every Python/config input. Next: preserve
rich formatting through reviewed mutation and Undo, and qualify additional DOCX
structures before expanding admission. Actual Office/Hancom, physical IME, normal
profiles and live Providers remain NOT_RUN. Mission INCOMPLETE; no main/push.


## Uniform Writer formatting candidate — 2026-10-05
The next bounded capability preserves homogeneous Western/Asian/complex weight,
opaque non-theme RGB and paragraph alignment in one Standard body paragraph.
Defaults remain off; the paired OXT exposes --enable-uniform-format explicitly.
Unknown interop metadata, theme/transparency, mixed runs, hidden text and multiple
paragraphs stay refused. Formatting plus effective style defaults participate in
capture validation and custom Undo fingerprints. RGB DOCX roundtrip metadata is
strictly matched, not a generic permission to copy arbitrary metadata. Product
entry/fixture additions are isolated; no change to app/native binary0fc8624.

Owned Sandbox R10:28 native checks and16 formatted Apply/Undo cycles PASS, including
saved/reopened DOCX and unchanged file hash. R1/R2/R7 postwrite failures, R3/R4/R5
saved-format limitations, R6 unused-complex-color assumption, R8 UNO array typing
and R9 incorrect newline fixture are preserved. Actual paragraph break fixture
now verifies two paragraphs before refusal. R10 natural Office/Python0, cleanuptrue,
forcedfalse/processes0. Existing plain regression and installed widget qualification
are next; no F06 or general Writer acceptance promotion yet.
Evidence:D:/dev/grammar-evidence/grammar-writer-uniform-format-20261005.


## Uniform Writer installed qualification — 2026-10-05
Product OXT2a16115 (aa1f4df00d357f5e0b2127cd3360c2e9d25c4157f55f4d65e1410a2d6ce509dc)
with --enable-saved-docx --enable-uniform-format and unchanged app/native0fc8624
passes guest-product-r1:8 installed widget checks plus5 enclosing checks. Actual
hotkey/Instant/edited Accept/reread/Undo preserves every portion's script weights,
RGB and alignment. Source edit and Disable preserve the document. The Disable
UI reports "Apply outcome unconfirmed"; exact reread separately proves unchanged
text, not a confirmed protocol rejection. No automatic retry or Save.

Qualification is only pinned LibreOffice26.2.4.2, owned fresh profile, one Standard
body paragraph with homogeneous supported weights, opaque non-theme RGB and
alignment, new or local DOCX. No mixed formats, arbitrary interop metadata, theme
color, hidden fields, multiple formatted paragraphs or complex structures.
Ordinary product builds still default both capabilities off; previously qualified
plain mode stays separate. This is not general rich DOCX or normal-profile support.

Plain regression:73 checks/32 cycles PASS. Formatted R10:28 checks/16 cycles PASS.
Product app/Office/Python natural0, forcedfalse, zero residual processes, complete
uninstall. Exact original app/DLL hashes and new OXT input hashes are separate
receipts in D:/dev/grammar-evidence/grammar-writer-uniform-format-20261005.
Final R11:32 checks/16 cycles PASS, including inherited-style-change refusal,
explicit Save after Redo and exact text/format after reopening that saved file.
Natural Office/Python0, cleanuptrue, forcedfalse/processes0; every guest stopped.
F06 and Source generator now express this bounded union while preserving old
unsaved-only and saved-plain snapshots. Same-integrator review, no independent
reviewer. Mission INCOMPLETE, main unchanged, no push or live Provider.

## Mixed surroundings candidate — 2026-10-06

OXT source a3b231a, paired app/native0fc8624, pinned LibreOffice26.2.4.2.
Build with `--enable-uniform-format --enable-mixed-surroundings`; add
`--enable-saved-docx` for local DOCX. These options default off.

In one Standard body paragraph, select text with one supported bold/RGB style.
Other text may use different supported bold/RGB styles. Enable one review, press
Ctrl+Shift+G, review/edit the local draft, and explicitly Accept. Complete text and
surrounding formats are rechecked. Undo/Redo restores only the selected range.
There is no automatic file Save.

Owned installed synthetic widget8 and native39 checks passed, including16 saved
DOCX Apply/Undo cycles and explicit Save/reopen. Cross-format selections, selected
line breaks, multiple formatted paragraphs, complex structures and inconsistent
DOCX color metadata remain unsupported. Normal profiles, physical IME and actual
Office/Hancom are not qualified. Prior plain/uniform qualifications above retain
their original scope. Mission INCOMPLETE; no general-user installation or release.

Evidence: D:/dev/grammar-evidence/grammar-writer-mixed-20261006/RESULT.md.
Handoff records the preserved failed preparations and exact package/source chain.

## Paragraph-preserving candidate — 2026-10-06

OXT source9443ff2 with unchanged app/native0fc8624, LibreOffice26.2.4.2.
Add `--enable-paragraphs` to `--enable-uniform-format --enable-mixed-surroundings`
(and `--enable-saved-docx` for local DOCX). All options default off.

A bounded Standard body may now contain several paragraphs, including empty ones.
Select a homogeneous range within ONE paragraph, review/edit and explicitly Accept.
The adapter preserves the other paragraphs, supported bold/RGB runs, paragraph
alignment and empty-paragraph typing formats. Exact rereads cover the entire body
and layout; Undo/Redo changes only the selected range. No automatic file Save.
Cross-paragraph/cross-format selections and complex structures remain refused.

Owned synthetic qualification: native41 checks including8 saved DOCX cycles;
installed widget8 plus5 enclosing checks. This is not general Writer/Office/Hancom
or normal-profile acceptance. Physical IME/live Providers remain NOT_RUN.
Evidence: D:/dev/grammar-evidence/grammar-writer-paragraphs-20261006/RESULT.md.

## Cross-format selection candidate — 2026-10-06

OXT600aa72 with app/native0fc8624, pinned LibreOffice26.2.4.2. Add
`--enable-cross-format` to the saved/uniform/mixed/paragraph candidate switches.
Default remains off. The panel shows the policy before Enable: replacement uses
first selected style; Undo restores original styles. Select supported bold/RGB
runs within ONE Standard body paragraph. Several surrounding paragraphs are
allowed. Apply and Undo/Redo reread complete body text/format, preserving all
unselected paragraphs and empty typing formats. No automatic file Save.

Owned native47 and installed widget8/enclosing5 checks passed, including new and
saved/reopened DOCX, Unicode, original-style restoration, source-change/Disable
preservation and complete natural cleanup. Early native Undo failures are kept:
resetting an inherited color to default produced automatic/transparent color;
the implementation now restores the measured effective value. No generic theme
or unsupported metadata admission was added. Cross-paragraph selection and
complex structures remain unsupported. See grammar-writer-cross-format-20261006.

## Paragraph-span candidate — 2026-10-06

OXTcde1097 + unchanged app/native0fc8624, pinned LibreOffice26.2.4.2. Add
`--enable-paragraph-span` to cross-format/paragraph/mixed/uniform options (and
saved-DOCX option for local files). All default off. Policy appears before Enable.

Select across existing Standard body paragraphs, review/edit the whole selection,
and Accept. Draft must keep the same paragraph count and empty parts. Each nonempty
part uses its first selected style. Apply works from right to left without touching
paragraph delimiters; grouped Undo restores original range text/styles. Exact full
text/layout checks preserve surrounding text, alignments and empty typing formats.
No automatic Save. Limit8 paragraphs; installed owned fixture covers4 including an
empty paragraph and distinct alignments/colors. No normal-profile qualification.

Paragraph merging/splitting, new/deleted empty parts and complex structures remain
unsupported. Source/permission changes refuse; postwrite uncertainty preserves user
edits and dirty state without whole-body rollback or automatic replay. See evidence
D:/dev/grammar-evidence/grammar-writer-span-20261006/RESULT.md for failures and scope.


## Uniform paragraph topology candidate (2026-10-07)

Explicit `--enable-uniform-topology` additionally requires all prior paragraph/span
flags. It defaults off. Up to8 affected/output Standard body paragraphs must share
identical whole-paragraph supported character/RGB/alignment metadata. Reviewed
replacement can merge/split and add/remove empty parts using that common format.
Outside paragraphs retain exact text/styles. Undo/Redo is one range action, never
whole-body restoration. Embedded soft linebreaks in affected paragraphs are refused;
differently formatted spans keep the previous count/empty-part preservation policy.

Product OXTd9a6b1e (core86dfae4) + runtime0fc8624, installed fixturea426701, in pinned
LibreOffice26.2.4.2: native47 and installed10+5 PASS in owned synthetic new/local saved
DOCX only. No automatic Save. Source-change and Disable preserve actual contents.
Disable UI can still say outcome unconfirmed; independent reread proves no mutation.
Evidence: D:/dev/grammar-evidence/grammar-writer-topology-20261007. Product activation
in general profiles, physical IME, live Provider and general Writer are NOT_RUN.
