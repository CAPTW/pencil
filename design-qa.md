# Result Box and Action Hierarchy QA

- Source visual truth: `D:\dev-temp\user\codex-clipboard-cd0525bf-7a04-412c-a64c-9622e3bf659f.png`
- Implementation screenshot: `D:\dev-temp\grammar-result-layout-after-bottom-20260826.png`
- Semantic hierarchy screenshot: `D:\dev-temp\grammar-result-layout-semantic-qa-20260826.png`
- Minimum-window screenshot: `D:\dev-temp\grammar-result-layout-min-window-20260826.png`
- Result-plus-error regression source: `D:\dev-temp\user\codex-clipboard-2cbec40f-eefe-4a6f-84fd-78168c0d853d.png`
- Result-plus-error corrected preview: `D:\dev-temp\grammar-result-error-layout-after-20260826.png`
- Combined comparison: `D:\dev-temp\grammar-result-layout-comparison-20260826.png`
- State: Translation mode, empty editable result, result actions visible; synthetic semantic preview additionally covers guidance and warning styles.
- Viewport: Tauri window capture 436 x 469 px; semantic preview frame 410 x 358 px.
- Source dimensions: 451 x 182 px focused bug crop.
- Implementation dimensions: 436 x 469 px full Tauri window capture.
- CSS size and density normalization: default desktop CSS at device scale 1; no resampling. The source is a focused crop, so exact full-window pixel comparison is out of scope.

## Full-view comparison evidence

The source provides only the affected result-area crop. The implementation full-window capture confirms that the normal translation controls, rewrite mode controls, toggles, result card, scrollbar, and window frame remain intact. The focused region is compared directly in the combined comparison image.

## Focused region comparison evidence

The combined comparison places the reported overlap next to the corrected Tauri rendering. In the corrected state, the textarea retains its own complete border, a separate horizontal divider precedes the action row, and both buttons remain below the result box without covering its content.

## Findings and comparison history

### Iteration 0 — reported state

- P0: The textarea could keep a 132 px minimum height while its flex parent collapsed to a smaller height. The action row then occupied the same visible area, blocking editing and obscuring the result.
- P1: Recommended result text, instructions, guidance, and warnings did not have a sufficiently clear typographic and semantic-color hierarchy.

### Iteration 1 — populated result followed by an error

- P0: The earlier preview omitted the external `.error-row`. With populated rewrite details, the `252px` result-card flex basis remained shorter than its children; the action row overflowed the card and overlapped the following error by `28.5px`.
- Root cause: the result card used a fixed flex basis instead of content-based sizing, while child result and action regions were non-collapsing.
- Correction: the card now uses `flex: 1 0 auto`, and the external error row is non-collapsing with `flex: 0 0 auto`.

### Fixes applied

- Made the panel vertically scrollable when the window becomes shorter than its content.
- Gave the result card and result area non-collapsing flex bases and minimum heights.
- Added a dedicated divider and fixed flex boundary above the result action row.
- Raised recommended result text to 16 px / 500 weight.
- Kept instructions and guidance at 10 px with separate muted and green semantic colors.
- Kept warnings at 9 px with amber treatment and errors at 10 px with red treatment.
- Strengthened the result-card and textarea borders while retaining the existing product palette and radii.

### Post-fix evidence

- Actual Tauri rendering at 436 x 469 px shows no box/button overlap.
- Actual Tauri rendering at the configured 360 x 320 px minimum also preserves the complete textarea border, divider, and action row; overflow moves to the panel scrollbar.
- The focused semantic preview shows the recommended result as the dominant text and instruction, guidance, warning, and action layers as visually distinct.
- Computed-style evidence: result 16 px / 500 / rgb(23, 32, 27); instruction 10 px / rgb(106, 119, 112); guidance 10 px / rgb(53, 99, 79); warning 9 px / rgb(134, 90, 22).
- Computed geometry reports `overlaps: false`; the action region has a 1 px separator and 10 px top padding.
- Browser-rendered semantic preview contains the textbox and both action buttons, with zero console warnings or errors.
- The populated-result-plus-error preview expanded the card from `252px` to `305.5px`; actions remained inside the card, the error cleared the actions by `25px`, and browser console warnings/errors remained zero.
- Static layout regression contract, retained frontend contracts, typecheck, external production build, and `git diff --check` all pass.

## Required fidelity surfaces

- Fonts and typography: passed. Existing system font stack is preserved; size, weight, line height, and color now establish the requested hierarchy.
- Spacing and layout rhythm: passed. Result content and action controls have separate bordered regions and cannot overlap at reduced heights.
- Colors and visual tokens: passed. Recommended, instruction, guidance, warning, and danger text use explicit semantic tokens with readable contrast.
- Image quality and asset fidelity: passed. This region contains no raster assets; existing Lucide application icons remain unchanged.
- Copy and content: passed. Production copy is unchanged; synthetic copy appears only in the external QA preview.

## Remaining P3 polish

- The vertical scrollbar is intentionally visible at short window heights so all controls stay reachable. Its styling remains the native WebView2 treatment.

final result: passed
