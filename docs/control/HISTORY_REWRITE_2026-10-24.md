# History rewrite — 2026-10-24

Owner-requested exception to the no-history-rewrite / no-force-push rules of the
canonical contract, for one purpose only: remove a personal e-mail address from
commit and tag metadata.

## What changed

- Every commit and annotated tag whose author, committer or tagger e-mail was the
  personal address now uses `CAPTW <122129944+CAPTW@users.noreply.github.com>`.
  Names, dates, messages and file trees are unchanged (trees verified identical
  on `main`, `codex/grammar-local-trial-ba58b7d` and
  `claude/practical-bardeen-gj4d9g`).
- Because commit metadata is part of each SHA, every commit SHA changed. The full
  mapping is in [commit-map-2026-10-24.tsv](commit-map-2026-10-24.tsv)
  (`old_commit` → `new_commit`).
- Force-updated with lease on the previous SHA: `main`,
  `codex/grammar-local-trial-ba58b7d`, `codex/grammar-autonomous-r1`,
  `claude/eloquent-faraday-hh62qc`, `claude/practical-bardeen-gj4d9g`.

Release tags are re-pointed separately by the Owner (tag pushes are not permitted
from the cloud session):

| Tag | Old target | New tag object → commit |
| --- | --- | --- |
| `v0.1.0-prototype.1` | tag `b6ee3c6` → `49fb641` | `c137cf1` → `3c43fe5` |
| `v0.1.0-prototype.2` | tag `5d4e517` → `7951abf` | `d13a07d` → `152992f` |
| `v0.1.0-prototype.3` | `5d43b2b` | `19a352b` |

## Reading older references

Release assets (MANIFEST.json, BUILD_RECEIPT.json, VALIDATION.json), HANDOFF,
control state, path claims and other historical documents cite pre-rewrite SHAs.
They remain valid historical identifiers: look them up in the map to find the
equivalent commit. Published release ZIPs were not rebuilt or modified.

## Existing clones

A clone made before this rewrite has diverged history. With no unpushed work,
run `git fetch origin` and reset each local branch to its `origin/` counterpart.
With unpushed local commits, rebase only those commits onto the rewritten branch
(`git rebase --onto origin/<branch> <old origin/<branch> SHA> <branch>`). Do not
push the old history back. Set `user.email` to the noreply address before new
commits.
