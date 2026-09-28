# Technology Stack

**Project:** R5 Battery Estimator  
**Domain:** Aplicación local de bandeja para Windows con telemetría HID, estimación aprendida e historial  
**Researched:** 2026-09-28  
**Overall confidence:** MEDIUM — las versiones y APIs se verificaron en documentación oficial y registros primarios; la convivencia HID con el software del fabricante y el consumo residente todavía requieren validación sobre el equipo real.

## Recommendation

Construir la aplicación con **Tauri 2.12.0**, un núcleo **Rust 1.98.1 estable** y una ventana **React 19.3.0 + TypeScript 7.0.2 + Vite 8.3.1**. El proceso Rust debe ser dueño exclusivo de HID, SQLite, sondeo, cálculo, bandeja, autoinicio y notificaciones. La WebView solo renderiza DTOs tipados y solicita acciones estrechas mediante comandos Tauri.

Esta combinación es la mejor para el caso concreto:

- mantiene una UI detallada y flexible sin distribuir otra copia de Chromium;
- ofrece bandeja, ciclo de vida, autoinicio, instancia única, notificaciones e instalador desde el ecosistema oficial de Tauri;
- permite portar de forma directa el intercambio de feature reports ya validado con `node-hid` a `hidapi`, que usa `hid.dll` en Windows;
- mantiene el sondeo y la base de datos fuera de la WebView y activos aunque la ventana esté oculta;
- evita que el frontend tenga acceso genérico al dispositivo, al sistema de archivos o a SQL.

`node-hid` **no debe volver a investigar si existe telemetría**. El spike existente es la especificación ejecutable: VID `0x373E`, PID `0x0047`, interfaz HID 2, request de 65 bytes incluido el report ID y respuesta capturada `00 A1 00 02 02 00 83 00 5A`. La implementación Rust debe conservar fixtures byte por byte y comparar sus primeras lecturas reales con el probe antes de retirar ese probe del flujo de desarrollo.

## Supported Platform

| Item | Decision | Rationale |
|------|----------|-----------|
| Runtime OS | Windows 11 x64 como objetivo primario; Windows 10 x64 como compatibilidad secundaria si el propietario aún lo necesita | El proyecto es Windows-only y el receptor validado está en x64. El target MSVC actual requiere Windows 10 o superior. |
| Rust target | `x86_64-pc-windows-msvc` | Es Tier 1 con host tools y coincide con Tauri/Visual Studio Build Tools. No agregar x86 ni ARM64 sin hardware de prueba. |
| Web renderer | Microsoft Edge WebView2 Evergreen del sistema | Tauri reutiliza el WebView del sistema y recibe sus parches; no fijar ni incluir un runtime de ~180 MB. |
| Installer | NSIS `-setup.exe`, instalación por usuario | No necesita elevación y Tauri instala por defecto bajo `%LOCALAPPDATA%`. MSI/WiX queda para distribución empresarial futura. |
| Data location | `app.path().app_local_data_dir()` | La calibración es específica de este mouse/equipo y debe vivir bajo `FOLDERID_LocalAppData`, no en Roaming ni junto al ejecutable. |

## Recommended Stack

### Core Framework and Toolchain

| Technology | Exact version | Purpose | Why |
|------------|---------------|---------|-----|
| Rust stable | `1.98.1` | Núcleo residente, HID, persistencia y estimador | Release estable vigente; fijarlo en `rust-toolchain.toml` hace reproducibles los builds. No usar nightly. |
| Tauri | `2.12.0` | Shell de escritorio, ventana, IPC, tray y empaquetado | Release estable actual. Usa WebView2 del sistema y tiene APIs oficiales de bandeja y plugins de escritorio. |
| `tauri-build` | `2.7.0` | Build script de Tauri | Versión estable publicada y compatible con la línea Tauri 2. |
| `@tauri-apps/cli` | `2.12.0` | Desarrollo, build y bundles | Alinear CLI y API JS con el core 2.12.0. |
| `@tauri-apps/api` | `2.12.0` | `invoke`, eventos y ventana desde React | Es la única API Tauri que el frontend necesita en v1. |
| Node.js LTS | `24.21.0` | Toolchain frontend solamente | Node 24 es LTS hasta abril de 2028. No se distribuye dentro de la aplicación. |
| pnpm | `12.6.0` | Paquetes y lockfile frontend | Release actual, compatible con Node 24 y con binario nativo para Windows x64. |

**Version policy:** fijar estas versiones, guardar `Cargo.lock` y `pnpm-lock.yaml`, y actualizar en PRs separados con prueba de hardware y smoke test del instalador. No adoptar Tauri 3 alpha ni dependencias `next`, `beta` o `rc`.

### Desktop Lifecycle and Windows Integration

| Library | Exact version | Purpose | When to Use |
|---------|---------------|---------|-------------|
| Tauri `tray-icon` feature | core `2.12.0` | Icono, tooltip, menú y apertura de la ventana | Siempre. Crear el tray desde Rust para que exista aunque la WebView esté cerrada. |
| `tauri-plugin-single-instance` | `2.5.0` | Evitar dos pollers y dos escritores SQLite | Siempre; Tauri exige registrarlo como **primer plugin**. La segunda apertura enfoca la ventana existente. |
| `tauri-plugin-autostart` | `2.6.0` | Iniciar con la sesión de Windows | Siempre; habilitar tras la instalación/primer inicio y exponer un toggle verificable en Ajustes. No escribir manualmente el registro. |
| `tauri-plugin-notification` | `2.5.0` | Aviso nativo al cruzar el umbral | Siempre en el bundle instalado. En Windows solo funciona correctamente para aplicaciones instaladas, por lo que la aceptación debe probar el NSIS, no solo `tauri dev`. |
| `tauri-plugin-log` | `2.10.0` | Logs rotados en LocalAppData | Siempre; nivel `info` en release, sin payloads HID completos salvo diagnóstico explícito. |
| `tauri-plugin-updater` | `2.13.0` | Actualización firmada | Diferir hasta que exista distribución recurrente. Su firma es obligatoria y es distinta de Authenticode. |
| `tauri-plugin-process` | `2.4.0` | Reinicio después de una actualización | Instalar solo junto con updater. No es necesario para el MVP local. |

El backend debe interceptar `CloseRequested` y ocultar la ventana; **Salir** desde el menú de bandeja es la única acción que termina el proceso. El autoinicio debe abrir sin mostrar la ventana principal. La instancia única impide que el acceso directo, el autoinicio y un doble clic creen trabajadores duplicados.

### HID Access

| Technology | Exact version | Purpose | Why |
|------------|---------------|---------|-----|
| `hidapi` crate | `2.6.7` con `default-features = false`, `features = ["windows-native"]` | Enumerar y abrir la interfaz vendor, enviar y leer feature reports | La opción nativa llama `hid.dll` directamente, evita compilar la biblioteca C hidapi y expone `send_feature_report` / `get_feature_report`. |

Contrato de implementación:

1. Crear un adaptador `R5HidTransport` detrás de un trait pequeño (`query_battery`, luego `query_consumption_settings`).
2. Identificar por VID/PID, producto e `interface_number == 2`; no guardar paths HID entre reinicios porque cambian al reconectar.
3. Reproducir exactamente el request del probe. `hidapi` exige que el primer byte sea el report ID; los 64 bytes siguientes son el payload validado.
4. Validar marcadores y longitud antes de aceptar porcentaje/carga. Una respuesta ausente o malformada es `Unavailable`, nunca `0%`.
5. Ejecutar las llamadas HID bloqueantes fuera del hilo UI, idealmente en un único worker Rust dueño del handle. Reenumerar y reabrir ante suspensión, reconexión o error.
6. Mantener sondeo moderado y con backoff; medir que el polling no altera la latencia del mouse. Probar con `ATTACK SHARK GAMING.exe` abierto y cerrado porque el acceso concurrente todavía no está validado.

El spike `node-hid` se conserva como herramienta diagnóstica y oráculo de protocolo. No se incluye Node, `node-hid`, Python ni un sidecar en el instalador final.

### Local Persistence

| Technology | Exact version | Purpose | Why |
|------------|---------------|---------|-----|
| SQLite bundled | `3.53.2` vía `libsqlite3-sys` | Base local | Archivo portable y transaccional. Esta versión incluye la corrección del bug WAL-reset previo a 3.51.3. |
| `rusqlite` | `0.40.2`, feature `bundled` | Acceso síncrono tipado a SQLite | Evita depender de una DLL SQLite instalada y es suficiente para una app de un solo usuario. |
| `rusqlite_migration` | `2.6.0` | Migraciones embebidas y testeables | Declara compatibilidad con `rusqlite ^0.40`; evita SQL de migración disperso. |

Guardar un único `r5-battery.sqlite3` en `app_local_data_dir()`. Usar un worker de persistencia con una conexión escritora, transacciones explícitas, `PRAGMA foreign_keys=ON`, `PRAGMA journal_mode=WAL`, `PRAGMA synchronous=NORMAL` y `busy_timeout`. Agrupar escrituras y conservar eventos/cambios relevantes, no una fila cada segundo.

Schema mínimo recomendado:

- `profiles`: identidad exacta de polling rate, Competitive Mode, Motion Sync y sleep timeout;
- `battery_observations`: timestamp UTC, porcentaje informado, charging, profile id, calidad/estado de lectura;
- `discharge_segments`: tramos aceptados que alimentan el aprendizaje;
- `profile_models`: parámetros, evidencia y nivel de calibración por perfil;
- `events`: carga, desconexión, cambio de perfil, aviso y recuperación;
- `settings`: umbral, notificaciones, autoinicio y versión de preferencias.

No dar SQL al frontend y no usar `tauri-plugin-sql`: el backend debe devolver consultas de dominio (`dashboard_snapshot`, `history_range`, `review_profile`). Esto mantiene invariantes, migraciones y sanitización en un solo límite.

### Frontend

| Technology | Exact version | Purpose | Why |
|------------|---------------|---------|-----|
| React / React DOM | `19.3.0` | Ventana detallada | Ecosistema maduro para paneles, estados vacíos, accesibilidad y gráficos. |
| TypeScript | `7.0.2` | Contratos y chequeo estático | Release estable actual y compilador nativo. Ejecutar `tsc --noEmit` directamente. |
| Vite | `8.3.1` | Dev server y bundle estático | Línea estable mantenida; compatible con Node 24 y adecuada para Tauri. |
| `@vitejs/plugin-react` | `6.1.1` | JSX/Fast Refresh | Plugin oficial de la pila Vite/React. |
| TanStack Query | `5.104.0` | Estado asíncrono de comandos y snapshots | Modela carga/error/cache de consultas Tauri. Actualizar su cache con eventos del backend; no usarlo como base de datos. |
| Recharts | `3.10.1` | Historial de batería y consumo | API React/SVG suficiente para series pequeñas. Montar gráficos solo con la ventana visible. |
| Zod | `4.6.5` | Validar DTOs que cruzan IPC | Detecta deriva Rust/TypeScript temprano; usar schemas solo en el borde. |
| date-fns | `4.4.0` | Formato local y duraciones | Modular y sin runtime global. Guardar UTC; localizar únicamente al presentar. |
| Lucide React | `1.48.0` | Iconos accesibles de la ventana | Sistema consistente; mantener texto/labels y no comunicar estado solo por icono. |

Usar CSS Modules o CSS plano con variables de diseño; no agregar Tailwind, Material UI ni un kit de dashboard en v1. No cargar fuentes, scripts, iconos o telemetría desde CDN. La app debe funcionar completamente offline con CSP estricta y assets incluidos en el bundle.

**Nota TypeScript 7:** es la nueva implementación nativa y `7.0` todavía no expone la API programática histórica; el propio equipo recomienda TypeScript 6 en paralelo para herramientas que aún dependan de esa API. Esta pila no la necesita: Vite transpila, `tsc --noEmit` chequea y Vitest prueba. No incorporar `typescript-eslint`, generadores que importen `typescript`, ni plugins opacos hasta que declaren soporte explícito. Si aparece incompatibilidad concreta, fijar temporalmente `@typescript/typescript6@6.0.2` en lugar de mezclar dos compiladores sin necesidad.

### Rust Supporting Libraries

| Library | Exact version | Purpose | When to Use |
|---------|---------------|---------|-------------|
| `serde` | `1.0.229`, feature `derive` | DTOs IPC y configuración | Siempre. Mantener tipos de transporte separados de entidades DB. |
| `serde_json` | `1.0.151` | Payloads IPC y fixtures | Solo en el borde y pruebas, no como almacenamiento principal. |
| `thiserror` | `2.0.21` | Errores de dominio | Errores explícitos para `DeviceUnavailable`, `MalformedReport`, `Database`, etc. |
| `log` | `0.4.34` | Fachada de logging | Integrada con `tauri-plugin-log`; no agregar otra pila de tracing en v1. |
| `image` | `0.25.10`, solo features necesarias | Generar variantes RGBA del icono de bandeja | Si el porcentaje/estado se rasteriza dinámicamente. Deshabilitar codecs no usados. |

No añadir Tokio directamente al inicio. Un worker dedicado con `std::sync::mpsc`/timeouts mantiene HID y SQLite serializados y hace sencilla la detención ordenada. Usar `tauri::async_runtime` solo para comandos/eventos ya asíncronos; incorporar `tokio` explícito únicamente si aparece una necesidad medida.

## Testing Stack

| Layer | Tool/version | What to test |
|------|--------------|--------------|
| Rust unit/integration | `cargo test` de Rust `1.98.1` | Parser de frames, perfiles, segmentos de descarga, estimador, confianza y deduplicación de alertas. |
| Property tests | `proptest 1.11.0` | Porcentaje 0–100, tiempos no negativos, orden temporal, discontinuidades de carga y perfiles nunca mezclados. |
| DB tests | `tempfile 3.27.0` + `rusqlite_migration 2.6.0` | Base nueva, upgrades, rollback, WAL recovery y consultas con datos límite. |
| Frontend unit/component | `vitest 5.0.2`, `jsdom 30.1.1`, `@testing-library/react 16.3.3`, `@testing-library/user-event 14.6.7`, `@testing-library/jest-dom 7.0.1` | Estados fresco/antiguo/desconectado/cargando, accesibilidad, rangos y acciones de perfil. |
| Tauri mock | Mock runtime e `invoke` mock de Tauri | Contratos IPC y manejo de errores sin hardware. |
| Windows E2E | WebdriverIO `9.32.0` + `@wdio/tauri-service 1.4.0` | Abrir desde tray, enfocar segunda instancia, ocultar al cerrar, ajustes y navegación. |
| Hardware acceptance | Script/probe `node-hid` existente + build Rust instalado | Igualdad de lectura, sleep/wake, desconexión/reconexión, software oficial abierto, carga y reinicio de Windows. |

La mayor parte de la lógica debe vivir en crates/módulos puros, sin `AppHandle`, para que `cargo test` cubra el comportamiento crítico. Definir un `HidTransport` fake que reproduzca las cinco lecturas validadas y errores de timeout. El E2E no sustituye la aceptación con el receptor real.

## Packaging and Distribution

### Development prerequisites

- Visual Studio 2022 Build Tools con **Desktop development with C++** y Windows SDK.
- WebView2 Evergreen instalado (ya viene en Windows 11 y en Windows 10 moderno).
- Rust estable `1.98.1` para `x86_64-pc-windows-msvc`.
- Node `24.21.0` y pnpm `12.6.0` solo para construir el frontend.

### Production bundle

Usar `tauri build --bundles nsis` en un runner Windows. Configurar `webviewInstallMode: { "type": "embedBootstrapper" }`: agrega aproximadamente 1.8 MB y mantiene el instalador pequeño, aunque todavía necesita red solo cuando WebView2 falta. No usar `skip`; no incluir `fixedRuntime` salvo un requisito offline explícito.

Mantener el modo NSIS por usuario predeterminado. Probar instalación, actualización, desinstalación y autoinicio desde la ruta instalada. La notificación nativa de Windows no debe evaluarse desde un binario suelto de desarrollo.

Antes de distribuir fuera del equipo del propietario, firmar EXE e instalador con Authenticode/Timestamp. Tauri documenta certificados OV/EV, Azure Key Vault y Azure Artifact Signing. La firma no garantiza reputación inmediata de SmartScreen, pero una identidad estable evita reiniciarla en cada release. Si se habilita el updater, además generar y proteger la clave Tauri: la verificación de artefactos del updater no puede desactivarse y **no reemplaza** Authenticode.

### Startup behavior

1. Registrar `tauri-plugin-single-instance` primero.
2. Inicializar tray, worker HID/DB y plugins desde Rust.
3. Si el proceso arrancó por autostart, mantener la ventana oculta.
4. Activar autostart y verificar `is_enabled`; mostrar el estado real en Ajustes.
5. Al desinstalar, permitir que el instalador/plugin elimine el registro correspondiente; no crear tareas programadas ni servicios.

## Installation

Comandos iniciales recomendados después de crear el proyecto Tauri React/TypeScript:

```powershell
# Frontend runtime
pnpm add --save-exact react@19.3.0 react-dom@19.3.0 `
  @tauri-apps/api@2.12.0 @tanstack/react-query@5.104.0 `
  recharts@3.10.1 zod@4.6.5 date-fns@4.4.0 lucide-react@1.48.0

# Frontend build and tests
pnpm add -D --save-exact @tauri-apps/cli@2.12.0 typescript@7.0.2 `
  vite@8.3.1 @vitejs/plugin-react@6.1.1 vitest@5.0.2 jsdom@30.1.1 `
  @types/react@19.3.0 @types/react-dom@19.3.0 `
  @testing-library/react@16.3.3 @testing-library/user-event@14.6.7 `
  @testing-library/jest-dom@7.0.1 @wdio/cli@9.32.0 @wdio/tauri-service@1.4.0

# Run these inside src-tauri
cargo add tauri@2.12.0 --features tray-icon
cargo add tauri-build@2.7.0 --build
cargo add tauri-plugin-single-instance@2.5.0
cargo add tauri-plugin-autostart@2.6.0
cargo add tauri-plugin-notification@2.5.0
cargo add tauri-plugin-log@2.10.0
cargo add hidapi@2.6.7 --no-default-features --features windows-native
cargo add rusqlite@0.40.2 --features bundled
cargo add rusqlite_migration@2.6.0
cargo add serde@1.0.229 --features derive
cargo add serde_json@1.0.151 thiserror@2.0.21 log@0.4.34 image@0.25.10
cargo add --dev proptest@1.11.0 tempfile@3.27.0

# Only when self-update becomes a requirement
cargo add tauri-plugin-updater@2.13.0 tauri-plugin-process@2.4.0
```

Después, convertir los requisitos Cargo generados a pins exactos (`=2.12.0`, etc.) o depender del `Cargo.lock` versionado como fuente de reproducibilidad. Para esta aplicación final, versionar siempre ambos lockfiles.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Desktop shell | Tauri 2.12 | Electron + `node-hid` | Reutiliza el probe con menos porting, pero distribuye Chromium y Node y usa un modelo multiproceso tipo navegador; es una mala base para una utilidad residente cuyo requisito explícito es bajo consumo. |
| Desktop shell | Tauri 2.12 | .NET 10 WPF | Alternativa técnicamente sólida y posiblemente aún más nativa para Windows, pero exige escoger bibliotecas comunitarias separadas para tray/HID/gráficos y rehacer la UI en XAML. Elegirla solo si el equipo domina C# y no Rust/React. |
| Desktop shell | Tauri 2.12 | WinUI 3 / Windows App SDK | UI moderna, pero la bandeja y varios comportamientos Win32 requieren integración adicional y el empaquetado agrega complejidad que no mejora el valor central. |
| HID | `hidapi` `windows-native` | Windows HID por P/Invoke manual | Menos dependencia externa, pero multiplica código inseguro y detalles de handles sin aportar valor; `hidapi` ya expone la semántica correcta de report ID. |
| HID | Rust port of validated protocol | `node-hid` sidecar | Mantendría dos runtimes, IPC extra y packaging de addon nativo. Conservar el probe, no el sidecar. |
| Persistence | `rusqlite` + SQLite bundled | JSON/CSV | Sin transacciones, migraciones ni consultas robustas; propenso a pérdida/corrupción al cerrar o reiniciar. CSV queda solo para exportación futura. |
| Persistence | `rusqlite` | `tauri-plugin-sql` / SQL desde React | Expone demasiado poder al frontend y dispersa reglas de dominio. Toda persistencia debe quedar detrás de comandos Rust. |
| Persistence | SQLite | PostgreSQL/InfluxDB | Requieren servicio y administración para un único usuario y volumen mínimo; contradicen privacidad/offline. |
| Async/runtime | Worker Rust dedicado | Tokio explícito en toda la aplicación | El sondeo es lento, periódico y bloqueante; serializar HID+DB en un worker es más simple. Agregar async solo cuando una necesidad real lo justifique. |
| UI components | CSS Modules + HTML semántico | Material UI/Tailwind/dashboard kit | Aumentan dependencia y superficie visual para una sola ventana. Definir un sistema pequeño específico del producto. |
| State | TanStack Query + estado local | Redux/Zustand global | No existe suficiente estado cliente complejo; la fuente de verdad es Rust/SQLite. |
| Installer | NSIS por usuario | MSI/WiX | MSI solo es necesario para despliegue empresarial; WiX añade requisitos de build y elevación sin beneficio para el propietario inicial. |
| Autostart | Plugin oficial | `HKCU\\...\\Run`, Startup folder o Task Scheduler manual | Código Windows específico difícil de desinstalar y verificar. El plugin ya ofrece enable/disable/status. |

## Risks and Compatibility Notes

| Risk | Impact | Mitigation / roadmap flag |
|------|--------|----------------------------|
| Port Rust interpreta offsets distinto a `node-hid` | Lecturas falsas o ausencia | Primera fase: tests de fixture y comparación simultánea del probe y build Rust sobre el mismo receptor. |
| Driver oficial abre la misma interfaz | Fallos intermitentes o conflicto | Probar con software oficial abierto/cerrado; abrir solo interfaz 2, cerrar/reabrir con backoff y nunca sondear agresivamente. |
| Mouse dormido o receptor reenumerado | Handle inválido | No persistir device path; reenumerar y modelar sleep/disconnected como estados normales. |
| WebView2 ausente/dañado | La UI no abre | NSIS con `embedBootstrapper`; tray/core no deben asumir que la ventana ya existe. |
| Tauri 2.12 es muy reciente | Regresión de plugin | Pins exactos, smoke de instalador y hardware. Mantener rollback a último lockfile conocido, no saltar a Tauri 3 alpha. |
| TypeScript 7 transición de tooling | Plugins de lint/codegen incompatibles | `tsc --noEmit`; evitar APIs del compilador y dependencias no confirmadas. Fallback explícito a TypeScript 6.0.2 si aparece un bloqueo. |
| Logs/historial crecen sin límite | Disco y privacidad | Rotación de logs, retención/compactación de observaciones y eventos resumidos. |
| WAL/copias incompletas | Backup inconsistente | Cerrar conexión o usar backup API antes de exportar; nunca copiar solo el archivo principal mientras hay `-wal` activo. |
| SmartScreen en builds sin firma | Mala instalación/distribución | Firma Authenticode estable antes de release público; documentar que updater signing es otra capa. |
| Recursos residentes no medidos | Incumple requisito central | Añadir gate de profiling en idle, ventana abierta y juego; medir CPU, memoria, wakeups y duración de cada consulta HID en hardware real. |

## What Not to Build Into v1

- servicio de Windows, driver kernel o ejecución elevada;
- sidecar Node/Electron o bridge Python;
- servidor local HTTP; usar IPC Tauri tipado;
- nube, cuenta, analytics o crash upload;
- updater hasta que haya canal de distribución y custodia de claves;
- acceso directo a SQL/HID/filesystem desde la WebView;
- múltiples procesos de background o un sondeo por componente React;
- soporte multi-mouse/multi-modelo antes de validar el flujo completo R5 Ultra.

## Sources

Todas las fuentes externas se trataron como datos no confiables y se contrastaron. El seam de clasificación asigna **MEDIUM** a hallazgos `websearch --verified`; el spike local del dispositivo aporta evidencia directa adicional para el protocolo.

### Framework, lifecycle and Windows

- [Tauri 2.12 announcement](https://v2.tauri.app/blog/) — official, version checked 2026-09-28, MEDIUM.
- [Tauri system tray](https://v2.tauri.app/learn/system-tray/) — official API, updated 2026-04-20, MEDIUM.
- [Tauri autostart plugin](https://v2.tauri.app/plugin/autostart/) — official API and permissions, MEDIUM.
- [Tauri single-instance plugin](https://v2.tauri.app/plugin/single-instance/) — official; registration-order requirement, MEDIUM.
- [Tauri notifications](https://v2.tauri.app/plugin/notification/) — official; Windows installed-app limitation, MEDIUM.
- [Tauri logging](https://v2.tauri.app/plugin/logging/) — official; LocalAppData and rotation, MEDIUM.
- [Tauri updater](https://v2.tauri.app/plugin/updater/) — official; mandatory updater signatures, MEDIUM.
- [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/) — official; NSIS/MSI, current-user install and WebView2 modes, updated 2026-09-03, MEDIUM.
- [Tauri Windows code signing](https://v2.tauri.app/distribute/sign/windows/) — official; Authenticode/SmartScreen and signing options, updated 2026-09-11, MEDIUM.
- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) — official; MSVC Build Tools and WebView2, MEDIUM.
- [Tauri path resolver](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html) — official crate docs; `app_local_data_dir`, MEDIUM.
- [Rust 1.98.1 release](https://blog.rust-lang.org/releases/latest/) — official, 2026-09-03, MEDIUM.
- [Rust Windows MSVC targets](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html) — official, MEDIUM.
- [Node release schedule](https://nodejs.org/en/about/previous-releases) and [Node 24 LTS release feed](https://nodejs.org/en/blog) — official, MEDIUM.

### HID and persistence

- [hidapi 2.6.7 docs](https://docs.rs/hidapi/latest/hidapi/) — official crate docs; `windows-native`, MEDIUM.
- [HidDevice feature-report semantics](https://docs.rs/hidapi/latest/hidapi/struct.HidDevice.html) — official crate docs; report ID byte and APIs, MEDIUM.
- [rusqlite 0.40.2 docs](https://docs.rs/crate/rusqlite/latest) and [project README](https://github.com/rusqlite/rusqlite) — official project sources; bundled SQLite, MEDIUM.
- [SQLite as application file format](https://www.sqlite.org/appfileformat.html) — official SQLite documentation, MEDIUM.
- [SQLite WAL](https://www.sqlite.org/wal.html) and [3.51.3 fix](https://sqlite.org/releaselog/3_51_3.html) — official; concurrency, backup caveat and WAL-reset fix, MEDIUM.
- [Windows guidance for machine-specific local data](https://learn.microsoft.com/en-us/windows/apps/develop/windows-app-restore) — Microsoft, MEDIUM.

### Frontend and testing

- [React versions](https://react.dev/versions) — official; React 19.3.0, 2026-09-09, MEDIUM.
- [TypeScript 7.0 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/) — official; native compiler and temporary API limitation, MEDIUM.
- [Vite 8 announcement](https://vite.dev/blog/announcing-vite8) and [supported releases](https://vite.dev/releases) — official, MEDIUM.
- [TanStack Query v5 docs](https://tanstack.com/query/latest/docs/framework/react) — official, MEDIUM.
- [Vitest guide](https://vitest.dev/guide/) — official, MEDIUM.
- [Tauri testing overview](https://v2.tauri.app/develop/tests/) and [WebDriver guidance](https://v2.tauri.app/develop/tests/webdriver/) — official; mock runtime and WebdriverIO Tauri service, updated 2026-06-29, MEDIUM.
- Package versions were cross-checked on the official [npm registry](https://www.npmjs.com/) and [crates.io](https://crates.io/) on 2026-09-28.

