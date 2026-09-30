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

<!-- GSD:stack-start source:current V2 implementation -->

## Technology Stack

Only the Rust/Win32 V2 application is maintained. The former Tauri/React and C# applications were removed at the user's request on 2026-09-30. Historical `.planning/research/STACK.md` is an earlier proposal, not the current architecture.

- Windows 10/11 x64, Rust stable pinned by `rust-toolchain.toml`, target `x86_64-pc-windows-msvc`.
- Native Win32 window and notification area; GDI/GDI+ rendering via `windows` crate. No browser, WebView, HTTP server, .NET host or JavaScript runtime.
- `hidapi` windows-native for HID, `serde`/`serde_json` and `chrono` for local battery history and time, `thiserror` for errors.
- `winres` build dependency embeds the full shark icon from `src-tauri/icons/icon.ico` into the EXE.
- Brand art: `assets/shark-battery.png`; portable layout uses `Assets/shark-battery.png` beside the EXE.
- Rust crate directory remains `src-tauri` to preserve existing paths; its name does not imply a Tauri dependency.
- Application: `r5-battery-estimator-v2`. Diagnostic CLI: `r5-battery-probe`.
- Build: `cargo build --manifest-path src-tauri/Cargo.toml --release --target x86_64-pc-windows-msvc --bin r5-battery-estimator-v2`.
- Tests: `cargo test --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`. Ignored hardware tests need the receiver.
- Packaging: `scripts/package-v2.ps1` stages the portable; Inno Setup 6 builds `installer/R5BatteryEstimatorV2.iss` for per-user installation.
- Prerequisites: MSVC C++ Build Tools and Windows SDK. Exact crate versions are in `src-tauri/Cargo.toml` and `Cargo.lock`.

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
