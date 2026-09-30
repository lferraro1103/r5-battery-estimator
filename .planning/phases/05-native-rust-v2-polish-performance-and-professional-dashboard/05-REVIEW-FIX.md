---
phase: 05-native-rust-v2-polish-performance-and-professional-dashboard
fixed_at: 2026-09-30
review_path: .planning/phases/05-native-rust-v2-polish-performance-and-professional-dashboard/05-REVIEW.md
iteration: 2
findings_in_scope: 11
fixed: 11
skipped: 0
status: all_fixed
---

# Phase 05: Code Review Fix Report

**Source review:** `05-REVIEW.md`
**Iteration:** 1, all severities
**Summary:** 11 findings addressed, 0 skipped. `all_fixed` refers to the reported defects and misleading UI. Automatic consumption profiles remain a future capability; WR-08 is resolved through the review's explicit pending-UI alternative.

## Integración y verificación final del orquestador

Todos los commits fuente se integraron en la copia principal. La revisión independiente detectó y cerró un caso adicional de normalización: observaciones distintas en el mismo segundo se conservan y cortan ciclos, y solo se eliminan duplicados idénticos (`0d84ff2`). Verificación final `05-REVIEW-VERIFY.md`: clean, cero hallazgos de código pendientes en el alcance revisado.

- En la copia principal pasan **16 pruebas automatizadas**; 1 prueba física permanece ignorada. Build release y `scripts/package-v2.ps1` pasan, incluidos portable e instalador reconstruidos con el último arreglo.
- Copia personal actualizada en `H:\Juegos\R5 Battery Estimator\R5BatteryEstimatorV2.exe`; SHA256 coincide con el portable nuevo. EXE anterior conservado como `R5BatteryEstimatorV2.exe.pre-audit.bak`.
- Se detectó y corrigió también el valor Run persistido en este Windows: tenía barras invertidas delante de las comillas. Ahora conserva la misma ruta H, correctamente entrecomillada. El test de inicio real de sesión sigue pendiente.
- Consulta real de diagnóstico: `cargo run --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --bin r5-battery-probe` devolvió `hid_error` por marcadores no coincidentes y `Función incorrecta` (OS error 1). No se modificó ni amplió el parser para aceptar respuestas no validadas. La aceptación física del protocolo sigue pendiente; no se considera probada por los tests.
- Los perfiles automáticos continúan pendientes de un protocolo validado para los cuatro ajustes. La acción engañosa se deshabilitó y la limitación está documentada.

La sección siguiente conserva la evidencia del primer pase aislado; sus cifras de 15 pruebas corresponden a ese pase, no al estado final integrado.

## Fixed Issues

### CR-01: Cada sondeo usa un guard HID independiente

**Status:** fixed: requires human verification
**Files modified:** `src-tauri/src/battery/transport.rs`
**Commits:** `f84b60a`, `f77c19f`
**Applied fix:** Independently constructed transports share the process-wide lane and generation. A worker-owned RAII lease survives a timeout and releases only when actual work finishes, including unwind. Timeout generation invalidation uses compare-exchange so an older timeout cannot invalidate a newer request. The application creates one transport in its serialized worker.
**Verification:** Injected blocking work actually exceeds the budget. A second independently constructed transport is Busy while that work remains blocked; releasing a late successful 99% reading does not deliver it to the next request, which receives its own 42% reading. No simulated test invokes real HID.

### CR-02: El refresco manual detiene el hilo de mensajes de Windows

**Status:** fixed: requires human verification
**Files modified:** `src-tauri/src/native_v2.rs`
**Commit:** `f017bb3`
**Applied fix:** Startup, tray refresh and button refresh use one bounded, coalescing request channel and one worker. No Win32 handler calls HID or persists history. The worker posts BATTERY_UPDATED after publishing results; the handler updates tray and repaints. History locks are released before disk I/O.
**Verification:** Rust compilation, handler/source inspection, and deadline tests passed. Physical driver/UI responsiveness acceptance remains pending.

### WR-01: La calibración suma segmentos de descarga discontinuos

**Status:** fixed: requires human verification
**Files modified:** `src-tauri/src/native_v2.rs`
**Commits:** `2b31f44`, `dfa61e0`
**Applied fix:** Each cycle must independently start at ≥95% without charging, reach ≤5% without charging, and last at least 30 minutes. Charging, increases, non-increasing dates and gaps over 600 seconds reset unfinished cycles. Completed cycles contribute elapsed time and drop once; incomplete segments never contribute. Rapid distinct charging/percentage observations are retained rather than replacing the prior sample.
**Verification:** Fixtures exercise charging, percentage increases, gaps, missing complete cycles, and two independently complete cycles separated by an incomplete recharge segment. Weighted duration is asserted numerically.

### WR-02: El historial cargado no se valida y se sobrescribe sin reemplazo atómico

**Status:** fixed: requires human verification
**Files modified:** `src-tauri/src/native_v2.rs`, `src-tauri/Cargo.toml`
**Commit:** `6707d87`
**Applied fix:** Current and migrated history remove invalid percentages, future dates and samples older than fourteen days, then sort and deduplicate timestamps. Persistence creates a unique file in the same directory, writes/flushed/syncs all bytes, closes it and calls Windows MoveFileExW with REPLACE_EXISTING and WRITE_THROUGH. Replacement failure removes the temporary while preserving the previous file. The UI does not share a lock held during persistence.
**Verification:** Real Windows filesystem roundtrip and replacement pass. Injected replacement failure checks staged JSON, exact old-file bytes and temporary cleanup. Validation fixtures assert date/range/order/dedup behavior.

### WR-03: Autoinicio escribe barras invertidas literales en el valor Run

**Status:** fixed
**Files modified:** `src-tauri/src/native_v2.rs`
**Commit:** `c60fff1`
**Applied fix:** Run value uses ordinary double quotes around the executable path, without literal escape backslashes.
**Verification:** Unit test asserts the exact command for a path containing several spaces. Actual login startup has not been exercised; it remains a physical acceptance item.

### WR-04: README no coincide con el historial y la calibración actuales

**Status:** fixed
**Files modified:** `README.md`
**Commit:** `5403ac5`
**Applied fix:** English first and equivalent Spanish now describe the active rust-v2-history.json path and legacy migration, validated atomic history, deadline cadence, individual full-to-low cycle requirements, bounds and weighting, provisional/learned confidence labels, actual optional Run startup and portable relocation. Roadmap no longer describes existing autostart as unimplemented.
**Verification:** Compared both language sections with the final source implementation.

### WR-05: Ventana de tamaño fijo no se adapta a DPI ni al área de trabajo

**Status:** fixed: requires human verification
**Files modified:** `src-tauri/src/native_v2.rs`, `src-tauri/Cargo.toml`
**Commit:** `0808085`
**Applied fix:** Enable per-monitor-v2 DPI awareness; fit the desired DPI size within the current work area on startup, DPI/display changes and completed moves. The original native 1140×916 design is rendered into a GDI bitmap and uniformly scaled, retaining the original GDI+ ring, real blur, shark PNG and all drawing geometry. Rounded window region follows the actual client size; signed input coordinates map back to the logical layout. Maximize remains unavailable through WS_POPUP. The full shark's embedded resource is used if the external PNG cannot supply an icon; tray green/yellow/red behavior is retained.
**Verification:** Tests cover 96/144/192 DPI, laptop and very small work areas, aspect preservation, and refresh/minimize/close target centers. Compilation passed. Actual multi-monitor rendering and high-DPI visual acceptance remain pending. The scaling strategy is raster scaling of the native layout rather than a new responsive design.

### WR-06: El intervalo real añade consulta y persistencia a los 30 segundos

**Status:** fixed: requires human verification
**Files modified:** `src-tauri/src/native_v2.rs`
**Commit:** `f017bb3`
**Applied fix:** One Instant deadline drives queries; normal completion advances to the next scheduled deadline rather than sleeping thirty seconds after I/O. Missed intervals are skipped. Manual refresh leaves a future periodic deadline unchanged. BATTERY_UPDATED replaces the independent UI timer and the panel shows the last valid read time.
**Verification:** Tests assert two-second query duration preserves the next 30-second deadline, manual refresh does not shift it, and overrun advances to a future deadline without catch-up bursts.

### WR-07: Las horas provisionales no están identificadas junto a su valor

**Status:** fixed
**Files modified:** `src-tauri/src/native_v2.rs`
**Commit:** `2d55aa8`
**Applied fix:** Approximate remaining hours have adjacent initial/low-confidence or learned/medium-confidence labels. The full-charge card identifies cycle evidence. The tooltip separately states validated HID telemetry and estimation confidence, with sufficient space for the complete label. Confidence is qualitative and is documented as such.
**Verification:** Tooltip tests assert the actual 156-hour initial and 78-hour learned examples, their distinct labels and the Windows tooltip length budget.

### WR-08: Actualizar perfil solo actualiza batería

**Status:** fixed (UI mismatch); underlying automatic profiles pending
**Files modified:** `src-tauri/src/native_v2.rs`, `README.md`
**Commit:** `10206bd`
**Applied fix:** Replace the misleading action with disabled “Perfiles: pendiente”; command 2 performs no operation. Both README languages disclose battery-only validated protocol, one combined history/model, no settings-change separation, and four-setting automatic profiles still pending. No guessed configuration commands or invented profile values were added.
**Verification:** Source/README inspection and compilation passed. This explicitly does not claim completion of the future profile capability.

### IN-01: El test llamado timeout no ejercita un timeout real

**Status:** fixed
**Files modified:** `src-tauri/src/battery/transport.rs`, `src-tauri/src/native_v2.rs`
**Commits:** `f84b60a`, `f77c19f`, `2b31f44`
**Applied fix:** Replace the manual Busy flag test with injected blocking work, actual timeout, independent process-sharing transports, late successful result and distinct subsequent result. Add cycle fixtures broken by recharge, gaps and increases, plus independently weighted completed cycles.
**Verification:** These bounded tests pass without device I/O.

## Verification and handoff

All gates ran in the **isolated worktree**, not in the main checkout:

- Worktree: `C:/Users/Leaan/Documents/Codex/2026-09-28/tra/.codex/worktrees/rf-05-audit`
- Branch: `gsd-reviewfix/05-audit`
- Base: `c7c250732e68e25e04a34df1e8224bd642341ddd`
- `cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`: 15 tests passed, 0 failed, 1 physical hardware test ignored.
- Targeted checks and rereads were performed for each fix; logic corrections remain flagged for human acceptance above.
- Windows x64 release build passed. The final packaging invocation rebuilt the latest source.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-v2.ps1`: passed, including Inno Setup 6.7.3. Plain invocation initially encountered the local script execution policy; process-scoped Bypass succeeded without changing system policy.
- Portable: `outputs/R5BatteryEstimatorV2/R5BatteryEstimatorV2.exe` (403,456 bytes), complete PNG in `Assets/shark-battery.png`, README and license.
- Installer: `outputs/installer/R5BatteryEstimatorV2-Setup.exe` (2,971,859 bytes).
- Physical receiver, actual login and multi-monitor visual acceptance have not been exercised. No automated result here substitutes for those checks.
- Report is intentionally uncommitted. The orchestrator requested preservation of this checkout/branch for checked integration and cleanup; source commits are complete and no push occurred. Existing main-checkout debug, .gsd and state files were preserved.

---

_Fixer: gsd-code-fixer_
_Iteration: 1_
