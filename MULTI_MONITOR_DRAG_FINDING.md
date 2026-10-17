# Finding: Mixed-DPI Window Dragging on Windows

**Research updated:** 2026-10-16

## Summary

The reported behavior is a known Windows bug in the `winit` mixed-DPI handling path, not an application-specific layout problem. The local monitor arrangement matches the conditions in a Slint report: the higher-DPI primary monitor is on the right, the lower-DPI monitor is on the left, and the window grows while being dragged across the boundary.

Both `TimeRecon` and `vs_hub` use `eframe`, which uses `winit` for native windows. The dependency resolved during the investigation is `winit 0.30.13`; the bug is still reproducible in that version. Slint also uses `winit` and has a report of the same issue.

**Correction to the earlier finding:** the previous explanation attributed the growth to Aero Snap and speculated that `winit`'s synthetic `WM_MOUSEMOVE` could desynchronize drag tracking. The public reports do not support that diagnosis. Windows Snap may affect snapping at screen edges, but the documented runaway growth is caused by the `winit` `WM_DPICHANGED` handling described below.

## Local Setup

The two monitors share a vertical edge at `x = 0`:

- **`\\.\DISPLAY1` (primary, right):** `(0, 0)` to `(1920, 1200)`, 120 DPI (125% scale)
- **`\\.\DISPLAY2` (left):** `(-1920, 0)` to `(0, 1200)`, 96 DPI (100% scale)

Windows reports `WindowArrangementActive = 1`, `DockMoving = 1`, and `SnapSizing = 1`. These settings can affect normal Windows snapping, but they are not evidence for the mixed-DPI resize loop.

## Documented Cause

When a per-monitor-DPI-aware window crosses to a monitor with a different scale, Windows sends `WM_DPICHANGED`. Its `lParam` contains a suggested outer rectangle for the window on the target monitor. Microsoft's guidance says to resize and reposition the window with that rectangle using `SetWindowPos`.

In `winit 0.30.13`, the Windows backend instead calculates a conservative rectangle and checks the current window's monitor using `MonitorFromWindow`. During a drag, the window can still be mostly on the monitor it is leaving. The monitor check and nudge logic can therefore move the window back toward that monitor, causing another DPI transition and another size adjustment. Repeated transitions produce the visible size growth, jumping, or failure to land on the destination monitor.

The fix is in the windowing layer: use Windows' suggested rectangle in the affected DPI-transition path rather than forcing the rectangle back to the old monitor. This is consistent with the Microsoft `WM_DPICHANGED` guidance and is independently reported for applications using `winit`, including `eframe` and Slint.

## Public Reports and Fix Status

| Source | What it establishes | Status checked 2026-10-16 |
|---|---|---|
| [`winit` #4041](https://github.com/rust-windowing/winit/issues/4041) | Original Windows 11 24H2 report: mixed-DPI monitor dragging grows and shifts the window. | Closed after an upstream fix was merged to `master`. |
| [`winit` PR #4341](https://github.com/rust-windowing/winit/pull/4341) | Fixes #4041. On Windows 11+, it uses the OS-suggested rectangle instead of the problematic conservative adjustment; it retains the legacy adjustment for Windows 10. | Merged to `master`; this change is not in stable `winit 0.30.13`. |
| [`winit` #4600](https://github.com/rust-windowing/winit/issues/4600) | Follow-up confirms the problem still occurs with `winit 0.30.13`, both directly and through `eframe`, and documents a tested patch. | Open; updated 2026-10-12. |
| [`winit` PR #4694](https://github.com/rust-windowing/winit/pull/4694) | Backport proposal for the `0.30.x` branch: apply the suggested rectangle during a live move/resize. | Open, not merged. Its author reports testing through `eframe 0.36.2`; a later comment reports success on Windows 11 24H2 with mixed-DPI monitors. |
| [`egui` #7648](https://github.com/emilk/egui/issues/7648) | `egui` users report repeated scale-factor changes, bad pointer position, and incorrect resizing between monitors. A comment points to the underlying `winit` issue. | Open. |
| [`Slint` #11073](https://github.com/slint-ui/slint/issues/11073) | Same monitor arrangement and abnormal growth; a Slint user reports fixing it in `winit`, and Slint maintainers identify the `winit` backend. | Open; its 2026-09-28 update says the fix is available on the `winit 0.31` line or through a `0.30` backport. |
| [Microsoft `WM_DPICHANGED` documentation](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged) | Defines the suggested rectangle and recommends using it with `SetWindowPos`. | Current platform guidance. |

## Local A/B Validation

**Tested:** 2026-10-16 on the two-monitor setup above, using both `TimeRecon` and `vs_hub` with `eframe 0.36.2`.

The unmodified build used registry `winit 0.30.13`. The patched build used the `winit` revision from [PR #4694](https://github.com/rust-windowing/winit/pull/4694), commit `39c4009c41e5977d779cf8a365792c8153ca77c5`, through a temporary Cargo patch in an isolated copy. `cargo tree -i winit` confirmed the patched revision was selected. Both builds succeeded.

The `TimeRecon` A/B comparison started at the same 1000x750 physical size on the 120 DPI monitor. Its native title bar was dragged across the boundary with the foreground window verified. `vs_hub` was separately tested from the same starting size using its custom header drag:

| Build and direction | Before | After | Result |
|---|---|---|---|
| Unmodified `winit 0.30.13`, 125% to 100% | 1000x750 at 120 DPI | 1955x1466 at 120 DPI; window remained straddled across the boundary | Reproduced runaway growth and failure to switch DPI. |
| PR #4694 patch, 125% to 100% | 1000x750 at 120 DPI | 800x600 at 96 DPI on the secondary monitor | Correctly moved and preserved the logical size. |
| PR #4694 patch, 100% to 125% | 800x600 at 96 DPI | 1000x750 at 120 DPI on the primary monitor | Correctly moved back without growth. |
| PR #4694 patch in `vs_hub`, custom header, 125% to 100% | 1000x750 at 120 DPI | 800x600 at 96 DPI on the secondary monitor | `ViewportCommand::StartDrag` moved correctly without growth. |
| PR #4694 patch in `vs_hub`, custom header, 100% to 125% | 800x600 at 96 DPI | 1000x750 at 120 DPI on the primary monitor | Moved back correctly without growth. |

This validates the `0.30.x` backport proposal through the current `eframe` stack; it is not a direct run of `winit 0.31`. `eframe 0.36.2` currently resolves `winit 0.30.13`, so the 0.31 development line cannot be substituted directly into this app.

## Solution Options

### 1. Prefer an Upstream `winit` Release with the Fix

The fix from PR #4341 is on `winit`'s `master`/0.31 development line. As of this research, crates.io lists `0.30.13` as the latest stable release and `0.31.0-beta.3` as a prerelease. Moving to the newer line requires checking whether the current `eframe` version supports it; a beta upgrade should not be assumed to be drop-in.

### 2. Keep the Tested `0.30.x` Backport Temporarily

For the current `eframe`/`winit 0.30.13` stack, PR #4694 is the directly relevant backport proposal. The local A/B test above confirmed it fixes mouse dragging in `TimeRecon` through `eframe 0.36.2`. The exact PR commit is now temporarily pinned in both `TimeRecon/Cargo.toml` and `vs_hub/Cargo.toml`.

The PR is not merged as of the date above, so the pin uses a fork commit and should be removed when an official `winit 0.30.x` release contains the fix. A test comment notes that this live-drag-only backport does not fix the separate `Win+Arrow` / `Win+Shift+Arrow` cross-monitor placement issue that the broader #4341 change addresses. If keyboard-based snapping also matters, compare against #4341 rather than assuming the narrower backport covers it.

### 3. Avoid an App-Level Drag Rewrite as the First Fix

Changing `vs_hub` to manually move the window, subclassing `WM_DPICHANGED` in each app, or disabling Windows Snap would work around symptoms at a different layer. The cross-framework reports and the tested `winit` patches point to the shared windowing backend, so patching or upgrading `winit` is the more direct and maintainable solution.
