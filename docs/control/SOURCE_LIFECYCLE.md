# Source index and lifecycle

Canonical semantic state: `control/state.json`. Canonical durable docs: `docs/control/`. Git branch/HEAD/tree/parent/subject are measured at export, not manually repeated in tracked files. `product_audit_base` labels a historical product snapshot; doc/tool commits do not change that tested source. The generator verifies product diff remains zero for this R0 package.

Exact logical Sources (only these 12 markdown files go into the Project):

1. 00_READ_ME_FIRST.md
2. 01_PRODUCT_CHARTER.md
3. 02_CURRENT_REPOSITORY_STATE.md
4. 03_ARCHITECTURE_PRIVACY_AND_THREAT_MODEL.md
5. 04_CURRENT_DECISIONS_AND_AUTHORITY.md
6. 05_PHASE_STEP_ROADMAP.md
7. 06_ACCEPTANCE_AND_TEST_MATRIX.md
8. 07_MULTI_AGENT_DEVELOPMENT_CONTRACT.md
9. 08_ACTIVE_TASK_LEDGER.md
10. 09_SESSION_HANDOFF.md
11. 10_CURRENT_REPOSITORY_AUDIT.md
12. 11_SOURCE_INDEX_AND_LIFECYCLE.md

Source 02 is generated from semantic state plus measured Git identity. The manifest's resolved_state contains that snapshot plus package ID and payload digest. Digest is SHA256 of sorted filename + TAB + SHA256(file bytes), LF joined without trailing LF; its field is outside payload to avoid self-reference. Source 08/09 task fields are generated, never manually duplicated. Source 04 includes the first-adapter decision. Other Sources consume durable document content. A Source export is a snapshot, not a live synchronization service.

Qualified generation: `python scripts/project_source.py generate --repo <canonical-main-root> --output <NEW-external-root>`. It rejects candidate branch, dirty tracked tree, existing output root, product delta and inconsistent state. Review preview requires explicit `--preview`; PREVIEW_NOT_QUALIFIED packages must not be applied. Verification: `python scripts/project_source.py verify --repo <same-root> --package <root> --extract` (preview verification additionally needs `--allow-preview`). Main advance or altered inputs makes repository-bound verification fail stale rather than silently accepting it.

The package includes payload, manifest, checksums, verification record, Owner handoff, replacement matrix and two fresh-chat prompts; transport ZIP is archive only. Those external metadata files are not logical Sources. Verification record reports generator checks, not independent-review or UI success. Independent evidence is a separate result packet. Applying to the Project UI is an Owner action outside the Repository truth and never stored as volatile current Source state.

Refresh after adopted changes to behavior, decisions, acceptance, known defects or exact next task. Minor wording-only changes require a documented materiality decision. Old sources are replaced in the Project active set; existing repository/history evidence is preserved, not destructively deleted. No automatic successor execution follows an export or a successful fresh-chat check.

Future product changes require a successor generator policy because the R0 product-diff-zero guard deliberately anchors this audit. Do not bypass it by rewriting the historical audit base without an audited new acceptance/state decision.
