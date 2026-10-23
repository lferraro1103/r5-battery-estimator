---
quick_id: 261023-qz3
status: complete
completed: 2026-10-23
commit: a4374e6
---

Se reemplazó el aprendizaje por ciclo completo con un estimador por tasa de consumo en `src-tauri/src/native_v2.rs`.

## Cambios
- `estimate()` reemplaza a `learned_hours()`:
  - Suma Δ% neto y tiempo de todos los pares de muestras sin carga. Los pares con hueco ≤10 min cuentan como uso activo; los de hasta 3 días, como calendario.
  - El jitter de ±1% se cancela.
  - Una subida ≥5% o una muestra `charging` es una recarga y no aporta caída ni tiempo.
- Confianza según la caída activa observada: <5% sin estimación, <20% baja, <50% media, ≥50% alta.
- `Estimate` reporta horas de uso activo por carga y días por carga a tu ritmo (con ≥24 h de reloj observadas).
- Historial: retención de 14 → 60 días. `append_sample` compacta las mesetas a una lectura por intervalo activo sin ocultar huecos de reposo o PC apagada.
- UI y tooltip:
  - "Duración de carga completa" muestra las horas activas y "Uso activo · ~N días · confianza X".
  - "Autonomía restante" muestra horas y días restantes.
  - Sin datos suficientes se mantiene la línea base provisional de 200 h.

## Verificación
- `cargo test --target x86_64-pc-windows-msvc`: 25 pasan; 2 ignorados (hardware y el historial local).
- Los tests nuevos cubren:
  - descargas parciales con reposo y PC apagada
  - jitter
  - recarga sin flag
  - calendario vs uso activo
  - umbral de 5%
  - segmentos de carga
  - niveles de confianza
  - compactación
  - retención de 60 días
- Historial real de esta PC (`local_history_produces_an_estimate --ignored`): 3535 muestras dan 69,5 h de uso activo y 28,6 días por carga, confianza alta. Tooltip al 44%: "~31 h uso · ~13 días".
- Build release: OK (`r5-battery-estimator-v2.exe`, 445 KB).

## Notas
- En esta carpeta hay que compilar con un `CARGO_TARGET_DIR` corto: la ruta supera los 260 caracteres y `link.exe` falla con LNK1104.
- Pendiente fuera de alcance: validar con `r5-battery-probe` que el byte de carga reporta `charging = true` al enchufar el mouse.
