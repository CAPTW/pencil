# Grammar Tool-neutral Multi-Agent Development Contract

Contract-Version: 1.2.0

## Active mission override (Owner authorization, 2026-09-25)
For GRAMMAR-AUTONOMOUS-P3A-STABILIZATION-TO-P3B-CHROMIUM-PERSONAL-USE-R1, the Owner explicitly authorizes investigation, implementation, tests, local commits, reviewed safe local main integration and continuation through P3-A stabilization and the first Chromium textarea/simple-contenteditable adapter for personal use. This section supersedes conflicting historical one-step gates below, fixed file/iteration/commit budgets, per-step new-worktree requirements and fresh Source UI upload prerequisites. Use one dedicated mission worktree; preserve historical failed candidates and unrelated work. Historical failures remain failures. Review changed main before integration; no history rewrite or forced conflict resolution.

The mission packet records `mission_id`, `internal_continuation=true`, `scope_expansion=false`, `remote_publication=false`; legacy `automatic_continuation=false` means no continuation outside the authorized mission. `next_gate_policy=CONTINUE_WITHIN_MISSION` requires this exact mission and explicit boundary fields. No permission is implied for live Provider inference, paid calls, account/credential changes, user document/profile use, global installation, security weakening, push or release. Only task-owned synthetic environments are permitted. Unknown target/revision/source, protected fields and revoked permission fail closed. All other safety, ownership, evidence and validator requirements remain effective. P3-B activation requires the relevant safety prerequisites; unverified capabilities stay disabled. External ChatGPT Source application remains Owner work.

## GitHub validation amendment (Owner authorization, 2026-09-27)
The Owner now explicitly authorizes committing and pushing the current candidate to the existing PUBLIC repository CAPTW/pencil, branch codex/grammar-autonomous-r1, and standard GitHub-hosted Windows CI tests. This overrides the earlier no-push boundary only for that destination/branch. No main integration, tag/release, deployment, repository visibility change, paid larger runner, account/credential operation or live Provider inference is authorized. Repository-local source is sent; external local evidence and VM/user files are excluded. The exact optional github_validation object is checked in task/state; remote_publication=false continues to forbid general release/deployment outside this narrow source-push exception. CI synthetic success does not qualify interactive Windows capture/clipboard/Apply. Existing safety and historical failures remain.

## Historical task defaults (except active mission override)

## Authority and boundaries
Repository files and measured Git state are authority; chat, private memory, model identity and historical packets are not. Read this contract, control/state.json, docs/control/ACTIVE_TASK_LEDGER.md (if present), the current charter, roadmap and acceptance matrix before work. The Owner authorizes exactly one bounded task packet; examples in control/examples are EXAMPLE_NOT_AUTHORIZED and must never execute. Canonical integration uses Codex/Owner workflow, but every tool follows identical validation. P3-B direction approval does not authorize implementation. No automatic continuation and no next gate execution in this reset task. Preserve other agents' and existing dirty/untracked files.

control/state.json holds semantic state. The Project Source generator measures main HEAD/tree/parent/subject at export; never write self-referential HEAD identity into the tracked semantic state. Historical evidence remains explicitly historical. The Project Source package is external and its exact 12 payload files are the only logical Sources.

## Worktree and task admission
Every development task uses a NEW dedicated worktree and branch. Never develop on canonical main or reuse an existing worktree. Recommended branch: candidate/<task-id-lowercase>/<agent-id>/<timestamp>; worktree: D:\dev\worktrees\Grammar\<TASK_ID>__<AGENT_ID>__<timestamp>. Resolve canonical repository identity before starting. Validate task packet against current local main HEAD, tree and subject using `python scripts/validate_governance.py --root . --task <packet.json> --execution`. Stale base blocks admission: no inferred reinterpretation, rebase or cherry-pick. A successful validator proves structure/base, not Owner authorization, acceptance or safe external side effects; integrator checks actual Owner decision evidence.

Task, result, claim and handoff schemas in control/schemas are strict. A task explicitly records task_id, owner_decision, objective, base_branch/head/tree/subject, authorized_paths, forbidden_paths, required_reads, preconditions, invariants, acceptance_tests, live_tests, privacy_tests, evidence_root, commit_policy, integration_policy, project_source_policy, next_gate_policy, automatic_continuation and stop_conditions. Every one of these fields is required; packet_status separately distinguishes non-executable examples from an authorized packet. next_gate_policy is DO_NOT_EXECUTE_NEXT_GATE. Live checks not performed must be identified as not run, and privacy checks cannot be omitted. Result records entry/candidate identity, parents, subjects, changed/unintended paths, separate passed/failed/blocked tests and live/fixture evidence, privacy, limitations, residual processes, worktree state and disposition. A failed or blocked test never becomes PASS through fixture substitution.

## Exclusive path ownership
Before concurrent editing, integrator records claims in control/path-claims.json with task_id, agent_id, base_head, claimed_paths, claim_type, created_at, expires_at, release_state, conflicting_claim and owner_override. Only exclusive claims exist in this minimal implementation. Path/globs are repository-relative, Windows-case-normalized and slash-normalized. Absolute/parent paths and unsupported glob syntax fail closed. Active overlapping claims are rejected, including claims held by the same agent/task; released/expired claims do not confer authority. Expiry is checked against UTC. `owner_override` documents a decision but cannot silently bypass validator exclusion: release/re-scope conflicting claims first. No implicit transfer of another worktree's ownership.

Especially serialize src-tauri/src/main.rs, src/App.tsx, shared state, settings schemas, Provider manager, Apply safety, Project Source current-state files and root agent instructions. Authorized and forbidden path scopes must not overlap. The validator rejects ambiguity conservatively; narrow scopes instead of disabling it.

## Integration sequence
1. Verify exact authorized base.
2. Compare actual changed paths to task scope (not result claims alone).
3. Review candidate diff.
4. Re-run candidate tests with correct prerequisites.
5. Re-measure canonical main.
6. Fast-forward only if main is unchanged and all authorized conditions pass.
7. If main advanced, do not auto cherry-pick.
8. Review range-diff and semantic conflicts.
9. If needed create a bounded successor on the new base with fresh Owner authorization.
10. Re-run related tests after integration.
11. Assess current-state and Project Source materiality and regenerate only when warranted.

Do not broadly resolve merge conflicts or revert another agent's work. Do not reset, clean, stash, move branch forcibly, fetch/push/publish, modify credentials or execute next gate without corresponding authority. A candidate result is not main adoption or product acceptance.

## Handoff and tool wrappers
Handoff embeds the exact Task Packet and records base HEAD/tree, candidate HEAD/tree, current_diff (actual diff text or immutable evidence path plus SHA256), completed/failed/unrun verification, known/refuted hypotheses, allowed_paths, remaining_steps and no_go_boundaries. The receiving agent rechecks Git, diff, tests and evidence, not just the previous agent's prose. Preserve worktree ownership; any new development agent receives a new dedicated worktree and reviewed claim transfer.

AGENTS.md, CLAUDE.md and .cursor/rules/grammar-project.mdc only reference this contract with version and SHA256 pins. The source pin is SHA256 of UTF-8 contract text after CRLF-to-LF normalization (canonical UTF8-LF); line-ending conversion alone must not invalidate the pin. Validate pins whenever contract content changes. OpenCode may share AGENTS.md if its installed version/configuration supports it; local OpenCode loading is UNVERIFIED until directly observed. Do not install/configure OpenCode or duplicate rules based on assumptions.

## Usage
`python scripts/validate_governance.py --root . --self-test` validates repository governance and runs in-memory negative checks without modifying product files. Default validation accepts clearly marked non-executable historical examples; `--execution --task ...` refuses examples and verifies current main. Integration optionally passes `--result ... --task ... --execution` to verify candidate identity and actual changed paths. Claims are validated on every run. Runtime admission requires an active matching exclusive path claim covering authorized paths; releasing a claim prevents reuse. This is a small standard-library validator, not an orchestration service, agent launcher or a claim that tools loaded their wrapper successfully.
