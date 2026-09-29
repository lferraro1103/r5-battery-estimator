# Phase 5: Native Rust V2 polish, performance and professional dashboard - Context

**Gathered:** 2026-09-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Refine the existing direct-Win32 Rust V2 preview without reintroducing Chromium, WebView, .NET, or a local server. The result is a compact professional dashboard with the existing R5 battery data, a visible native identity, exact geometry for the history chart, subtle depth/glow, and a measured low resident footprint.
</domain>

<decisions>
## Implementation Decisions

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
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src-tauri/src/native_v2.rs` owns the direct Win32 window, tray, telemetry worker, history, estimator, and GDI drawing.
- `src-tauri/icons/icon.ico` is the lightweight distribution icon.

### Established Patterns
- GDI resources are selected then deleted in the same drawing function.
- The resident worker owns HID reads and persists only valid readings.

### Integration Points
- The release preview is copied to `outputs/rust-v2-preview/` with its `.ico` companion.
</code_context>

<specifics>
## Specific Ideas

The user wants a panel visually as polished as the supplied dark reference: custom compact title bar, centered typography, large illuminated battery ring, aligned history chart, and material-looking cards.
</specifics>

<deferred>
## Deferred Ideas

New HID profile-setting reads, autostart, notifications, installer work, and a full WebView-free vector mascot system remain separate from this UI/performance refinement.
</deferred>
