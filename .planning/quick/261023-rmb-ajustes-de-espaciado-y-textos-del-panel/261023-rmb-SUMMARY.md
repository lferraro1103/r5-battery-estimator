---
quick_id: 261023-rmb
status: complete
completed: 2026-10-23
commit: 8f3bb20
---

- El gráfico volvió a y=530 con plot 604–752: el espacio entre el anillo y el gráfico es otra vez el original (~57 px).
- Textos: "R5 Ultra" / "Autonomía estimada"; filas "Duración en días" y "Días restantes"; estado del anillo "En reposo".
- `--render-png <ruta> --asleep` renderiza el estado de mouse en reposo.
- Verificación: render despierto y en reposo revisados. `cargo test`: 30 pasan, 2 ignorados. Release reinstalado en H:\R5 Indicator; backup `R5BatteryEstimatorV2.exe.bak-261023-rbh`.
- No verificado: después del reinicio el mouse estaba en reposo y no llegó una lectura nueva en ~3 min (la lectura HID no cambió).
