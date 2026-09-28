<!-- GSD:project-start source:PROJECT.md -->

## Project

**R5 Battery Estimator**

Una aplicación local para Windows que lee la batería informada por el Attack Shark R5 Ultra y la convierte en estimaciones comprensibles de autonomía. Funciona desde la bandeja del sistema y ofrece una ventana detallada con porcentaje, carga, consumo aprendido, perfiles automáticos y tiempo restante tanto para uso cotidiano como para juego continuo.

Está pensada inicialmente para el dueño de un R5 Ultra conectado por receptor 2.4 GHz. Aprende de la descarga real del mouse para reemplazar progresivamente la estimación inicial por datos personalizados.

**Core Value:** Mostrar una estimación útil y cada vez más precisa del tiempo de batería restante del R5 Ultra, basada en su telemetría real y en el patrón de consumo correspondiente a la configuración activa.

### Constraints

- **Compatibility**: Windows y Attack Shark R5 Ultra por receptor 2.4 GHz en v1 — es el hardware disponible y validado.
- **Privacy**: Datos de uso y calibración almacenados localmente — no hay necesidad de transmitir telemetría.
- **Accuracy**: La interfaz debe comunicar nivel de confianza y estado de calibración — una cifra temporal sin suficiente historial no debe parecer exacta.
- **Background operation**: Debe consumir pocos recursos y no interferir con la latencia del mouse — la lectura periódica no puede degradar el juego.
- **Profiles**: Solo polling rate, Competitive Mode, Motion Sync y tiempo de reposo definen identidad de perfil — decisión explícita del usuario.

<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Recommendation

- mantiene una UI detallada y flexible sin distribuir otra copia de Chromium;
- ofrece bandeja, ciclo de vida, autoinicio, instancia única, notificaciones e instalador desde el ecosistema oficial de Tauri;
- permite portar de forma directa el intercambio de feature reports ya validado con `node-hid` a `hidapi`, que usa `hid.dll` en Windows;
- mantiene el sondeo y la base de datos fuera de la WebView y activos aunque la ventana esté oculta;
- evita que el frontend tenga acceso genérico al dispositivo, al sistema de archivos o a SQL.

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

### HID Access

| Technology | Exact version | Purpose | Why |
|------------|---------------|---------|-----|
| `hidapi` crate | `2.6.7` con `default-features = false`, `features = ["windows-native"]` | Enumerar y abrir la interfaz vendor, enviar y leer feature reports | La opción nativa llama `hid.dll` directamente, evita compilar la biblioteca C hidapi y expone `send_feature_report` / `get_feature_report`. |

### Local Persistence

| Technology | Exact version | Purpose | Why |
|------------|---------------|---------|-----|
| SQLite bundled | `3.53.2` vía `libsqlite3-sys` | Base local | Archivo portable y transaccional. Esta versión incluye la corrección del bug WAL-reset previo a 3.51.3. |
| `rusqlite` | `0.40.2`, feature `bundled` | Acceso síncrono tipado a SQLite | Evita depender de una DLL SQLite instalada y es suficiente para una app de un solo usuario. |
| `rusqlite_migration` | `2.6.0` | Migraciones embebidas y testeables | Declara compatibilidad con `rusqlite ^0.40`; evita SQL de migración disperso. |

- `profiles`: identidad exacta de polling rate, Competitive Mode, Motion Sync y sleep timeout;
- `battery_observations`: timestamp UTC, porcentaje informado, charging, profile id, calidad/estado de lectura;
- `discharge_segments`: tramos aceptados que alimentan el aprendizaje;
- `profile_models`: parámetros, evidencia y nivel de calibración por perfil;
- `events`: carga, desconexión, cambio de perfil, aviso y recuperación;
- `settings`: umbral, notificaciones, autoinicio y versión de preferencias.

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

### Rust Supporting Libraries

| Library | Exact version | Purpose | When to Use |
|---------|---------------|---------|-------------|
| `serde` | `1.0.229`, feature `derive` | DTOs IPC y configuración | Siempre. Mantener tipos de transporte separados de entidades DB. |
| `serde_json` | `1.0.151` | Payloads IPC y fixtures | Solo en el borde y pruebas, no como almacenamiento principal. |
| `thiserror` | `2.0.21` | Errores de dominio | Errores explícitos para `DeviceUnavailable`, `MalformedReport`, `Database`, etc. |
| `log` | `0.4.34` | Fachada de logging | Integrada con `tauri-plugin-log`; no agregar otra pila de tracing en v1. |
| `image` | `0.25.10`, solo features necesarias | Generar variantes RGBA del icono de bandeja | Si el porcentaje/estado se rasteriza dinámicamente. Deshabilitar codecs no usados. |

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

## Packaging and Distribution

### Development prerequisites

- Visual Studio 2022 Build Tools con **Desktop development with C++** y Windows SDK.
- WebView2 Evergreen instalado (ya viene en Windows 11 y en Windows 10 moderno).
- Rust estable `1.98.1` para `x86_64-pc-windows-msvc`.
- Node `24.21.0` y pnpm `12.6.0` solo para construir el frontend.

### Production bundle

### Startup behavior

## Installation

# Frontend runtime

# Frontend build and tests

# Run these inside src-tauri

# Only when self-update becomes a requirement

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

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `$gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `$gsd-debug` for investigation and bug fixing
- `$gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `$gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
