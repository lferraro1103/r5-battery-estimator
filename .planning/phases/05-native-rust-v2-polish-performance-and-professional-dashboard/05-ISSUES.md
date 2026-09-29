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
