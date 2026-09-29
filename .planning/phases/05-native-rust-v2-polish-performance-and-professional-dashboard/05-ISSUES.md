# Phase 5 visual regression register

User-reported regressions recorded before implementation:

- [x] Do not use `shark-head-battery`; use the approved full shark (`shark-battery`) for title-bar, window/tray icon, and panel mascot.
- [x] Restore a visible provisional remaining-autonomy value while the full-charge model is still learning; keep the full-charge card explicitly labelled `Aprendiendo`.
- [x] Restore the tray hover path: the tooltip includes live percent and charging state after shell version negotiation.
- [x] Restore the approved vertical positions for the outlined battery glyph and `No cargando` text.
- [x] Force an outlined, hollow battery glyph so it can never inherit a filled GDI brush.
- [x] Replace the dark arc shadow with layered translucent green emission around the ring.
- [x] Add the same restrained green light treatment behind the history series.
- [x] Align the history bars glyph with the heading's optical centre.

Verification still required from the user on the live desktop preview because the direct Win32 surface is not available to the current automated screen-capture surface.

## Approved polish pass

- [x] Replace clock, autonomy, and history glyph strokes with antialiased vector primitives.
- [x] Align the history and autonomy glyphs to their respective label baselines.
- [x] Raise the battery glyph and status copy slightly as a single group.
- [x] Give the inner progress ring the same real blurred bloom treatment as the chart, without changing the two-ring geometry.
- [x] Vertically balance the complete duration-card content group, including title, value, and supporting copy.
- [x] Use a consistent display type scale for `Aprendiendo` and remaining-autonomy hours.
- [x] Keep the percentage display field invariant between valid and invalid readings.

## User verification follow-up

| Item | Status before this pass | Resolution |
| --- | --- | --- |
| History alignment | Fixed | No change requested. |
| Duration-card type scale and balance | Fixed | No change requested. |
| Autonomy glyph alignment | Fixed in this pass | Raised two pixels against the label's optical centre. |
| Clock glyph | Fixed in this pass | High-quality GDI+ ellipse and pixel-offset mode. |
| Inner-ring bloom | Fixed in this pass | Stronger real blurred mask; two-ring geometry preserved. |
| Percentage/status rhythm | Fixed in this pass | Percentage moved down 4 px; status moved up 5 px. |

## Next user-requested polish pass

- [x] Render the approved full-shark title-bar artwork with high-quality interpolation and close the title/icon gap.
- [x] Make the inner battery progress ring's real blurred bloom visibly read as a contained green, illuminated 3D-style gradient.
- [x] Remove the white specular line from the approved glow treatment, widen the green halo slightly, and tighten the title-bar title gap again.
- [x] Replace the pixelated-feeling card type with a cleaner professional family, keeping one coherent hierarchy for `DURACIÓN DE CARGA COMPLETA`, `Se aprende con tu descarga real.`, `AUTONOMÍA RESTANTE`, and `Se recalcula cada 30 segundos.`.
- [x] Replace the tray art with a battery-state shark face in green, yellow, or red (no battery on its head).
- [x] Switch the card metadata to a more restrained professional typeface, expose remaining autonomy in the tray hover text, and add `Actualizar ahora` to the tray menu.
