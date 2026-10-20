# Pencil changelog

## Unreleased source improvements

- Standardize visible app branding on Pencil while retaining compatible settings,
  executable and Native Messaging identifiers.
- Compute proven emphasis alignment without a second model inference; keep model
  alignment for unproven cases and retain semantic/register/source/format guards.
- Add bounded Korean synonym, connective, nested-condition and honorific regression
  coverage with actual local Gemma HWP/HWPX Accept, Undo and saved-file rereads.
- Package rHWP Native Messaging review with original-file protection checks.
- Restrict test cleanup to explicitly marked owned paths; preserve shared AppData.
- Improve model startup failure/retry reporting and Windows main/PR source checks.

These are source changes, not a new executable release or whole-product acceptance.
Read [qualification and limitations](docs/PROTOTYPE.md) and the detailed
[handoff](docs/control/HANDOFF.md). Historical failed measurements remain evidence.

## Published prototypes

See the immutable [GitHub releases](https://github.com/CAPTW/pencil/releases) and
per-package MANIFEST/VALIDATION records for the exact released binaries.
