---
quick_id: 261023-rbh
status: complete
completed: 2026-10-23
commit: 808faa8
---

Las mejoras 1–3 están implementadas, junto con una revisión visual y de textos que pidió el usuario a mitad de la tarea.

## Cambios (src-tauri/src/native_v2.rs)
- **Peso del uso reciente**: `estimate()` pondera caída y tiempo con una vida media de 7 días; la confianza sigue usando la caída observada sin ponderar. En el historial real la duración pasó de 69,5 h a 78,4 h, porque la última semana consumió menos.
- **Avisos de batería baja**:
  - `low_battery_alert()` avisa a 20% y a 5%, una vez por descarga, y se rearma al cargar o al superar el 25%.
  - `check_low_battery()` muestra la notificación de Windows (NIF_INFO + NIIF_WARNING) desde el hilo de UI tras cada lectura.
- **Fecha de carga**: `charge_by()` + `spanish_date()` → "Cargar antes del jue 5 nov", independiente del idioma de Windows.
- **Visual**:
  - Título "Batería del R5 Ultra", subtítulo descriptivo y píldora de estado del receptor (en línea / en reposo / no detectado). Se quitó el "R5 ULTRA" rojo duplicado.
  - El anillo y el ícono de batería se colorean por nivel (verde ≥55, ámbar ≥25, rojo), igual que el ícono de bandeja.
  - `metric_card()`: cifra + unidad + filas "A tu ritmo", "Confianza" y "Cargar antes del".
  - Gráfico de 7 días con promedio horario, ejes 0/50/100 % y fechas en cada medianoche local.
  - Botón azul neutro en lugar de rojo.
- **Textos**:
  - Estados: "En batería", "Último valor conocido", "Sin lectura".
  - Footer: "Última lectura: HH:MM · actualización automática cada 30 s".
  - Tooltip: "R5 Ultra · 43% · ~34 h de uso · ~12 días · cargar antes del jue 5 nov".
- **Desarrollo**: `render_panel()` separado de WM_PAINT; `--render-png <ruta>` guarda el panel con el historial local, sin ventana ni HID.

## Verificación
- `cargo test`: 30 pasan, 2 ignorados (hardware e historial local).
- Tests nuevos:
  - recencia
  - umbrales y rearmado de avisos
  - mensaje del aviso
  - fecha en español
  - promedio horario y medianoches
  - etiquetas de tiempo
  - tooltips
- Tres asserts de tasas mixtas pasaron a tolerancia relativa del 3%, o a un rango, por el ponderado.
- Revisión visual con `--render-png`: estado real (43%, 78 h, ~29 días, cargar antes del jue 5 nov) y estado vacío (Calibrando, sin datos).
- Release compilado y reinstalado en `H:\R5 Indicator\R5 Battery Estimator\`. Backup del exe anterior: `R5BatteryEstimatorV2.exe.bak-261023-qz3`. La app relanzada registró una lectura nueva (43%).

## No verificado
- La notificación de batería baja no se vio en pantalla: la batería está en 43% y no se forzó.
