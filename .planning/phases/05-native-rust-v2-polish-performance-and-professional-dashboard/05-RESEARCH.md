# Phase 5: Native Rust V2 polish, performance and professional dashboard - Research

**Researched:** 2026-09-29  
**Domain:** Direct Win32/Rust rendering, Windows shell integration, and visual-quality remediation  
**Confidence:** MEDIUM

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Runtime and memory
- Keep a single native Windows process with a 30-second HID poller; no WebView or managed UI runtime.
- Prefer GDI/Win32 primitives and an `.ico` file over GDI+ image decoding in the resident process.
- Report both working set and private bytes because Windows shared DLL pages inflate working set.
- Compile release with LTO, stripping, a single codegen unit, `panic=abort`, and size optimization.

### Dashboard visual direction
- Preserve the dark midnight dashboard, fluorescent green battery ring, restrained red refresh action, and custom title bar from the approved concept.
- Add the real application icon to title bar and system tray.
- Correct baseline alignment for the 24 h/ahora labels and center the chart vertically within its card.
- Add professional depth with soft card shadows and a low-contrast ring aura; avoid noisy animations.

### Interaction and correctness
- Window has only minimize-to-tray and close-to-tray controls; no maximize.
- Invalid HID responses never become 0%; preserve the explicit invalid state.
- Keep native tray actions: Abrir panel, Actualizar perfil, Cerrar programa.

### the agent's Discretion
- Fine-tune colors, stroke widths, spacing, and icon fallback behavior against the approved screenshot.

### Deferred Ideas (OUT OF SCOPE)

New HID profile-setting reads, autostart, notifications, installer work, and a full WebView-free vector mascot system remain separate from this UI/performance refinement.
</user_constraints>

## Project Constraints (from AGENTS.md)

- Keep the target to Windows plus Attack Shark R5 Ultra through its 2.4 GHz receiver.
- Keep telemetry and calibration local; do not transmit usage data.
- Do not present a time estimate as exact without sufficient history; communicate confidence/calibration.
- Maintain low background resource use and do not interfere with mouse latency.
- Profile identity remains limited to polling rate, Competitive Mode, Motion Sync, and sleep time.
- Use the active GSD workflow for repository changes.

## Summary

The current V2 renderer is a sound low-memory native shell, but it draws all visual primitives with legacy GDI. The observed defects are traceable to that implementation: `Arc` does not provide the controlled anti-aliased, round-capped stroke required for a premium ring; chart x-coordinates are normalized between the first and last retained sample instead of the fixed 24-hour domain; the title, button, and axis labels are drawn from hard-coded left offsets rather than measured/centred text rectangles. The existing code also destroys/reloads an icon for each branded paint and does not update the tray tooltip following new HID state. [VERIFIED: `src-tauri/src/native_v2.rs:501-555`, `src-tauri/src/native_v2.rs:559-612`, `src-tauri/src/native_v2.rs:820-900`, `src-tauri/src/native_v2.rs:933-947]

Use a narrowly scoped hybrid renderer: retain a buffered GDI paint surface for cards, text, chart clipping, and native shell compatibility, then use GDI+ only for anti-aliased static dashboard vector geometry (ring, aura, chart line/area) and exactly one cached, downscaled mascot bitmap while the panel is visible. This is the minimum-risk route to correct rounded caps and quality antialiasing without adding a UI framework, renderer process, frame loop, or image decode on the 30-second poller. Microsoft documents that GDI+ owns PNG/ICO decoders, while buffered painting prevents per-paint setup overhead when initialized on the drawing thread. [CITED: https://learn.microsoft.com/en-us/windows/win32/gdiplus/-gdiplus-using-image-encoders-and-decoders-use] [CITED: https://learn.microsoft.com/en-us/windows/win32/api/uxtheme/nf-uxtheme-beginbufferedpaint]

**Primary recommendation:** Replace the V2 `WM_PAINT` path with `BufferedPaintInit` + `BeginBufferedPaint` once per UI thread, render a single coordinate system from the approved UI contract, cache real brand assets once, use GDI+ only for the quality-critical ring/chart primitives, and make every layout hit-test derive from the same named `Rect` constants used for paint.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Native dashboard paint and layout | Win32 client renderer | GDI+ vector layer | A single `WM_PAINT` implementation owns visual geometry and text measurement. |
| Rounded external window | Win32 window manager | DWM on Windows 11 | A region is the cross-version fallback; DWM is an optional Windows 11 visual hint. |
| Battery ring and chart quality | GDI+ vector layer | Buffered GDI surface | GDI+ supplies anti-aliasing and line-cap control; buffer prevents flicker. |
| Original branding | Native asset cache | Shell icon handles | PNG mascot is panel-only; multi-frame ICO serves title/taskbar/tray. |
| Tray percent/status | Shell notification area | HID snapshot | The UI worker updates `NIM_MODIFY` after valid or invalid state changes. |
| Refresh click and title controls | Win32 input / hit testing | HID worker | Both use shared bounds and never cause a paint-driven device read. |

## Standard Stack

### Core

| Technology | Version | Purpose | Why standard for this phase |
|---|---:|---|---|
| `windows` crate | `=0.62.2` | Existing typed Win32 bindings | Already provides the V2 window, GDI, shell, and input APIs. [VERIFIED: `src-tauri/Cargo.toml:25`] |
| UxTheme buffered paint | Windows Vista+ | Flicker-free `WM_PAINT` surface | `BeginBufferedPaint` returns a buffered target DC and Microsoft recommends one `BufferedPaintInit` per drawing thread. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/uxtheme/nf-uxtheme-beginbufferedpaint] |
| GDI+ | Built into Windows | Quality antialiased static arcs/paths and one PNG load | It supports PNG and ICON decoding; restrict it to cached/static UI work. [CITED: https://learn.microsoft.com/en-us/windows/win32/gdiplus/-gdiplus-using-image-encoders-and-decoders-use] |
| Shell notification area | Windows Shell | Real tray icon, tooltip, menu | `Shell_NotifyIconW` supports add, modify, delete, and version negotiation. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw] |

### Supporting

| Technology | Purpose | When to use |
|---|---|---|
| `CreateRoundRectRgn` + `SetWindowRgn` | Deterministic rounded `WS_POPUP` silhouette, including Windows 10 | Apply once after creation and again only if client size/DPI changes. The system owns the HRGN after success. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createroundrectrgn] [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn] |
| `DwmSetWindowAttribute` corner preference | Windows 11-native external rounding/shadow hint | Use only on a non-region Windows 11 path; Microsoft says region windows cannot use DWM rounding. [CITED: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners] |
| `LoadImageW` | Load the multi-frame `.ico` once per app lifetime | Use for title/taskbar/tray; destroy unshared icon handles deterministically. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadimagew] |

### Alternatives Considered

| Instead of | Could use | Tradeoff |
|---|---|---|
| GDI+ static primitives | Direct2D/DirectWrite | Better GPU/vector API but requires COM/device-loss handling and raises implementation risk for a single fixed dashboard. [ASSUMED] |
| Region rounded window | DWM-only `DWMWCP_ROUND` | Less custom clipping but can be unavailable for customized/floating windows and is Windows 11-only. [CITED: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners] |
| One cached mascot bitmap | Decode the PNG during every paint | Simpler code, but repeats file/decode work and is expressly unsuitable for an idle resident app. [ASSUMED] |

**Installation:** No external package is recommended. Extend the existing `windows` feature list only to expose GDI+, UxTheme, and DWM bindings as needed. [VERIFIED: `src-tauri/Cargo.toml:25`]

## Package Legitimacy Audit

No external package installation is proposed. The implementation uses the existing `windows` crate and Windows system DLLs.

## Architecture Patterns

### System Architecture Diagram

```text
HID worker (30 s / manual refresh)
          |
          v
  validated battery snapshot ----> history persistence
          |
          +----> tray state mapper ----> NIM_MODIFY (icon + tooltip)
          |
          +----> InvalidateRect
                              |
                              v
                         WM_PAINT
                              |
                  Buffered DC / fixed layout scene
                    |             |             |
                    v             v             v
             GDI text/cards  GDI+ ring/chart  cached brand asset
                              |
                              v
                      EndBufferedPaint commit
```

### Recommended Project Structure

```text
src-tauri/src/
├── native_v2.rs          # Win32 lifecycle, input routing, tray and paint dispatch
├── native_v2/layout.rs   # Named logical rectangles, DPI scale and hit-test helpers
├── native_v2/render.rs   # Buffered paint, card/chrome/text/vector functions
├── native_v2/assets.rs   # Lifetime-owned .ico handles and cached panel bitmap
└── native_v2/tray.rs     # Add/modify/delete and safe tooltip builder
```

### Pattern 1: One logical geometry source

**What:** Define each geometry element once as named logical `Rect` data. Derive both the draw rectangle and click hit test from that value. Do not use magic coordinate fragments in both branches.

**When to use:** All panel regions, especially title controls and the red refresh CTA.

**Implementation direction:** The source currently paints the button with one set of numbers and checks it with a different hard-coded inequality. Quote: `RoundRect(dc, 852, 872, 1080, 932, 12, 12);` and `} else if y > 872 && x > 852 {`. [VERIFIED: `src-tauri/src/native_v2.rs:168-190`, `src-tauri/src/native_v2.rs:559-575] Use one `REFRESH_RECT.contains(point)` condition with no unbounded right/bottom edge.

### Pattern 2: Buffered, no-animation scene repaint

**What:** Initialize buffered paint once after creating the UI thread; begin/end it inside `WM_PAINT`; invalidate only after valid state changes, manual refresh result, or the existing cadence.

**When to use:** Whole dashboard painting. This removes visible flicker without creating an animation loop.

**Example:**

```rust
// Source: Microsoft BeginBufferedPaint documentation
let paint_buffer = BeginBufferedPaint(target_dc, &paint_rect, format, None, &mut buffered_dc);
if !paint_buffer.is_invalid() {
    render_scene(buffered_dc, snapshot, layout, assets);
    EndBufferedPaint(paint_buffer, true)?;
}
```

The native V2 code already invalidates only on the timer and manual refresh: `let _ = SetTimer(Some(hwnd), 1, 30_000, None);` and `let _ = InvalidateRect(Some(hwnd), None, false);`. [VERIFIED: `src-tauri/src/native_v2.rs:115-117`, `src-tauri/src/native_v2.rs:149-156`, `src-tauri/src/native_v2.rs:193-195]

### Pattern 3: Antialiased ring as a scene layer

**What:** Render track first, value arc second, then two low-alpha wider arcs behind only the value arc. Use a single start angle at twelve o'clock, clamp input to the reported 0–100 range, and convert percent to at most one full turn.

**When to use:** Valid readings only. Invalid reading paints track plus muted glyph/text but no red/green fake value.

**Implementation direction:** GDI `Arc` maps start/end points and produces the present jagged/flat-capped result. The source uses `Arc` with a 34-pixel aura and 20-pixel value stroke: `let _ = Arc(`. [VERIFIED: `src-tauri/src/native_v2.rs:848-879] Replace only these ring layers with GDI+ `GdipSetSmoothingMode`, `GdipSetCompositingQuality`, `GdipCreatePen1`, `GdipSetPenStartCap`, `GdipSetPenEndCap`, and `GdipDrawArcI`/a graphics path. This preserves a GDI core while supplying quality geometry. GDI+ documents `DrawArc` as a pen-driven graphics primitive. [CITED: https://learn.microsoft.com/en-us/windows/win32/gdiplus/-gdiplus-using-a-pen-to-draw-lines-and-shapes-use]

**Correct order/direction:** Use screen-angle convention with start at -90° and sweep `clamp(percent, 0, 100) * 3.6`. Paint track first. The value begins at twelve o'clock and sweeps clockwise. At 100%, draw a full ellipse/path rather than a 360° round-capped arc, preventing the doubled seam. This is an implementation recommendation based on the locked UI contract. [ASSUMED]

### Pattern 4: Fixed-domain chart, clipped quality series

**What:** The plot domain is `now minus 24 hours` to `now`, not first sample to last sample. Convert every observation directly into this fixed domain; clip the series and alpha under-fill to the plot rectangle.

**When to use:** Always, including sparse history. Show a single dot for one valid sample and an explicit empty state for zero samples.

**Implementation direction:** Current code instead derives x from `first.at`/`last_at`: `((sample.at.saturating_sub(first.at)) as f64 / (last_at - first.at) as f64)`. [VERIFIED: `src-tauri/src/native_v2.rs:536-550] This directly causes the `24 h`/`ahora` alignment and trend-scale defect. Use layout constants from the UI contract for label baseline and rectangle edges, then use GDI+ clip state before drawing the filled polygon and 3 px rounded series path. [ASSUMED]

### Pattern 5: Asset lifecycle separation

**What:** Load the multi-size ICO once at startup into owned `HICON` handles for class/taskbar/tray; load and downscale the two large approved PNG assets once only while the panel is visible, then dispose the cached bitmap when hidden or on exit.

**When to use:** Assets never load in `WM_PAINT`, timer, or HID thread.

**Implementation direction:** Existing `draw_brand_icon` calls `native_icon` and then `DestroyIcon` every paint. Quote: `let icon = native_icon();` and `let _ = DestroyIcon(icon);`. [VERIFIED: `src-tauri/src/native_v2.rs:578-583] `native_icon` also requests `64` x `64` from a 16 x 16 ICO; quote: `IMAGE_ICON,` / `64,` / `64,`. [VERIFIED: `src-tauri/src/native_v2.rs:602-609] Microsoft warns a 16x16-only tray icon is scaled unattractively at high DPI and recommends appropriate size frames. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw]

The original `shark-battery.png` and `shark-head-battery.png` are each 1254 x 1254 pixels; decode-to-full-resolution merely to display at 84/28 px wastes resident memory. [VERIFIED: local asset inspection, `R5BatteryEstimator.Native/Assets/shark-battery.png` and `R5BatteryEstimator.Native/Assets/shark-head-battery.png`] Pre-generate approved downscaled panel/title assets at build time, retain their alpha in a DIB section, and `AlphaBlend` them from cached memory. Windows documents `AlphaBlend` for semitransparent bitmap display but notes scaling uses `COLORONCOLOR`; therefore resize once using high quality during build/cache creation rather than relying on `AlphaBlend` to resample at every paint. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-alphablend]

### Pattern 6: Native tray state update

**What:** Keep one stable notification-area identity and call `NIM_MODIFY` with `NIF_ICON | NIF_TIP` after every valid, invalid, manual, or periodic snapshot change. After `NIM_ADD`, call `NIM_SETVERSION` using `NOTIFYICON_VERSION_4` and include `NIF_SHOWTIP` when standard hover text is required.

**When to use:** App initialization and state update, never per paint.

**Implementation direction:** Existing tray data uses `NIF_MESSAGE | NIF_ICON | NIF_TIP` but its tip is hardcoded: `"R5 Battery Estimator V2"`. [VERIFIED: `src-tauri/src/native_v2.rs:933-946] The new mapper must return exactly the UI contract’s valid/invalid text rather than fake 0%. Microsoft identifies `NIF_TIP` as the flag that makes `szTip` valid and `NIM_MODIFY` as the operation that modifies the icon belonging to its original ID. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw] [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw]

### Anti-Patterns to Avoid

- **Legacy GDI `Arc` as a premium progress ring:** it is the source of pixelated edges/caps. Keep it only for fallback graphics; do not layer wider opaque arcs as a simulated glow.
- **Calling `SetWindowRgn` and expecting DWM rounded corners:** Microsoft explicitly says windows with regions cannot receive DWM rounding. Use region as cross-version fallback or DWM as the Windows 11 strategy, not both. [CITED: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners]
- **Hard-coded, independently positioned text:** `DrawTextW` currently always uses `DT_LEFT | DT_TOP | DT_SINGLELINE`; quote: `DrawTextW(dc, &mut wide, &mut rect, DT_LEFT | DT_TOP | DT_SINGLELINE);`. [VERIFIED: `src-tauri/src/native_v2.rs:427-436] Use `DT_CENTER | DT_VCENTER | DT_SINGLELINE` for all visual-centre groups and measured bounds for axis endpoints.
- **Generic `IDI_APPLICATION` branding fallback as normal operation:** only use it after an explicit asset load failure, and provide the outlined-battery fallback in the panel.
- **Unbounded click checks:** every point must be constrained by all four edges of a named rectangle.
- **PNG decode or icon creation per paint/poll:** it inflates working set and creates unnecessary handles.

## Don't Hand-Roll

| Problem | Do not build | Use instead | Why |
|---|---|---|---|
| Rounded Win32 outer mask | Pixel-by-pixel transparent corners | `CreateRoundRectRgn` + `SetWindowRgn` fallback or DWM corner preference | OS owns correct clipping and region lifetime after setting it. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn] |
| Tray lifecycle | Custom polling/tooltip overlay | `Shell_NotifyIconW` add/modify/delete/version | Shell owns status-area behavior and hover invocation. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw] |
| Buffered painting | Ad-hoc compatible-DC allocation every redraw | UxTheme buffered paint | Its thread initialization avoids per-operation internal setup/teardown. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/uxtheme/nf-uxtheme-beginbufferedpaint] |
| Ring antialiasing | Multiple opaque GDI arcs | GDI+ smoothing/line caps for a static vector layer | Correct curve coverage, cap geometry, and compositing are already provided by Windows. [ASSUMED] |
| Icon high-DPI assets | Upscale one 16px ICO | Multi-frame ICO + `LoadImageW` per appropriate metric | Microsoft warns that 16px-only icon scaling is unattractive. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw] |

## Common Pitfalls

### Pitfall 1: The ring looks inverted, square-capped, or pixelated

**What goes wrong:** The bright value can appear underneath an opaque dark aura, the sweep can visually run counter-clockwise, and the end looks clipped.

**Why it happens:** The current path uses opaque legacy GDI `Arc` strokes, leaves angle semantics implicit, and paints aura/value with unrelated widths. [VERIFIED: `src-tauri/src/native_v2.rs:838-879]

**How to avoid:** Track → low-alpha aura(s) → value arc; clockwise twelve-o’clock mapping; antialiased GDI+ arcs; round caps only for non-full values; full ellipse at 100%. Test 0, 1, 50, 77, 99, and 100.

**Warning signs:** A green/dark ring appears reversed, 100% has a double seam, or the same 77% label does not align with the corresponding arc endpoint.

### Pitfall 2: Rounded window becomes square or loses clipping

**What goes wrong:** Borderless `WS_POPUP` paints a square rectangle or a region fights the DWM corner hint.

**Why it happens:** A custom popup has no standard non-client frame, and Microsoft documents that a region excludes the app from DWM rounding.

**How to avoid:** Apply one explicit path at `WM_CREATE`: (Windows 10-compatible) rounded region and custom 1px inner border; or (Windows 11) non-region frame/DWM rounded preference. Validate the actual target OS, then do not layer both.

**Warning signs:** Square corners, exposed rectangular background corners, or missing shadow after using a region. [CITED: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners]

### Pitfall 3: Real art disappears or the UI memory jumps

**What goes wrong:** Generic icon replaces shark art, or loading 1254px PNGs repeatedly grows working set.

**Why it happens:** Current V2 only loads the ICO from beside the executable and falls back to `IDI_APPLICATION`; it does not own a cached panel mascot. [VERIFIED: `src-tauri/src/native_v2.rs:586-612]

**How to avoid:** Ship a multi-frame ICO and small pre-rasterized 28px/84px branded assets. Load them once per panel show/lifetime; dispose unshared `HICON`/bitmaps on shutdown/hide as the selected cache policy requires. Microsoft lists `DestroyIcon` for icons loaded without `LR_SHARED`. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadimagew]

### Pitfall 4: Chart labels look aligned but evidence is misleading

**What goes wrong:** The `24 h`/`ahora` labels sit on fixed ends but the data is scaled to the first and last sample; a short run looks like a day-long trend.

**Why it happens:** Current x calculation divides by `last_at - first.at`. [VERIFIED: `src-tauri/src/native_v2.rs:536-550]

**How to avoid:** Preserve 24-hour x mapping independent of data availability. Centre y labels around the exact grid lines; align both x labels to a shared baseline derived from one `plot_rect`.

### Pitfall 5: Tray hover does not reveal percent

**What goes wrong:** The static tooltip never changes as HID state changes.

**Why it happens:** V2 only calls `NIM_ADD` with static text. [VERIFIED: `src-tauri/src/native_v2.rs:933-947]

**How to avoid:** Factor `update_tray(snapshot)` and invoke it on initial state, worker update marshalled to UI, and manual refresh. Use a bounded null-terminated `szTip` writer; never use it to masquerade an invalid result as zero.

## State of the Art

| Old approach | Current approach | Impact |
|---|---|---|
| Per-paint raw GDI scene | Buffered paint with a fixed static scene | Removes flicker and avoids repetitive buffer setup when thread-initialized. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/uxtheme/nf-uxtheme-beginbufferedpaint] |
| One low-resolution shell icon | Multi-frame icon tailored to shell metric | Prevents visibly poor high-DPI tray scaling. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw] |
| DWM rounding assumed for custom windows | DWM preference or explicit region selected by platform | Avoids unsupported combined behavior. [CITED: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | GDI+ static vector calls plus a cached downscaled bitmap remain within the approved native memory budget on the target hardware. | Summary / Pattern 3 / Pattern 5 | Could exceed the private-byte target; measure open/hidden before approving. |
| A2 | Direct2D would add more implementation/device-loss risk than the needed static GDI+ vector layer. | Alternatives | Could leave potential rendering quality/performance unused. |
| A3 | Full ellipse at exactly 100% best avoids a cap seam in the chosen GDI+ calls. | Pattern 3 | Needs visual snapshot verification. |

## Open Questions

1. **Exact visual acceptance of the original assets at 28px and 84px**
   - What we know: The approved originals are each 1254 x 1254 PNG and the UI contract specifies their placements.
   - What's unclear: Whether exported downscaled variants preserve the user-approved crop at Windows scaling modes.
   - Recommendation: Produce side-by-side 100%/125% visual snapshots and get user acceptance before replacing the source art.

2. **Windows 10 vs Windows 11 rounded-window route**
   - What we know: DWM rounding is supported from Windows 11 build 22000 and region windows cannot use it.
   - What's unclear: Whether the owner needs a Windows 10 visual fallback at this milestone.
   - Recommendation: Implement the region fallback first because it meets the fixed rounded silhouette contract cross-version; add DWM-only polish only after snapshot validation.

3. **Percent display in taskbar hover vs notification-area hover**
   - What we know: Native tray standard tooltip comes from `NOTIFYICONDATAW.szTip` using `NIF_TIP`.
   - What's unclear: The user’s wording may mean the notification-area icon, not a taskbar button thumbnail.
   - Recommendation: Ensure the tray tooltip contains the live percentage/status first; only add taskbar thumbnail tooltip behavior if user still reports it missing.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| Rust toolchain | Native V2 build | ✓ | `1.98.1` [ASSUMED from project stack; not reprobed in this research] | — |
| Windows GDI/GDI+/UxTheme/DWM/Shell DLLs | Renderer and tray | ✓ | System API | No third-party runtime; retain plain GDI fallback only for catastrophic GDI+ initialization failure. |
| Existing `windows` crate | Typed API bindings | ✓ | `=0.62.2` [VERIFIED: `src-tauri/Cargo.toml:25`] | — |

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | No | Local single-user utility; no account boundary in phase scope. |
| V3 Session Management | No | No remote session in phase scope. |
| V4 Access Control | No | No user-controlled privilege boundary is introduced. |
| V5 Input Validation | Yes | Bound `percent` to reported 0–100 before rendering and bound tooltip writes to `szTip`; treat HID result as invalid rather than displaying it as 0. |
| V6 Cryptography | No | No cryptographic feature is introduced. |

### Known Threat Patterns for direct Win32 rendering

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Native GDI/HICON handle leak on repeated repaint | Denial of service | Lifetime-own cached handles; destroy every unshared icon/bitmap exactly once. |
| Unbounded shell tooltip copy | Tampering / denial of service | Fixed-size, null-terminated bounded copy into `NOTIFYICONDATAW.szTip`. |
| Unchecked HID percent used as geometry | Denial of service | Validate/clamp before sweep calculation; invalid state has no value arc. |

## Sources

### Primary (official Microsoft documentation)

- [SetWindowRgn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowrgn) — region clipping, coordinate semantics, and ownership transfer.
- [CreateRoundRectRgn](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createroundrectrgn) — rounded-region construction.
- [Apply rounded corners in desktop apps](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners) — DWM preference and region incompatibility.
- [Shell_NotifyIconW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw) and [NOTIFYICONDATAW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw) — shell lifecycle, `NIM_MODIFY`, tooltip/icon flags, high-DPI icon guidance.
- [BeginBufferedPaint](https://learn.microsoft.com/en-us/windows/win32/api/uxtheme/nf-uxtheme-beginbufferedpaint) — buffered DC lifecycle and per-thread initialization.
- [LoadImageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadimagew) — ICO file load and deterministic icon cleanup.
- [AlphaBlend](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-alphablend) — transparent bitmap composition and scaling constraint.
- [Using GDI+ image encoders and decoders](https://learn.microsoft.com/en-us/windows/win32/gdiplus/-gdiplus-using-image-encoders-and-decoders-use) — built-in PNG/ICON decoder support.

### In-repository primary evidence

- [VERIFIED: `src-tauri/src/native_v2.rs:501-555`] — chart geometry and time normalization.
- [VERIFIED: `src-tauri/src/native_v2.rs:559-612`] — refresh-button geometry and per-paint icon load.
- [VERIFIED: `src-tauri/src/native_v2.rs:820-900`] — legacy-GDI ring/aura implementation.
- [VERIFIED: `src-tauri/src/native_v2.rs:933-947`] — static tray registration/tooltip.
- [VERIFIED: `.planning/phases/05-native-rust-v2-polish-performance-and-professional-dashboard/05-UI-SPEC.md`] — locked visual geometry and interaction contract.

## Metadata

**Confidence breakdown:**

- Standard stack: MEDIUM — Windows APIs were checked in Microsoft documentation; exact GDI+ Rust binding names need compile confirmation during implementation.
- Architecture: MEDIUM — grounded in the current renderer and official lifecycle APIs; visual quality choice is deliberately constrained by the project’s low-memory requirement.
- Pitfalls: HIGH — directly evidenced in the current native renderer plus official window/tray documentation.

**Research date:** 2026-09-29  
**Valid until:** 2026-10-29
