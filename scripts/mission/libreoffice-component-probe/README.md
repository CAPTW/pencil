# Isolated LibreOffice component probe

This is a test component, not a shipped Grammar adapter. Never register it in a normal
user profile or expose its test UNO endpoint as a product IPC channel. It has no
mutation method, no production authentication, and no general sensitive-field policy.

Package `affinity.py`, `description.xml`, and `META-INF/manifest.xml` at the root of
`GrammarAffinity.oxt` (ZIP). Run through `guest-libreoffice-selection-probe.ps1` with
`GRAMMAR_AFFINITY_FIXTURE=1` in the existing restricted-Medium, task-owned Windows
Sandbox. The launcher registers only into its disposable `C:\GrammarLOProfile`.
The independent `libreoffice-affinity-probe.py` creates its own visible Writer document,
requests the component's single-use inspection, and checks PID, native window/thread
identity, selected text, unchanged document, replay refusal and natural cleanup.

The component uses `com.sun.star.awt.AsyncCallback`. A successful affinity probe proves
where that callback ran for this LibreOffice version. It does **not** prove SolarMutex
retention across arbitrary callbacks, absence of reentrancy, atomic replacement,
privacy qualification, or the complete desktop shortcut/Instant/Accept flow.

The separate `libreoffice-selection-probe.py` includes deliberately failing Impress
Undo qualification and a failed dispatch alternative. Do not remove those assertions
or count their earlier successful checks as whole-suite acceptance. `SAL_SKIA=raster`
is a process-local software-rendering condition for the vGPU-disabled guest, not a
system security change or proof of the earlier crash's root cause.

Next production work requires a target-side Writer review transaction, original
operation deadline, exact document/controller/range/source/revision binding,
reentrancy and concurrent-edit rejection, sensitive-content refusal, authenticated
transport and explicit opt-in/Accept integration. Keep activation disabled until
those production paths and their adversarial cases run against the actual app.
