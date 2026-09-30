---
phase: 05-native-rust-v2-polish-performance-and-professional-dashboard
reviewed: 2026-09-30
depth: deep
files_reviewed: 12
files_reviewed_list:
  - src-tauri/src/native_v2.rs
  - src-tauri/src/battery/transport.rs
  - src-tauri/src/battery/protocol.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/probe.rs
  - src-tauri/Cargo.toml
  - src-tauri/build.rs
  - src-tauri/tests/protocol.rs
  - src-tauri/tests/hardware_smoke.rs
  - scripts/package-v2.ps1
  - installer/R5BatteryEstimatorV2.iss
  - README.md
findings:
  critical: 2
  warning: 8
  info: 1
  total: 11
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-09-30
**Depth:** deep  
**Files Reviewed:** 12  
**Status:** issues_found

## Summary

Se revisaron los doce archivos del alcance V2 y se cotejaron los diez hallazgos de `Downloads/05-REVIEW.md`. Dos bloqueos siguen vigentes: el guard HID no es compartido por las instancias creadas en cada sondeo y el refresco manual ejecuta I/O en el hilo de mensajes Win32. El aprendizaje mezcla tramos separados; la carga del historial tampoco valida orden/rangos y su escritura puede truncarlo. El problema de comillas del autoinicio también permanece. Cuatro observaciones anteriores corresponden a ejecutables Tauri/C# eliminados y ya no describen código mantenido; seis persisten en V2, incluida la cadencia. La prueba física de hardware sigue marcada `ignore`, por lo que la compilación y las pruebas automatizadas no demuestran funcionamiento con un receptor real.

## Estado de hallazgos de la revisión anterior

| Hallazgo | Estado actual | Evidencia en el árbol actual |
|---|---|---|
| CR-01 — el guard HID no serializa transportes nuevos | Sigue vigente (BLOCKER). La aplicación crea transportes distintos para el sondeo y cada refresco; cada uno tiene sus propios `Arc<AtomicBool>`. | [native_v2.rs:144](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:144), [native_v2.rs:151](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:151), [native_v2.rs:232](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:232), [transport.rs:42](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/battery/transport.rs:42) |
| CR-02 — el sondeo manual bloquea el hilo de UI | Sigue vigente (BLOCKER). `probe_once` es síncrono en ambos comandos de refresco y en el botón. | [native_v2.rs:225](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:225), [native_v2.rs:265](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:265) |
| CR-03 — build .NET depende de un probe Rust inexistente | Ya no aplica. El proyecto C# y su `.csproj` fueron retirados; el build mantenido es Cargo V2. | [Cargo.toml:17](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/Cargo.toml:17), [package-v2.ps1:6](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/scripts/package-v2.ps1:6) |
| WR-01 — comando IPC de Tauri hace HID síncrono | Ya no aplica. No hay comando Tauri/IPC en el runtime actual. | [Cargo.toml:20](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/Cargo.toml:20) |
| WR-02 — cadencia suma duración de sondeo más 30 s | Sigue presente en V2: cada iteración duerme 30 s, consulta y persiste, y recién entonces comienza la siguiente espera. El intervalo entre consultas es 30 s más consulta/persistencia. | [native_v2.rs:149](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:149) |
| WR-03 — calibración combina sesiones distintas | Sigue vigente. Se valida que exista una muestra ≤5%, pero luego se agregan caídas de todos los pares posteriores que cumplan condiciones, incluso a través de recargas/aumentos o huecos. | [native_v2.rs:1174](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:1174), [native_v2.rs:1186](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:1186) |
| WR-04 — historial persistido sin validar y escritura directa | Sigue vigente. Se deserializa y retorna sin validar orden/fechas; una escritura directa puede dejar JSON truncado si el proceso se interrumpe. | [native_v2.rs:1114](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:1114), [native_v2.rs:1169](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:1169) |
| WR-05 — runner C# sin timeout de proceso | Ya no aplica. No hay runner ni proceso probe hijo en la aplicación V2. | [Cargo.toml:17](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/Cargo.toml:17) |
| WR-06 — timer y refresco C# se solapan | Ya no aplica como hallazgo sobre C#; el problema de concurrencia relevante en V2 está cubierto por CR-01 y el bloqueo del hilo por CR-02. | [native_v2.rs:149](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:149), [native_v2.rs:232](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:232) |
| WR-07 — comando Run contiene barras invertidas ante comillas | Sigue vigente. La cadena Rust contiene `\\\"`, así que almacena barras invertidas literales junto a los delimitadores de comillas. | [native_v2.rs:555](/C:/Users/Leaan/Documents/Codex/2026-09-28/tra/src-tauri/src/native_v2.rs:555) |

## Critical Issues

### CR-01: Cada sondeo usa un guard HID independiente

**Severity:** BLOCKER  
**File:** `src-tauri/src/native_v2.rs:144-151,232-233,246-247,276-277`; `src-tauri/src/battery/transport.rs:42-46,57-64`  
**Issue:** La aplicación construye `R5HidTransport::new()` en el sondeo inicial, en el hilo periódico y en cada ruta de refresco. El guard `lane_owned` vive dentro de cada instancia, así que la protección contra consultas simultáneas solo funciona entre clones de una misma instancia. Dos rutas concurrentes pueden abrir el receptor y enviar feature reports al mismo tiempo, con errores/intercalado de transacciones.  
**Fix:** Crear una instancia de transporte compartida para toda la aplicación y pasar clones de esa instancia a los sondeos y refrescos; alternativamente, hacer que el guard de transacción sea global al proceso. La solución debe mantener la ocupación del canal hasta que termine realmente una consulta que excedió el presupuesto.

### CR-02: El refresco manual detiene el hilo de mensajes de Windows

**Severity:** BLOCKER  
**File:** `src-tauri/src/native_v2.rs:225-238,244-252,265-281`  
**Issue:** Los comandos del menú y el botón llaman a `probe_once` de manera síncrona dentro de `window_proc`. La consulta espera hasta dos segundos, más enumeración, apertura y settle; mientras tanto la ventana no procesa repintado ni entrada. Esto reproduce una ventana congelada ante un driver lento o varias interfaces HID.  
**Fix:** Enviar la solicitud a un trabajador serializado y devolver el resultado a la ventana con un mensaje Win32; actualizar estado/tray e invalidar la ventana al procesar ese mensaje.

## Warnings

### WR-01: La calibración suma segmentos de descarga discontinuos

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:1174-1204`  
**Issue:** La función solo requiere que después de la primera muestra no cargando ≥95% haya alguna muestra no cargando ≤5%. Luego suma las caídas de todos los pares elegibles del resto del historial. Si hubo carga/recharge, subida porcentual o hueco mayor a diez minutos, ese límite se salta en vez de cerrar la sesión; las caídas de sesiones separadas pueden formar una duración aprendida que no representa una descarga completa.  
**Fix:** Segmentar el historial en sesiones contiguas y descartar cada sesión al encontrar carga, subida o hueco excesivo; calcular una tasa solo sobre sesión(es) que individualmente satisfagan los umbrales definidos.

### WR-02: El historial cargado no se valida y se sobrescribe sin reemplazo atómico

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:1114-1138,1147-1170`  
**Issue:** El JSON se acepta tal cual, aunque sus muestras estén desordenadas o tengan timestamps futuros. Como el estimador usa ventanas adyacentes, ese orden altera la calibración y las fechas futuras pueden sobrevivir la poda de 14 días. Además, `fs::write` trunca el archivo antes de escribirlo; un cierre/interrupción puede hacer que el siguiente arranque descarte toda la historia corrupta.  
**Fix:** Al cargar, validar `percent <= 100`, timestamps razonables y orden monotónico; rechazar o reparar entradas inválidas. Persistir a un temporal del mismo directorio, sincronizar y reemplazar el archivo anterior de forma atómica.

### WR-03: Autoinicio escribe barras invertidas literales en el valor Run

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:552-563`  
**Issue:** `format!("\\\"{}\\\"", ...)` genera una cadena con `\"ruta\"`, incluyendo los caracteres barra invertida en el valor del registro. Windows Run espera el ejecutable como una ruta normalmente entrecomillada y puede no iniciar la aplicación, especialmente desde una ruta con espacios.  
**Fix:** Construir el valor como `format!("\"{}\"", executable.display())` y validar el comando guardado/inicio de sesión en una instalación real.

### WR-04: README no coincide con el historial y la calibración actuales

**Severity:** WARNING  
**File:** `README.md:23-25,45,94-96,116`; `src-tauri/src/native_v2.rs:1106-1112,1174-1204`
**Issue:** El README indica `%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json`, pero V2 persiste y migra desde `rust-v2-history.json` y `battery-history.json`, respectivamente. La instrucción apunta al archivo de migración heredado, que V2 no actualiza. Además, describe que bastan 3% y 30 minutos para aprender, omitiendo que el código exige una muestra no cargando >=95% y otra posterior <=5%; el usuario puede esperar una calibración mucho antes de que ocurra.
**Fix:** Identificar el historial activo y alinear el algoritmo descrito en inglés y español con la política de ciclos finalmente adoptada al corregir WR-01.

### WR-05: Ventana de tamaño fijo no se adapta a DPI ni al área de trabajo

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:75-80,169-182,265-267`  
**Issue:** El panel, hitboxes y dibujo usan coordenadas constantes de 1140×916 píxeles, sin configurar conciencia de DPI ni recalcular tamaños al cambiar de monitor. En escalado alto Windows puede virtualizar el tamaño/coordenadas; en pantallas pequeñas la ventana puede exceder el área disponible y zonas visibles pueden no coincidir con hitboxes esperadas.  
**Fix:** Hacer el layout escalable por DPI y tamaño disponible, manejar cambios de DPI/monitor y limitar posición/tamaño inicial al área de trabajo.

---

### WR-06: El intervalo real añade consulta y persistencia a los 30 segundos

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:149-154,293-296,444`  
**Issue:** El worker espera 30 segundos, consulta y guarda el historial. El siguiente inicio ocurre después de sumar esas duraciones. El timer de UI tiene un reloj independiente y puede repintar justo antes de llegar la lectura nueva, conservando la anterior hasta el próximo timer. El texto «Se recalcula cada 30 segundos» no describe con precisión esa coordinación.
**Fix:** Usar deadlines en el worker y notificar la llegada del resultado al hilo UI. Mantener un solo reloj de consulta y publicar la hora de la última lectura.

### WR-07: Las horas provisionales no están identificadas junto a su valor

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:430-451,1207-1212,1658-1663`  
**Issue:** Sin calibración, `remaining_hours(None, 78)` devuelve 156 h por la referencia fija de 200 h. La tarjeta y tooltip muestran «Autonomía» como una cifra entera y no indican junto a ella que es provisional ni el nivel de confianza. «Aprendiendo» aparece en otra tarjeta y no explica el origen del valor del tooltip. Esto incumple la restricción de precisión del proyecto, aunque la fórmula aritmética sea correcta.
**Fix:** Mostrar «estimación inicial»/«confianza baja» en la autonomía y tooltip hasta disponer de evidencia personalizada, y diferenciar confianza de lectura HID de confianza de estimación.

### WR-08: Actualizar perfil solo actualiza batería

**Severity:** WARNING  
**File:** `src-tauri/src/native_v2.rs:225-252,498`; `src-tauri/src/battery/protocol.rs:21-33`; `README.md:39,73,110,144`  
**Issue:** El comando 2 «Actualizar perfil» ejecuta la misma consulta de batería que el comando 4. No lee polling rate, Competitive Mode, Motion Sync o reposo; el historial no guarda identidad de perfil y el estimador utiliza un único conjunto de muestras. Cambiar ajustes del mouse no separa sus consumos. README lo lista en roadmap, pero el menú presenta una acción de perfiles ya disponible.
**Fix:** Implementar lectura validada de los cuatro ajustes e historial/modelos por perfil antes de ofrecer esa acción, o identificar claramente en UI que esa funcionalidad está pendiente. Es una capacidad faltante frente a los requisitos, no una regresión introducida por la limpieza.

## Cobertura pendiente

### IN-01: El test llamado timeout no ejercita un timeout real

**Severity:** INFO  
**File:** `src-tauri/src/battery/transport.rs:145-154`; `src-tauri/src/native_v2.rs:1223-1316`  
**Issue:** `timeout_returns_without_publishing_late_result_or_overlapping` solo activa manualmente `lane_owned` y comprueba Busy. No ejecuta una consulta que exceda el presupuesto, un resultado tardío ni dos instancias de transporte. Tampoco hay prueba que combine una recarga con suficientes intervalos válidos para revelar la calibración entre sesiones. Los siete tests verdes no cubren estas condiciones.
**Recommendation:** Inyectar una consulta controlable para probar timeout/concurrencia y agregar fixtures de dos ciclos separados por carga, huecos y aumentos de porcentaje. No ejecutar pruebas HID simuladas contra el dispositivo real.

## Verificación de esta sesión

- Siete pruebas automatizadas pasan; la prueba física del receptor quedó ignorada.
- Build release V2 Windows x64 y generación del instalador pasan tras retirar el código anterior.
- Confirmación secundaria del orquestador: revisó el código de transporte, handler Win32, historial y autoinicio; corrigió el estado WR-02 y añadió WR-06..08 e IN-01 con evidencia del árbol actual.
- Cuatro hallazgos anteriores ya no aplican por eliminar C#/Tauri; seis siguen presentes en V2. No se aplicaron fixes funcionales nuevos como parte de esta revisión.
- El fallo genérico de icono en la notebook no se reprodujo en esta sesión. La recuperación TaskbarCreated no prueba su causa.

_Reviewed: 2026-09-30_
_Reviewer: the agent (gsd-code-reviewer)_  
_Depth: deep_
