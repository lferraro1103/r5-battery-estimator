# Phase 1: Monitor de batería en vivo - Research

**Researched:** 2026-09-28  
**Domain:** Tauri 2 / Rust HID / React para Windows  
**Confidence:** HIGH en protocolo observado y paquetes; MEDIUM en recuperación, que requiere prueba con hardware.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Información en la bandeja
- **D-01:** El icono comunica el estado mediante colores y no incrusta el porcentaje como texto diminuto.
- **D-02:** El tooltip usa un resumen compacto con el patrón `R5 Ultra · 73% · 18 h restantes`.
- **D-03:** El clic izquierdo abre un menú pequeño junto a la bandeja, no la ventana detallada directamente.
- **D-04:** El menú contiene la acción **Abrir panel**.
- **D-05:** El menú debe admitir **Actualizar perfil** cuando esa capacidad se implemente en la fase 2; la fase 1 no debe simular una actualización de perfil inexistente.

### the agent's Discretion
- Paleta exacta de colores, iconografía, textos accesibles y comportamiento del clic derecho, siempre que el estado no dependa exclusivamente del color.
- Presentación de los estados de carga, sueño, lectura antigua y desconexión dentro de la ventana detallada.
- Formato y redondeo de la autonomía provisional, manteniendo inequívoca su condición inicial.

### Deferred Ideas (OUT OF SCOPE)
- **Actualizar perfil** pertenece a la fase 2, donde se leerá la configuración y se resolverán perfiles automáticos numerados.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|---|---|---|
| BATT-01 | El usuario puede ver el porcentaje de batería y el estado de carga que informa su R5 Ultra por receptor 2.4 GHz, junto con la hora de la última lectura válida. | Puerto exacto del feature report validado y snapshot con `readAtUtc`. |
| BATT-02 | Si el mouse duerme, deja de responder o se desconecta el receptor, el usuario ve el estado correspondiente y la antigüedad de la última lectura; la app no presenta el fallo como 0%. | Máquina de estados conserva la última lectura y separa ausencia del receptor de fallo de consulta. |
| BATT-03 | Tras despertar el mouse, reconectar el receptor o reanudar Windows, el usuario vuelve a recibir lecturas válidas sin reiniciar la app. | Invalidación del handle, reenumeración y backoff verificables con hardware. |
| EST-01 | Desde la primera lectura válida, el usuario ve una estimación inicial claramente marcada como provisional. | Semilla de 200 h del fabricante, presentada como techo provisional y no calibrado. |
| UI-01 | El usuario puede consultar rápidamente batería, estado y autonomía estimada desde la bandeja de Windows, y abrir una ventana detallada. | Tray nativo Rust y panel React consumen el mismo snapshot. |
| WIN-02 | La app evita instancias duplicadas y consulta el mouse con una cadencia que no perjudica perceptiblemente su respuesta durante el juego. | Plugin oficial de instancia única, un solo worker HID y aceptación de recursos/latencia. |
</phase_requirements>

## Summary

Implementar un único worker Rust como dueño del handle HID, del sondeo y del snapshot autoritativo; bandeja y React solo proyectan ese snapshot. El protocolo local ya fija VID/PID, interfaz candidata, bytes de solicitud, dos layouts de respuesta y rango válido. `[VERIFIED: .planning/spikes/001-current-battery-telemetry/probe/read-battery.js:3-4,11-16,21-23,35-55]`

La fase debe instalar solo lo necesario para HID, tray, instancia única, logging, IPC tipado y pruebas de componentes. Persistencia, autostart, notificaciones, gráficos, updater y perfiles quedan fuera de esta fase. `[VERIFIED: .planning/REQUIREMENTS.md:14-19,33-41; AGENTS.md:184-193]`

**Primary recommendation:** planificar primero el scaffold y fixtures del protocolo, después el worker/estado/reconexión, y finalmente bandeja/panel; cerrar con aceptación en hardware e instalable Windows.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Enumeración, feature reports y reapertura | Backend Rust | Windows HID | El frontend no recibe acceso HID genérico. `[VERIFIED: AGENTS.md:29-33,191-192]` |
| Parser, frescura y estimación provisional | Dominio Rust | — | Produce una sola verdad tipada para todos los consumidores. `[VERIFIED: 01-CONTEXT.md:54-61]` |
| Tray, menú e instancia única | Host Tauri/Rust | — | Debe existir aunque la WebView esté oculta. `[VERIFIED: AGENTS.md:63-67]` |
| Panel detallado y accesibilidad | React/WebView | Rust IPC | Presenta el snapshot, no controla el sondeo. `[VERIFIED: AGENTS.md:29-33,96-104]` |

## Standard Stack

| Library | Exact version | Purpose | Decision |
|---|---:|---|---|
| Rust | 1.98.1 | Toolchain MSVC | Pin en `rust-toolchain.toml`. `[VERIFIED: AGENTS.md:51-57]` |
| `tauri` | 2.12.0 | Shell, IPC y feature `tray-icon` | `[VERIFIED: crates.io + https://v2.tauri.app/release/tauri/v2.12.0/]` |
| `tauri-build` | 2.7.0 | Build script | `[VERIFIED: crates.io + https://v2.tauri.app/release/]` |
| `tauri-plugin-single-instance` | 2.5.0 | Un poller por sesión | Debe registrarse primero. `[VERIFIED: crates.io + https://v2.tauri.app/plugin/single-instance/]` |
| `hidapi` | 2.6.7 | HID con `default-features=false`, `windows-native` | `[VERIFIED: crates.io + https://docs.rs/hidapi/2.6.7/hidapi/]` |
| `serde`, `serde_json`, `thiserror`, `log`, `tauri-plugin-log` | 1.0.229 / 1.0.151 / 2.0.21 / 0.4.34 / 2.10.0 | DTO, fixtures, errores y logs | `[VERIFIED: crates.io; AGENTS.md:67,110-113]` |
| React / React DOM | 19.3.0 | Panel | `[VERIFIED: npm registry + https://react.dev/versions]` |
| `@tauri-apps/api` / CLI | 2.12.0 | IPC y toolchain | `[VERIFIED: npm registry + https://v2.tauri.app/release/]` |
| TypeScript / Vite / plugin React | 7.0.2 / 8.3.1 / 6.1.1 | Build frontend | `[VERIFIED: npm registry + https://vite.dev/guide/]` |
| Zod | 4.6.5 | Validación en el borde IPC | `[VERIFIED: npm registry + https://zod.dev/packages/zod]` |
| Vitest / jsdom / Testing Library | 5.0.2 / 30.1.1 / 16.3.3 / 14.6.7 / 7.0.1 / DOM 10.4.2 | Pruebas de estados UI | `[VERIFIED: npm registry + https://vitest.dev/guide/ + https://testing-library.com/docs/react-testing-library/intro/]` |

Usar versiones exactas y lockfiles. No instalar TanStack Query, Recharts, date-fns, Lucide, SQLite, autostart, notification, updater, `image`, proptest ni WebDriverIO en esta fase. `[VERIFIED: phase requirements in .planning/REQUIREMENTS.md:10-34; AGENTS.md:65-69,100-104,114,121-126]`

## Package Legitimacy Audit

La comprobación obligatoria se ejecutó con `gsd-tools query package-legitimacy check` en los ecosistemas correctos y se contrastaron versiones exactas con crates.io/npm y documentación oficial. Los valores de descargas son la observación del 2026-09-28. `[VERIFIED: package-legitimacy seam + official registries]`

### Cargo

| Package@version | Official registry / repository | Recency and maintenance | Install behavior | Verdict | Rationale / disposition |
|---|---|---|---|---|---|
| `tauri@2.12.0` | crates.io / `tauri-apps/tauri` | 984,131/wk; repo oficial; versión publicada 2026-09-26 | Crate compilado; sin postinstall | OK | Approved |
| `tauri-build@2.7.0` | crates.io / `tauri-apps/tauri` | 979,211/wk; publicada 2026-09-26 | build-dependency Rust | OK | Approved |
| `tauri-plugin-single-instance@2.5.0` | crates.io / `tauri-apps/plugins-workspace` | 309,140/wk; publicada 2026-09-26 | Crate compilado | OK | Approved |
| `hidapi@2.6.7` | crates.io / `ruabmbua/hidapi-rs` | 126,878/wk; proyecto desde 2016; publicada 2026-08-27 | Compila backend `windows-native`; no descarga posinstalación | OK | Approved |
| `serde@1.0.229` | crates.io / `serde-rs/serde` | 25,495,208/wk; proyecto desde 2014 | Crate compilado | OK | Approved |
| `serde_json@1.0.151` | crates.io / `serde-rs/json` | 25,646,578/wk; proyecto desde 2015 | Crate compilado | OK | Approved |
| `thiserror@2.0.21` | crates.io / `dtolnay/thiserror` | 29,884,500/wk; publicada 2026-09-23 | Proc-macro normal | OK | Approved |
| `log@0.4.34` | crates.io / `rust-lang/log` | 22,983,309/wk; proyecto desde 2014 | Crate compilado | OK | Approved |
| `tauri-plugin-log@2.10.0` | crates.io / `tauri-apps/plugins-workspace` | 181,780/wk; publicada 2026-09-26 | Crate compilado | OK | Approved |

### npm

| Package@version | Official registry / repository | Recency and maintenance | `postinstall` | Verdict | Rationale / disposition |
|---|---|---|---|---|---|
| `react@19.3.0` | npm / `react/react` | 203,157,609/wk; 2026-09-09 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `react-dom@19.3.0` | npm / `react/react` | 191,662,480/wk; 2026-09-09 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@tauri-apps/api@2.12.0` | npm / `tauri-apps/tauri` | 2,886,550/wk; 2026-09-26 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@tauri-apps/cli@2.12.0` | npm / `tauri-apps/tauri` | 2,569,236/wk; 2026-09-26 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `zod@4.6.5` | npm / `colinhacks/zod` | 341,523,466/wk; 2026-09-13 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `typescript@7.0.2` | npm / `microsoft/TypeScript` | 330,811,370/wk; 2026-07-08 | none | OK | Approved; usar CLI, no API programática de TS 7. |
| `vite@8.3.1` | npm / `vitejs/vite` | 208,347,325/wk; 2026-09-24 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@vitejs/plugin-react@6.1.1` | npm / `vitejs/vite-plugin-react` | 106,041,647/wk; 2026-08-28 | none | OK | Approved |
| `vitest@5.0.2` | npm / `vitest-dev/vitest` | 122,333,681/wk; 2026-09-25 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `jsdom@30.1.1` | npm / `jsdom/jsdom` | 116,551,648/wk; 2026-09-22 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@testing-library/react@16.3.3` | npm / `testing-library/react-testing-library` | 68,918,804/wk; 2026-08-27 | none | OK | Approved |
| `@testing-library/user-event@14.6.7` | npm / `testing-library/user-event` | 61,433,545/wk; 2026-09-02 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@testing-library/jest-dom@7.0.1` | npm / `testing-library/jest-dom` | 74,520,416/wk; 2026-08-09 | none | OK | Approved |
| `@testing-library/dom@10.4.2` | npm / `testing-library/dom-testing-library` | 84,726,538/wk; 2026-09-13 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@types/react@19.3.0` | npm / `DefinitelyTyped/DefinitelyTyped` | 188,748,985/wk; 2026-09-09 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `@types/react-dom@19.3.0` | npm / `DefinitelyTyped/DefinitelyTyped` | 161,332,389/wk; 2026-09-09 | none | SUS | Muy reciente; riesgo aprobado por el usuario para la ejecución one-shot. |
| `pnpm@12.6.0` | npm / `pnpm/pnpm` | 216,468,628/wk; 2026-09-22 | `node install.js` | SUS / APPROVED EXCEPTION | El usuario aprobó explícitamente el riesgo del postinstall para esta ejecución one-shot; activar mediante Corepack con versión exacta. |

**Packages removed due to SLOP verdict:** none.  
**Packages flagged SUS:** todos los indicados en la tabla npm fueron marcados únicamente por `too-new`; nombre, registry y repositorio oficial están verificados. La aprobación explícita del conjunto exacto para la ejecución one-shot es final y no requiere otra elección. `[VERIFIED: package-legitimacy seam + official docs; APPROVED: user one-shot authorization]`  
**Postinstall:** ninguna dependencia auditada declara `postinstall` salvo `pnpm@12.6.0`. Los internals de `install.js` siguen sin verificación independiente, pero el usuario aceptó explícitamente ese riesgo para activar exactamente 12.6.0 mediante Corepack en esta ejecución. `[VERIFIED: npm registry metadata; APPROVED EXCEPTION: user one-shot authorization]`

## Architecture and Implementation Patterns

### Flujo recomendado

```text
Windows HID receiver
  -> worker Rust único
  -> send/get feature report
  -> parser + validación
  -> snapshot autoritativo
     -> tray/tooltip/menu Tauri
     -> comando inicial + evento de cambio
     -> panel React validado con Zod
```

1. **Protocolo exacto.** La fuente local define `VID = 0x373e`, `PID = 0x0047`; candidatos con `usagePage >= 0xff00 || interface === 2`; request de 65 bytes con `payload[3] = 0x02`, `payload[4] = 0x02`, `payload[6] = 0x83`; espera observada de 120 ms y `getFeatureReport(0, 65)`. Acepta los layouts exactos `bytes[1] === 0xa1 && bytes[4] === 0x02 && bytes[6] === 0x83` o `bytes[0] === 0xa1 && bytes[3] === 0x02 && bytes[5] === 0x83`, y solo porcentajes `<= 100`. `[VERIFIED: .planning/spikes/001-current-battery-telemetry/probe/read-battery.js:3-4,11-16,21-23,35-55]`

2. **Un dueño de HID.** El worker mantiene como máximo un handle, serializa todas las operaciones y publica snapshots; ningún componente React crea timers ni invoca lecturas directas. `[VERIFIED: AGENTS.md:163,191-192]`

3. **Estado resiliente.** Conservar `lastValidReading` y `readAtUtc`; un error nunca produce 0%. La ausencia de la interfaz después de enumerar puede presentarse como desconectado. Si el receptor está presente pero open/query falla, usar inicialmente “dormido o temporalmente no disponible”: el protocolo observado no demuestra una señal exclusiva de sueño. `[VERIFIED: .planning/REQUIREMENTS.md:10-12; ASSUMED: clasificación del fallo presente]`

4. **Recuperación.** Al fallar, descartar el handle, reenumerar y reabrir con backoff no superpuesto; después de una pausa larga del scheduler (suspensión) forzar la misma ruta. Cadencia inicial recomendada: lectura normal cada 5 min y recuperación 15 s → 1 min → 5 min → 15 min con jitter. Es un punto de partida `[ASSUMED]` y debe ajustarse tras medir el hardware y la latencia.

5. **Tauri.** Crear el tray en Rust mediante `TrayIconBuilder`, menú nativo mostrado al clic izquierdo y `Abrir panel`; registrar `tauri-plugin-single-instance` antes de los demás y, en su callback, mostrar/enfocar la ventana existente. `[CITED: https://docs.rs/tauri/latest/tauri/tray/struct.TrayIconBuilder.html; https://v2.tauri.app/plugin/single-instance/]`

6. **Estimación provisional.** El fabricante anuncia hasta 200 h, sin condiciones de configuración publicadas; calcular `reported_percent * 200 / 100`, redondear a horas y rotular “provisional” o “hasta”. No guardar ni presentar esa cifra como calibrada. `[CITED: https://attackshark.com/products/attack-shark-r5ultra-carbon-fibre-wireless-8k-paw3950max-gaming-mouse]`

## Don't Hand-Roll

| Problem | Use instead | Why |
|---|---|---|
| Win32 HID FFI | `hidapi` `windows-native` | Expone feature reports sobre `hid.dll`. `[CITED: https://docs.rs/hidapi/2.6.7/hidapi/]` |
| Mutex/puerto de instancia | `tauri-plugin-single-instance` | Integra la segunda apertura con la ventana existente. `[CITED: https://v2.tauri.app/plugin/single-instance/]` |
| Tray o menú HTML oculto | Tray/menu nativo Tauri | Cumple el clic izquierdo y funciona sin panel visible. `[CITED: https://v2.tauri.app/learn/system-tray/]` |
| Poller por componente | Worker Rust único | Evita consultas concurrentes y contradicciones. `[VERIFIED: AGENTS.md:163,191-192]` |

## Common Pitfalls

- **Desplazar mal el report ID:** portar ambos layouts y fixtures exactos; `hidapi` usa el primer byte como report ID. `[CITED: https://docs.rs/hidapi/latest/hidapi/struct.HidDevice.html]`
- **Confundir fallo con 0%:** el último porcentaje válido permanece visible con antigüedad/estado. `[VERIFIED: .planning/REQUIREMENTS.md:10-12]`
- **Reenumerar o abrir en cada tick:** mantener un handle y reabrir solo tras fallo/reenumeración. `[ASSUMED]`
- **Bloquear UI:** `send_feature_report`/`get_feature_report` son síncronos; ejecutarlos solo en el worker. No hay timeout público de feature report en la API documentada; comprobar pronto que ningún fallo deja el worker colgado. `[CITED: https://docs.rs/hidapi/latest/hidapi/struct.HidDevice.html]`
- **Confiar en `WindowEvent::Suspended/Resumed`:** tao documenta esos eventos como no soportados en Windows; usar hueco de reloj/reintento, y agregar integración Win32 solo si la aceptación demuestra que hace falta. `[CITED: https://docs.rs/tao/latest/tao/event/enum.WindowEvent.html]`
- **Ocultar que 200 h es marketing:** tooltip y panel deben incluir marcador provisional. `[CITED: página oficial Attack Shark anterior; VERIFIED: .planning/REQUIREMENTS.md:23]`

## Test and Acceptance Strategy

| Scope | Verification |
|---|---|
| Parser | `cargo test` con fixtures para ambos layouts, truncado, marcadores incorrectos, 0 y 100, y >100 rechazado. `[VERIFIED: local parser source cited above]` |
| State machine | Unit tests: inicio, fresco, fallo con última lectura, desconectado, recuperación y carga; nunca convertir error en 0%. `[VERIFIED: .planning/REQUIREMENTS.md:10-12]` |
| UI | Vitest/jsdom/Testing Library para fresco, cargando, antiguo, dormido/no disponible y desconectado; el estado no depende solo del color. `[VERIFIED: 01-CONTEXT.md:23-26; AGENTS.md:123]` |
| IPC | Testear schema Zod contra snapshots válidos e incompatibles; mocks de Tauri prueban frontend, no Rust real. `[CITED: https://v2.tauri.app/develop/tests/mocking/]` |
| Hardware | Comparar probe `node-hid` y build Rust instalado; repetir con mouse dormido/despierto, receptor desconectado/reconectado, suspensión/reanudación de Windows y software oficial abierto/cerrado. `[VERIFIED: AGENTS.md:126,173-175]` |
| WIN-02 | Perfil de CPU, memoria, wakeups, duración de consulta y ausencia de consultas superpuestas en idle, panel abierto y juego; prueba A/B de respuesta del mouse. El umbral numérico debe acordarse tras baseline. `[VERIFIED: AGENTS.md:182; ASSUMED: umbral todavía no definido]` |
| Single instance/tray | Segunda apertura enfoca la existente; clic izquierdo muestra menú; `Abrir panel` funciona; cerrar panel no detiene el worker. `[CITED: https://v2.tauri.app/plugin/single-instance/; https://v2.tauri.app/learn/system-tray/]` |

## Environment Availability

| Dependency | Required By | Available | Version / evidence | Planner action |
|---|---|---:|---|---|
| Node.js | frontend | ✓ | 24.15.0 | Satisface los engines consultados; el pin de proyecto sigue siendo 24.21.0. `[VERIFIED: local CLI + npm metadata]` |
| npm | registry/tooling | ✓ | 12.0.2 | Solo verificación; usar pnpm aprobado. `[VERIFIED: local CLI]` |
| pnpm | frontend | ✓, wrong pin | 11.19.0; proyecto pide 12.6.0 | Checkpoint de legitimidad y luego activar 12.6.0 vía Corepack. `[VERIFIED: local CLI; AGENTS.md:57]` |
| Corepack | pnpm pin | ✓ | 0.34.6 | Activar exactamente pnpm 12.6.0 bajo la aprobación one-shot ya registrada. `[VERIFIED: local CLI; APPROVED: user authorization]` |
| Rust/rustup/cargo | backend | ✗ | no encontrado | **Prerequisito bloqueante:** instalar Rust 1.98.1 MSVC. `[VERIFIED: local CLI; AGENTS.md:51,132-135]` |
| Visual Studio C++ Build Tools + Windows SDK | MSVC/Tauri | ✗ no detectado | `cl`/`vswhere` no encontrados | **Prerequisito bloqueante:** instalar workload Desktop development with C++ y SDK. `[VERIFIED: local CLI; AGENTS.md:132]` |
| WebView2 Evergreen | UI | ✓ | 153.0.4234.48 | Sin acción. `[VERIFIED: registro local]` |

## Project Constraints (from AGENTS.md)

- Windows 11 x64 primario, Windows 10 x64 secundario, target `x86_64-pc-windows-msvc`; R5 Ultra por receptor 2.4 GHz. `[VERIFIED: AGENTS.md:15,39-43]`
- Sin telemetría remota; frontend sin acceso genérico a HID, filesystem o SQL. `[VERIFIED: AGENTS.md:16,29-33]`
- Comunicar incertidumbre y calibración; no presentar provisional como exacto. `[VERIFIED: AGENTS.md:17]`
- Bajo consumo y cero interferencia perceptible con el mouse. `[VERIFIED: AGENTS.md:18]`
- No servicio Windows, driver, elevación, sidecar, servidor HTTP, nube, updater, multiproceso de sondeo ni soporte multi-modelo en v1. `[VERIFIED: AGENTS.md:184-193]`

## Security Domain

| ASVS category | Applies | Control |
|---|---:|---|
| V2 Authentication | no | Aplicación local sin cuentas. `[VERIFIED: .planning/REQUIREMENTS.md:51-60]` |
| V3 Session Management | no | No existe sesión remota. `[VERIFIED: .planning/REQUIREMENTS.md:51-60]` |
| V4 Access Control | yes | IPC mínimo, sin comandos genéricos HID/FS/shell. `[CITED: https://v2.tauri.app/security/capabilities/]` |
| V5 Input Validation | yes | Validar tamaño, marcadores, rango 0–100 y DTO con Zod. `[VERIFIED: local parser source; CITED: https://zod.dev/packages/zod]` |
| V6 Cryptography | no | La fase no almacena secretos ni transmite datos. `[VERIFIED: .planning/REQUIREMENTS.md:51-60]` |

Mitigaciones: CSP solo local, sin contenido/CDN remoto; lockfiles exactos y conjunto SUS aprobado una sola vez por el usuario; no registrar buffers HID completos ni paths salvo diagnóstico explícito; backoff/coalescing contra agotamiento de recursos. `[CITED: https://v2.tauri.app/security/csp/; VERIFIED: AGENTS.md:67; APPROVED: user one-shot authorization]`

## Assumptions Log

| # | Claim | Risk if Wrong |
|---|---|---|
| A1 | Sondeo normal 5 min y backoff 15 s → 1 min → 5 min → 15 min. | Latencia de recuperación o carga insuficiente; ajustar después del baseline. |
| A2 | “Dormido o temporalmente no disponible” es la etiqueta honesta cuando el receptor existe pero la consulta falla. | Hardware podría ofrecer una señal discriminante aún no descubierta. |
| A3 | Detectar hueco del scheduler y reintentar basta tras resume sin Win32 adicional. | Si falla aceptación, agregar recepción de eventos de energía/dispositivo como tarea explícita. |
| A4 | El techo de 200 h es útil como semilla lineal. | Condiciones del fabricante no publicadas; mantener siempre etiqueta provisional. |
| A5 | Los límites cuantitativos de recursos se fijarán desde un baseline. | Sin umbral acordado WIN-02 solo puede cerrarse con evidencia comparativa. |

## Open Questions (RESOLVED)

1. **RESOLVED — `get_feature_report`:** cada consulta queda detrás de un worker supervisado con presupuesto de respuesta de 2 s; timeout envenena la generación, exige reset del handle y bloquea publicación tardía. Un smoke real Rust/probe, ejecutado antes de la UI, es vinculante: FAIL bloquea y UNVERIFIED permanece como checkpoint humano.
2. **RESOLVED — convivencia con software oficial:** el gate temprano debe ejecutar y registrar la matriz con `ATTACK SHARK GAMING.exe` cerrado y abierto. Solo PASS permite continuar; FAIL y UNVERIFIED no se infieren ni se autoaprueban.
3. **RESOLVED — WIN-02:** CPU ≤0.5%, working set oculto ≤120 MiB y p95 HID ≤500 ms son presupuestos provisionales para recopilar evidencia. Requieren valores medidos y una comparación humana A/B durante juego; estar bajo los umbrales nunca produce PASS automático.
4. **RESOLVED — dependencias/pnpm:** el usuario aprobó explícitamente en modalidad one-shot el conjunto exacto auditado, incluidos paquetes SUS por recencia y el postinstall de `pnpm@12.6.0`. Esa aprobación es final; se conservan pins, lockfiles y notas de auditoría.

## Sources

### Primary
- `.planning/spikes/001-current-battery-telemetry/probe/read-battery.js` — protocolo reproducible observado.
- `AGENTS.md`, `.planning/REQUIREMENTS.md`, `01-CONTEXT.md` — alcance y restricciones.
- crates.io y npm registry — identidad, versión, repositorio, fecha, descargas y scripts.
- [Tauri tray](https://v2.tauri.app/learn/system-tray/), [single instance](https://v2.tauri.app/plugin/single-instance/), [security](https://v2.tauri.app/security/), [testing mocks](https://v2.tauri.app/develop/tests/mocking/).
- [hidapi 2.6.7](https://docs.rs/hidapi/2.6.7/hidapi/), [React versions](https://react.dev/versions), [Vite guide](https://vite.dev/guide/), [Zod](https://zod.dev/packages/zod).
- [Attack Shark R5 Ultra](https://attackshark.com/products/attack-shark-r5ultra-carbon-fibre-wireless-8k-paw3950max-gaming-mouse) — autonomía comercial declarada.

## Metadata

**Confidence breakdown:** Stack HIGH; protocolo HIGH; arquitectura HIGH; recuperación MEDIUM hasta aceptación de hardware; estimación provisional MEDIUM por condiciones no publicadas del dato comercial.  
**Research date:** 2026-09-28  
**Valid until:** 2026-10-05 para paquetes; protocolo válido mientras el hardware/firmware no cambie.
