# Isolated Writer transaction experiment

Known unsafe experiment: the undoActionAdded counterexample currently FAILS.
Do not ship or enable in a user document. This is a Windows Sandbox-only test
component, not a shipped adapter. It refuses
outside WDAGUtilityAccount plus GRAMMAR_WRITER_FIXTURE=1. The job interface is a
test driver, not authenticated product IPC. Never register it in a user profile.
The separately registered Hooks service deliberately injects adversarial edits.

The guarded guest-libreoffice-selection-probe.ps1 launcher uses a disposable
C:/GrammarLOProfile, the publisher-verified portable LibreOffice 26.2.4.2,
software raster rendering, and the bundled Python client. No macro policy changes,
Provider requests, user documents or host clipboard are involved. Package only
writer.py, hooks.py, description.xml and META-INF/manifest.xml at the OXT root;
the client is scripts/mission/libreoffice-writer-client.py. Existing mission
Sandbox launchers supply offline mappings and a restricted Medium token.

The experiment binds a new unsaved plain-body Writer document and controller,
selected range/source, selection/content revisions, one-shot review token,
30-second review lifetime and original bounded operation deadline. Each operation
runs through AsyncCallback and checks actual process/window-thread affinity.
Mutation is followed by exact rereads and no whole-document rollback. A postwrite
mismatch is UNKNOWN_CHANGED, never a no-write or safe-rollback claim.

Writer setString deletes then inserts, so target UI-thread execution alone does
not establish atomicity against synchronous listeners. The experiment batches
modified-state notifications with disableSetModified/enableSetModified, restores
the dirty flag, preserves Undo, and revalidates after enterUndoContext. Permission
revocation increments an epoch immediately even while a callback is pending;
REVOKING means cleanup is still pending. Tests inject user edits from the actual
in-process Undo and modification listeners; this does not prove all possible
editor callbacks safe.

Remaining product prerequisites include authenticated bounded transport, full
privacy/sensitive-content eligibility, bounded reads before retrieval, formatting
and saved-document support, full reentrancy analysis, and installed Grammar
review UI integration. This fixture must not be advertised as DOCX or arbitrary
LibreOffice support. Protected sections and non-body structures are refused.

The evidence root is D:/dev/grammar-evidence/grammar-libreoffice-writer-20261005.
It preserves failed candidates separately from subsequent runs, exact OXT/client
snapshots, source hashes, acceptance budgets and guest cleanup receipts.
