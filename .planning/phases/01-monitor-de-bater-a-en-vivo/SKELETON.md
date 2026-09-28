# Walking Skeleton — R5 Battery Estimator

**Phase:** 1  
**Generated:** 2026-09-28

## Capability Proven End-to-End

> Con el R5 Ultra conectado por receptor 2.4 GHz, el usuario puede leer el porcentaje real del dispositivo y una autonomía marcada como provisional desde la bandeja, abrir el panel y ver el mismo snapshot autoritativo.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Framework | Tauri 2.12.0 con host Rust 1.98.1 y panel React 19.3.0/TypeScript 7.0.2 | Mantiene HID, sondeo, bandeja e IPC fuera de la WebView y evita distribuir Chromium o un sidecar. |
| Device boundary | `R5HidTransport` sobre `hidapi` 2.6.7 `windows-native` | Porta el feature report validado sin exponer HID genérico al frontend. |
| Read model | `BatterySnapshot` inmutable, propiedad del worker Rust | Bandeja y panel consumen una sola verdad y distinguen lectura válida, antigüedad y fallo. |
| Data layer | Sin base durable en Phase 1; snapshot en memoria y logs locales acotados | Ningún requisito de esta fase necesita historial durable; SQLite y aprendizaje se incorporarán cuando DATA-01/EST-03 los requieran, sin inventar persistencia prematura. |
| Authentication | No aplica | Aplicación personal, local y sin cuentas, sesiones ni red. |
| Deployment target | Windows 11 x64, bundle NSIS por usuario; Windows 10 x64 secundario | Coincide con el hardware validado y el target MSVC, y permite probar el comportamiento real de bandeja. |
| Directory layout | Dominio/adaptadores bajo `src-tauri/src/battery`, shell en `src-tauri/src`, presentación en `src` | Conserva dependencia hacia adentro y permite sustituir transporte, reloj y scheduler en pruebas. |

## Stack Touched in Phase 1

- [x] Project scaffold: Tauri, React, TypeScript, Vite, Cargo/pnpm y runners de prueba con versiones exactas.
- [x] Routing: comandos/eventos IPC estrechos para obtener y actualizar `BatterySnapshot`; no servidor HTTP.
- [ ] Database: no corresponde a la frontera de Phase 1; no hay lectura/escritura durable hasta una fase con requisito DATA/learning.
- [x] UI: menú nativo de bandeja y panel React conectados al mismo snapshot.
- [x] Deployment: comandos locales reproducibles y bundle NSIS de Windows.

## Binding Execution Spine

1. Portar el transporte Rust/hidapi mínimo y obtener **PASS** contra el probe real, incluida convivencia con el software oficial; FAIL o UNVERIFIED bloquean el resto.
2. Conectar el transporte aprobado al snapshot, bandeja y panel en un tracer vertical.
3. Añadir estados antiguos/desconectados, recuperación, instancia única y métricas residentes provisionales.
4. Finalizar accesibilidad y generar el NSIS reproducible.
5. Cerrar con aceptación instalada: interrupciones reales, una sola instancia y comparación humana A/B durante juego.

Los presupuestos CPU ≤0.5%, memoria oculta ≤120 MiB y p95 HID ≤500 ms son límites provisionales para recolectar evidencia, no un PASS automático de WIN-02.

## Out of Scope (Deferred to Later Slices)

- Lectura, creación o actualización de perfiles; **Actualizar perfil** pertenece a Phase 2 per D-05.
- Historial durable, segmentos de descarga y modelos aprendidos por perfil.
- Autonomías separadas para uso cotidiano y juego continuo.
- Autoinicio, notificaciones de batería baja y preferencias persistentes.
- Updater, nube, cuentas, analytics, soporte Bluetooth o modelos distintos del R5 Ultra.

## Subsequent Slice Plan

Cada fase posterior agrega una capacidad vertical sin mover HID, sondeo, base de datos o reglas de dominio a la WebView:

- Phase 2: reconocer exactamente los cuatro ajustes y recuperar el perfil automático numerado.
- Phase 3: aprender autonomía cotidiana y de juego continuo por perfil con evidencia local y confianza visible.
- Phase 4: operar al iniciar Windows, conservar preferencias y emitir avisos de batería baja deduplicados.
