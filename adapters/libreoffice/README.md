# LibreOffice Writer adapter (implementation in progress)

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
