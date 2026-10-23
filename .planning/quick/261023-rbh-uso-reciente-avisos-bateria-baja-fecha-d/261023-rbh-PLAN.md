---
quick_id: 261023-rbh
slug: uso-reciente-avisos-bateria-baja-fecha-d
mode: quick
created: 2026-10-23
depends_on: 261023-qz3
---

Mejoras 1–3 propuestas tras 261023-qz3, más el pedido del usuario: una visual y unos textos más profesionales.

## Task 1 — Peso del uso reciente
- files: src-tauri/src/native_v2.rs
- action: en `estimate()`, cada par pesa 0.5^(edad/7 días) respecto de la última muestra, tanto en caída como en tiempo. La confianza sigue dependiendo de la caída observada sin ponderar.
- verify: test donde el consumo se duplica en la última semana y la estimación se acerca al ritmo nuevo.

## Task 2 — Avisos de batería baja
- files: src-tauri/src/native_v2.rs
- action:
  - Función pura `low_battery_alert(notified, percent, charging)`: avisa a 20% y a 5%, una vez por descarga. Se rearma al cargar o al superar el 25%.
  - Notificación de Windows desde el ícono de bandeja (NIF_INFO), en el hilo de UI.
- verify: test de umbrales, de repetición y de rearmado.

## Task 3 — Fecha estimada de carga
- files: src-tauri/src/native_v2.rs
- action: "Cargar antes del jue 4 nov", calculada con los días restantes a tu ritmo. Fecha en español sin depender del locale.
- verify: test del formato de fecha.

## Task 4 — Visual y textos
- files: src-tauri/src/native_v2.rs
- action:
  - `render_panel()` separado de WM_PAINT, más `--render-png <ruta>` para revisión visual.
  - Encabezado: "Batería del R5 Ultra", un subtítulo descriptivo y una píldora con el estado del receptor.
  - Anillo coloreado por nivel (verde/ámbar/rojo), igual que el ícono de bandeja.
  - Tarjetas: cifra con su unidad ("de uso activo") y filas "A tu ritmo", "Confianza", "Cargar antes del".
  - Gráfico de 7 días con promedio horario, ejes 0/50/100 % y fechas.
  - Footer y botón neutro; tooltips más claros.
- verify: PNG antes/después, `cargo test`, build release y reinstalación.
