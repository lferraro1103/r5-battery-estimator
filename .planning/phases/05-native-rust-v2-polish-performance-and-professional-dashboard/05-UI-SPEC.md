---
phase: "5"
slug: "native-rust-v2-polish-performance-and-professional-dashboard"
status: draft
shadcn_initialized: false
preset: none
created: "2026-09-29"
---

# Phase 5 — Native Rust V2 polish, performance and professional dashboard — UI Design Contract

> Canonical visual and interaction contract for the direct-Win32 Rust preview. It deliberately preserves the approved WinForms dashboard’s information hierarchy while fixing the V2 regressions: lost shark identity, off-centre text and controls, an unscaled ring, unanchored chart labels, square chrome, and missing material depth. No browser, WebView, managed UI runtime, local HTTP service, or continuous animation is permitted.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none — direct Win32/GDI drawing, buffered paint only |
| Preset | not applicable |
| Component library | none |
| Icon library | product-owned shark assets plus native line glyphs |
| Font | Segoe UI (fallback: system UI sans-serif) |
| Source of visual truth | Approved 1140 × 950 WinForms dashboard reference supplied by the user, then this geometry contract |

The application is a fixed-size dashboard, not a responsive web surface. Its client drawing area is **1140 × 950 logical pixels at 100% DPI**. The renderer must scale the complete coordinate system for per-monitor DPI rather than changing spacing relationships. It has no resize grip and no maximize path.

---

## Asset and Brand Contract

| Surface | Required asset | Presentation contract |
|---------|----------------|-----------------------|
| Title bar | Original shark-head-with-battery app icon | 28 × 28 px, fully visible, no crop, x=20/y=10 in the 48 px chrome; it is not substituted with the Windows application icon. |
| Main panel masthead | Original full chibi shark holding a battery (`shark-battery`) | 84 × 84 px, fully visible with transparent background, x=1000/y=62; it balances the `R5 ULTRA` badge and is never replaced by a generic or monochrome mark. |
| Window icon and taskbar | Same square shark-head-with-battery `.ico` | 16/20/24/32/48/64 px icon frames; no clipping at any shell size. |
| System tray | Dedicated square shark-head-with-battery `.ico`, with a battery fill/status accent | Must remain recognizable at 16 px and use the same brand family as the window icon; it is intentionally distinct from the larger panel mascot. |

The original user-approved art is the source of truth. Asset loading must be outside the 30-second HID worker and never re-decode an image while idle. The panel may cache a pre-rasterized native bitmap only while visible; the tray and title use the `.ico` frames. If a brand asset fails to load, use a deliberate outlined battery fallback, never the generic Windows application icon.

---

## Layout Geometry

All coordinates below are relative to the 1140 × 950 client area. They are a contract, not approximate suggestions.

| Region | Bounds / anchor | Contract |
|--------|-----------------|----------|
| Window root | 0,0 → 1140,950 | Clip all painting to a 24 px rounded outer mask. Draw a 1 px `#23313B` border inside that mask. The outer corners must be visibly rounded, never a square full-bleed rectangle. |
| Custom title bar | 0,0,1140,48 | 48 px tall, `#0D161D`; 1 px bottom divider. Entire empty region drags the window. It contains icon/title on the left and only minimize/close on the right. |
| Title-bar branding | icon 20,10,28,28; text x=66, y=13 | `R5 Battery Estimator` is vertically centred in the 48 px bar; the title’s text baseline and icon optical centre align. |
| Window controls | minimize hitbox 1028,0,56,48; close hitbox 1084,0,56,48 | Both glyphs are centred in their own hitboxes. Minimize and close hide to the tray; there is no maximize button, hitbox, icon, or keyboard route. Close hover is red-tinted; minimize hover is a subtle neutral surface. |
| Masthead | title x=60/y=78; subtitle x=62/y=143 | Main title ends before x=760, preserving whitespace before the badge. The subtitle is directly below and aligned to the title’s optical left edge. |
| Badge and mascot | badge right-aligned at x=990/y=91; mascot 1000,62,84,84 | `R5 ULTRA` sits visually centred against the mascot’s middle third. Badge and mascot cannot overlap the title. |
| Battery ring module | x=58/y=198/w=305/h=305 | Entire ring and all internal text are inside these bounds; no glow, arc, glyph, or status text may clip below y=503. |
| Full-charge card | x=390/y=265/w=350/h=230 | Align its top and bottom exactly with the remaining-autonomy card. |
| Remaining-autonomy card | x=765/y=265/w=340/h=230 | Align its top and bottom exactly with the full-charge card. |
| History card | x=38/y=570/w=1064/h=280 | Its visual centre is y=710. Content is vertically centred as a group, not shifted down toward the footer. |
| Footer note | x=50/y=895 | Icon and text form one vertically centred group in the 60 px footer band. |
| Refresh action | x=852/y=872/w=228/h=60 | 24 px corner radius. The text-and-icon group is centred as a whole, not positioned with independent left offsets. |

### Visual Hierarchy

The live **battery ring is the dashboard's primary visual anchor**: it receives the first glance through its scale, fluorescent value arc, restrained aura, and central percentage. The full-charge card explains calibration, while **remaining autonomy is secondary to the live battery reading**: it is prominent enough to scan but must never compete with, replace, or visually outweigh the ring. The history chart is tertiary evidence below the fold; the red refresh action is the only action accent and must not become a competing focal point.

### Battery Ring

The ring is an information instrument, not a generic progress circle.

- Centre: `(210, 350.5)`; circular artboard: `305 × 305`; visual ring inset: 34 px.
- Track: 22 px rounded stroke, `#24323D`, with an open 12 o’clock start/end cap only when the percentage is below 100%.
- Value arc: 22 px rounded stroke, `#2FE189`; map **exactly 0–100 reported percent to 0–360 degrees**. It must not use an arbitrary partial scale or render more than one full revolution. 0% shows only the track; 100% is a complete green ring with no seam gap.
- Aura: two or three low-alpha concentric green strokes outside the arc, fading into the background. The aura must be soft and radial, never a hard dark-green donut or a rectangular glow. It is static and repainted only when the window paints.
- Percent: horizontally centred at x=210.5. Use the display role at y=307–371; it is slightly above the icon/status group and must never move when the number changes from `9%` to `100%`.
- Battery glyph: horizontally centred at x=210.5 and vertically centred between the percentage block and status baseline. Top y=393, 40 × 19 outer body plus 5 px terminal. It uses the same green as the value arc when valid and muted grey when invalid.
- Status: horizontally centred at x=210.5, y=430–456. `No cargando`, `Cargando`, or the explicit invalid state must be below the glyph with a clear 12–16 px optical gap. It may not sit on top of the glyph or near the arc.
- Invalid state: show `—%`, a muted grey glyph, and `Sin lectura válida`; do not change the ring to 0%, red, or a made-up time.

### Metric Cards

Each metric card uses the same interior grid: 28 px left/right padding, 38 px top padding, 28 px bottom padding. The content is left-aligned; the **content block** is vertically balanced within the 230 px card, so a one-line label, large value, and note never appear top-heavy or favour one side.

| Card | Icon anchor | Label | Value | Supporting copy |
|------|-------------|-------|-------|-----------------|
| Full charge | clock centre 429,320 | x=449/y=305 | x=420/y=365/w=290, centred only for the value | x=420/y=445 |
| Remaining autonomy | bars x=802, baseline 320 | x=826/y=305 | x=795/y=365/w=270, centred only for the value | x=795/y=445 |

`Aprendiendo` must use the complete 290 px value field and be visually centred in that field; it must have equal perceived left and right margin. Do not clip its descender or its support line. Values are large but do not overflow: if a learned number would exceed the field, use a compact, honest formatting such as `1.2k h`, never shrink unrelated UI or allow clipping.

### History Chart

| Element | Exact bounds / alignment |
|---------|--------------------------|
| Card title glyph | bars at x=92, baseline 622 |
| Heading | x=120/y=605; vertically centred with the glyph |
| Plot rectangle | left=145, top=665, right=1068, bottom=799 |
| Y labels | `100%` right-aligned to x=130 and vertically centred on y=665; `0%` right-aligned to x=130 and vertically centred on y=799 |
| X labels | `24 h` left-aligned at x=145; `ahora` right-aligned at x=1068; both share a y=815 top and the same baseline |
| Grid | 1 px low-contrast dashed lines only inside the plot rectangle; four horizontal intervals and three internal vertical divisions |
| Series | 3 px green rounded line; translucent green area fades from 18% alpha beneath the line to 0% at the plot bottom; it is clipped to the plot rectangle |

The X axis always represents the real previous 24 hours, not the span between the first and last sample. Map `now − 24h` to x=145 and `now` to x=1068. No sample may be invented to fill empty time. The Y axis is always 0–100%; map 100% to y=665 and 0% to y=799. A single valid sample uses a single dot; zero valid samples use the documented empty state rather than a diagonal or rising placeholder line. Charging transitions may be drawn from observations but must not be misrepresented as a discharge trend.

---

## Material, Color, and Effects

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | `#0A1117` | Window body and negative space |
| Secondary (30%) | `#101B23` | Cards, chart container, title-bar-adjacent surfaces |
| Secondary border | `#233440` | 1 px card/root borders and chart dividers |
| Primary text | `#F7F9FC` | Main title and all primary values |
| Muted text | `#BED0E9` | Subtitle, labels, notes, axes, and informational copy |
| Accent (10%) | `#2FE189` | Battery value arc, valid battery glyph, chart series/area, tray battery status only |
| Action red | `#E8373D` | `Actualizar ahora`, close-hover only |
| Warning | `#F2C94C` | Optional medium-battery tray fill/status only; never chart/ring for a valid percentage |
| Critical | `#EB5757` | Optional low-battery tray fill/status only; never a disconnection state |

Accent reserved for: valid battery percentage arc, battery glyph, chart series/area, and the battery-fill portion of the tray icon. It is not used for arbitrary borders, all text, or every clickable item.

Cards use `#101B23`, 16 px corner radii, a 1 px `#233440` border, and a static 4 px/5 px dark shadow. The window root and refresh action use 24 px corner radii. Add restrained depth only: a darker inner edge at the card bottom and the low-contrast ring aura described above. The chart area receives a subtle green under-fill. Do not add moving gradients, animated pulses, noisy particle effects, glossy bevels, or expensive per-frame blur.

---

## Spacing Scale

Declared values (all multiples of 4):

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4 px | Icon terminal details and hairline offsets |
| sm | 8 px | Tight icon/text gaps |
| md | 16 px | Card radius, normal component gaps |
| lg | 24 px | Major internal padding and title-bar control width increments |
| xl | 32 px | Card content rhythm and ring inset baseline |
| 2xl | 48 px | Major dashboard separation |
| 3xl | 64 px | Masthead-to-content rhythm |

There are no spacing-scale exceptions: title-bar height, control hitboxes, and component radii use four-pixel increments. Optical text and glyph-centering coordinates may use measured pixel positions where required to centre a visual asset exactly.

---

## Typography

Use exactly two weights: regular **400** and bold **700**. No condensed, ornamental, or browser-provided font substitution is allowed.

| Role | Size | Weight | Line height | Usage |
|------|------|--------|-------------|-------|
| Label | 12 px | 400 | 1.25 | Card labels, chart axes, supporting notes |
| Body/control | 15 px | 400 | 1.35 | Title-bar title, refresh label, ring status, footer note |
| Supporting heading | 18 px | 400 | 1.25 | Subtitle and `R5 ULTRA` badge (badge is bold) |
| Display | 42 px | 700 | 1.10 | Main product title, ring percent, metric values |

The main title and primary numeric/value text use Display 42 px / 700. `Aprendiendo` uses Display 42 px / 700 within its fixed value field. The title-bar label is Body/control 15 px / 400. Labels remain Label 12 px / 400. All text must be positioned with measured text extents or centred DrawText flags; magic left offsets are not acceptable for any visually centred copy.

---

## Interaction Contract

| Surface | Interaction | Result |
|---------|-------------|--------|
| Empty custom title bar | Drag | Moves the borderless window through the OS move loop. Controls and buttons do not start a drag. |
| Minimize glyph | Click | Hides panel to the system tray; polling and tray remain active. |
| Close glyph or OS close | Click / Alt+F4 | Hides panel to the system tray; it does not quit the resident monitor. |
| Tray left click | Click | Shows and foregrounds the existing panel. |
| Tray right click | Click | Opens native context menu: `Abrir panel`, `Actualizar perfil`, separator, `Cerrar programa`. |
| Tray tooltip | Hover | Valid reading: `R5 Battery Estimator — {percent}% — {Cargando|No cargando}`. Invalid reading: `R5 Battery Estimator — sin lectura válida`; no fake 0%. |
| Refresh action | Click or keyboard activation | Performs one immediate native HID refresh, persists only a valid observation, updates panel and tooltip, and returns to idle. |
| `Actualizar perfil` | Tray menu | Performs the current permitted profile/configuration refresh without creating a fake profile or altering visual state on incomplete data. |
| `Cerrar programa` | Tray menu | Explicitly terminates the resident application. It is the only route that quits the monitor; no confirmation is needed because it does not delete data. |

Keyboard focus indicators are visible on the custom controls and refresh action. Icon-only window controls expose accessible names: `Minimizar a la bandeja` and `Cerrar a la bandeja`. Each window control has a 56 × 48 px hitbox; the refresh action has a 228 × 60 px hitbox.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Primary CTA | `Actualizar ahora` |
| Valid non-charging ring status | `No cargando` |
| Valid charging ring status | `Cargando` |
| Learning value | `Aprendiendo` |
| Learning support | `Se aprende con tu descarga real.` |
| Remaining support | `Se recalcula cada 30 segundos.` |
| Footer information | `La app nunca muestra una desconexión como 0%.` |
| Empty history heading | `Aún no hay historial` |
| Empty history body | `Dejá el R5 Ultra conectado para registrar sus primeras lecturas.` |
| Single-sample chart note | `Se necesita otra lectura para trazar la tendencia.` |
| Error / unavailable state | `No se pudo leer el R5 Ultra. Conservamos la última lectura válida; revisá el receptor y volvé a intentar.` |
| Destructive confirmation | Not applicable: `Cerrar programa` stops monitoring but neither deletes history nor settings. |

Do not show `156 h`, a full-charge duration, or an autonomous value as learned until an accepted full-discharge segment exists. Until then, display `Aprendiendo` for duration and `—` for the estimate if there is no explicitly supported provisional model. This preserves the accuracy constraint and prevents the false confidence already reported by the user.

---

## Performance and Native Rendering Guardrails

- One direct Win32/Rust process; no Chromium, Edge, WebView2, .NET CLR, local HTTP server, or child renderer process.
- One 30-second HID worker and one UI invalidation at that cadence; manual refresh is the only extra read. No polling loop is attached to painting, hover, chart, or animation.
- Use buffered WM_PAINT to prevent flicker. Reuse stock GDI objects where possible and pair every created brush, pen, font, bitmap, menu, and icon with deterministic cleanup.
- Static shadows/glows are layered primitive geometry or cached raster data. No timer-driven fade, frame loop, animated gradient, or image decode runs while the panel is hidden.
- Titlebar/tray icon construction must produce and keep native `HICON` handles only as long as required; do not leak one per repaint.
- Measure idle process **private bytes and working set** with the panel hidden and with it open. The dashboard must not add a browser-runtime-sized memory footprint; treat a regression above the approved native baseline as a release blocker and record the result in the phase verification artifact.

---

## UI Considerations

Applicable state considerations resolved: 8 covered, 2 backstop, 0 unresolved.

| Category | Element(s) | Status | Resolution / Reason |
|----------|------------|--------|---------------------|
| loading | Initial panel and manual refresh | ✅ covered | Preserve the last valid reading while native refresh is in flight; do not blank the dashboard or show 0%. |
| error | Ring, metrics, tooltip | ✅ covered | Invalid HID uses the documented explicit unavailable copy and preserves a last valid value only when it is timestamped as stale. |
| empty | History chart | ✅ covered | Zero valid samples render the documented empty history heading/body, not a fabricated diagonal line. |
| populated | History chart | ✅ covered | Two or more valid samples draw the real 24-hour time-scale series, grid, clipped green under-fill, and anchored axes. |
| partial | Learning/estimate cards | ✅ covered | Insufficient valid discharge evidence shows `Aprendiendo` and an honest unavailable estimate rather than a learned duration. |
| zero-one-many | History chart | ✅ covered | Zero shows empty state; one shows a single point plus the single-sample note; two or more show the series. |
| overflow | Metric values and tray tooltip | ✅ covered | Metric values use compact unit formatting before the measured field would overflow; tray text is concise and omits nonessential details before shell truncation. |
| long-text | Error message, title-bar label, menu labels | 🧪 backstop | Visual regression test at 100%, 125%, and 150% DPI verifies no clipping or overlap; static product/menu strings are not abbreviated. |
| long-text | Card learning value | 🧪 backstop | A long localized/large value fixture verifies measured centring and compact formatting instead of spill or shrinkage. |

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| none | none | not applicable — direct native Win32 surface, no component registry |

---

## Acceptance Screens and Verification

The executor must capture and compare the following screenshots at 100% DPI before presenting a preview:

1. Valid non-charging reading around 75–80%: original shark is visible in title bar and masthead; ring is fully contained; percentage, glyph, and status have the specified vertical rhythm; cards and button are centred; chart labels sit on the same baseline.
2. Valid charging reading below 20%: ring reflects the actual numeric percentage; tray tooltip exposes the actual percentage and `Cargando`; the invalid/disconnected visual is not used.
3. Invalid/mouse asleep reading: no `0%`, no red discharge implication, explicit unavailable copy, tray tooltip says `sin lectura válida`.
4. No history / one history sample / multi-sample history: no fabricated trend; the 24-hour plot bounds, 0–100 scale, and x-axis anchors hold in all three states.
5. Panel hidden in tray: no visible window, native tray icon remains branded and recognizable at 16 px, tooltip shows the current status, and private/working-set measurement stays within the native budget.
6. 100%, 125%, and 150% DPI: root and cards retain rounded corners, all custom chrome controls remain visible, and no label, mascot, glow, or footer content clips.

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS
- [ ] Dimension 7 Inventory Provenance: PASS (not applicable: Tool none)

**Approval:** pending
