# R5 Battery Estimator V2

Native Windows battery telemetry and runtime estimator for the Attack Shark R5 Ultra.

[English](#english) · [Español](#español)

---

## English

### Overview

R5 Battery Estimator is a native Windows desktop utility for the **Attack Shark R5 Ultra** connected through its 2.4 GHz receiver. It turns receiver battery telemetry into battery percentage, charging state, remaining runtime, estimated full-charge duration, and a local history chart.

The vendor software exposes mouse settings but does not provide a user-specific remaining-time estimate. This project fills that gap locally, without Electron, Chromium, Edge, WebView2, cloud services, or analytics.

### How it works

1. A Rust probe sends a HID feature-report request to the receiver and validates the returned percentage and charging state.
2. A single native Rust/Win32 host renders the dashboard, notification-area menu, chart, and estimate.
3. One worker polls on 30-second deadlines and coalesces manual refresh requests. Slow queries skip missed deadlines rather than overlapping; result notifications repaint the panel immediately and show the last valid reading time.
4. Valid observations contain timestamp, percentage, and charging state. On load, out-of-range, future and older-than-14-day samples are removed, then timestamps are sorted and deduplicated. Writes flush a same-directory temporary file before Windows replaces the previous history.
5. Learning requires each individual cycle to start at ≥95% without charging and reach ≤5% without charging, over at least 30 minutes. Charging, a percentage increase, non-increasing timestamps or a gap longer than ten minutes discard the unfinished cycle. Only completed cycles contribute total elapsed time and percentage drop; the resulting full-charge duration is bounded to 5–1,000 hours.

Before a complete cycle exists, the application uses a provisional **200-hour** reference. The full-charge card shows **Aprendiendo**; remaining hours and the tooltip say **Estimación inicial · confianza baja**. With complete cycles they say **Aprendida · confianza media**. These are qualitative evidence labels, not statistical confidence intervals; a valid HID reading does not prove the runtime estimate is accurate.

```text
full-charge hours = 100 / (percentage drop / elapsed hours)
remaining hours   = current percentage × full-charge hours / 100
```

### Features

| Area | Implementation |
| --- | --- |
| Telemetry | Rust HID feature-report probe using `hidapi` over Windows `hid.dll`. |
| Dashboard | Percentage, charge state, color status circle, remaining hours, and full-charge duration. |
| History | Local 24-hour time-versus-percentage chart. |
| Tray lifecycle | Close/minimize hides the panel; either tray click exposes Open panel, Profiles: pending (disabled), Refresh now, optional Start with Windows, and Exit program. |
| Icons | Full-shark application icon and a color-coded shark-face tray icon (green/yellow/red). |
| Safety | No missing or invalid report is displayed as 0%. |

### Privacy, scope, and limitations

No account, network request, cloud service, analytics, or telemetry upload exists. Fourteen days of observations remain only on the computer at `%LOCALAPPDATA%\R5 Battery Estimator\rust-v2-history.json`. The older `battery-history.json` is read only for migration if the active file cannot be loaded; V2 does not update that legacy file.

The current scope is Windows x64 and R5 Ultra 2.4 GHz receivers. Polling rate, Competitive Mode, Motion Sync, sleep settings, and usage pattern affect consumption, so the initial estimate is provisional. Unknown firmware reports fail safely instead of producing invented readings.

Automatic profiles are pending: the validated protocol reads battery only, not polling rate, Competitive Mode, Motion Sync, or sleep timeout. The app currently learns one combined model and does not separate history when settings change. The disabled “Perfiles: pendiente” menu item reflects this limitation; no unvalidated configuration commands are sent.

### Architecture and build

```text
R5 Ultra receiver → Rust / hidapi probe → native Rust / Win32 desktop shell
                                               ├─ tray and dashboard
                                               ├─ local history
                                               ├─ runtime estimator
                                               └─ 24-hour chart
```

The portable release requires only Windows 10/11 x64 and the R5 Ultra receiver. It is a graphical Windows executable, so it does not open a console window. Rust is needed only to build from source.

```powershell
cargo build --manifest-path src-tauri\Cargo.toml --release --target x86_64-pc-windows-msvc --bin r5-battery-estimator-v2
.\src-tauri\target\x86_64-pc-windows-msvc\release\r5-battery-estimator-v2.exe
```

Only the native V2 application is maintained in this repository. The former Tauri/React and C# applications have been removed from the current tree; older versions remain available in Git history. `src-tauri` is retained as the Rust directory name for path compatibility, but no Tauri dependency or WebView remains. The `r5-battery-probe` executable is a diagnostic tool, not a second application.

Build prerequisites: Rust as pinned in `rust-toolchain.toml`, Visual Studio C++ Build Tools and the Windows SDK. Brand artwork lives in `assets/shark-battery.png`. When distributing the executable, copy that file to `Assets/shark-battery.png` beside it. To stage a portable build and generate the Inno Setup installer (if Inno Setup 6 is installed), run `powershell -File scripts/package-v2.ps1` from the repository root. The staged application is `outputs/R5BatteryEstimatorV2/R5BatteryEstimatorV2.exe`.

The dashboard scales its original native layout uniformly to the current monitor's DPI and work area, with matching button coordinates. Maximize remains disabled. The ring, blurred glow and full-shark PNG are retained; the executable embeds a shark icon as a fallback if the adjacent PNG is absent.

Optional **Iniciar con Windows** in the tray menu stores a quoted executable path in the current user's `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry named `R5BatteryEstimator`. It needs no administrator privileges. Disable it from the same menu before moving a portable copy, or toggle it off and on from the new location to update the path.

### Roadmap

1. Validate HID protocol behavior across firmware revisions and vendor-app states.
2. Read polling rate, Competitive Mode, Motion Sync, and sleep timeout for automatic consumption profiles.
3. Add calibration confidence intervals and quality indicators.
4. Move local history to versioned SQLite when profile-level migrations are needed.
5. Sign the existing per-user installer; optional per-user autostart is already available.

---

## Español

### Resumen

R5 Battery Estimator es una utilidad de escritorio nativa para Windows destinada al **Attack Shark R5 Ultra** conectado por su receptor 2.4 GHz. Convierte la telemetría de batería del receptor en porcentaje, estado de carga, autonomía restante, duración estimada de carga completa y un gráfico local de historial.

El software oficial expone ajustes del mouse, pero no proporciona una estimación de tiempo restante específica para el usuario. Este proyecto cubre esa necesidad de manera local, sin Electron, Chromium, Edge, WebView2, servicios en la nube ni analítica.

### Cómo funciona

1. Un probe Rust envía una solicitud de *feature report* HID al receptor y valida el porcentaje y estado de carga devueltos.
2. Un único host nativo Rust/Win32 renderiza el panel, menú de bandeja, gráfico y estimación.
3. Un único trabajador sondea con plazos de 30 segundos y combina las solicitudes de refresco manual. Las consultas lentas saltan plazos vencidos sin superponerse; la notificación del resultado repinta inmediatamente el panel y muestra la hora de la última lectura válida.
4. Las observaciones válidas contienen hora, porcentaje y estado de carga. Al cargar se eliminan porcentajes inválidos, fechas futuras y muestras de más de 14 días, se ordenan las fechas y se deduplican. La escritura vacía y sincroniza un temporal del mismo directorio antes de reemplazar el historial anterior mediante Windows.
5. Para aprender, cada ciclo individual debe comenzar en ≥95% sin carga y alcanzar ≤5% sin carga durante al menos 30 minutos. Carga, aumento de porcentaje, fechas no crecientes o un hueco mayor a diez minutos descartan el ciclo incompleto. Solamente los ciclos completos aportan tiempo total y caída porcentual; la duración de carga completa calculada se limita a 5–1.000 horas.

Antes de observar un ciclo completo, la aplicación usa una referencia provisional de **200 horas**. La tarjeta de carga completa muestra **Aprendiendo**; las horas restantes y el tooltip indican **Estimación inicial · confianza baja**. Con ciclos completos indican **Aprendida · confianza media**. Son etiquetas cualitativas de evidencia, no intervalos estadísticos de confianza; una lectura HID válida no demuestra que la estimación temporal sea precisa.

```text
horas de carga completa = 100 / (caída porcentual / horas transcurridas)
horas restantes         = porcentaje actual × horas de carga completa / 100
```

### Funcionalidades

| Área | Implementación |
| --- | --- |
| Telemetría | Probe Rust de feature reports HID con `hidapi` sobre `hid.dll` de Windows. |
| Panel | Porcentaje, estado de carga, círculo cromático, horas restantes y duración de carga completa. |
| Historial | Gráfico local de 24 horas: hora frente a porcentaje. |
| Ciclo de bandeja | Cerrar/minimizar oculta el panel; ambos clics de bandeja ofrecen Abrir panel, Perfiles: pendiente (deshabilitado), Actualizar ahora, Iniciar con Windows opcional y Cerrar programa. |
| Iconos | Icono de aplicación de tiburón completo y carita de tiburón en bandeja según batería (verde/amarillo/rojo). |
| Seguridad | Ningún reporte faltante o inválido se muestra como 0%. |

### Privacidad, alcance y límites

No existe cuenta, solicitud de red, servicio en la nube, analítica ni subida de telemetría. Catorce días de observaciones quedan sólo en el equipo, en `%LOCALAPPDATA%\R5 Battery Estimator\rust-v2-history.json`. El antiguo `battery-history.json` se lee únicamente para migración si no se puede cargar el archivo activo; V2 no actualiza ese archivo heredado.

El alcance actual es Windows x64 y receptores R5 Ultra 2.4 GHz. Polling rate, Competitive Mode, Motion Sync, reposo y patrón de uso afectan el consumo; por eso la estimación inicial es provisional. Los reportes de firmware desconocidos fallan de forma segura, sin inventar lecturas.

Los perfiles automáticos están pendientes: el protocolo validado lee solamente batería, no polling rate, Competitive Mode, Motion Sync ni reposo. La app aprende actualmente un único modelo combinado y no separa el historial al cambiar ajustes. El elemento deshabilitado «Perfiles: pendiente» refleja este límite; no se envían comandos de configuración sin validar.

### Arquitectura y compilación

```text
Receptor R5 Ultra → probe Rust / hidapi → shell de escritorio nativo Rust / Win32
                                               ├─ bandeja y panel
                                               ├─ historial local
                                               ├─ estimador de autonomía
                                               └─ gráfico de 24 horas
```

El release portable solo requiere Windows 10/11 x64 y el receptor R5 Ultra. Es un ejecutable gráfico de Windows, así que no abre una terminal. Rust solo es necesario para compilar desde código fuente.

```powershell
cargo build --manifest-path src-tauri\Cargo.toml --release --target x86_64-pc-windows-msvc --bin r5-battery-estimator-v2
.\src-tauri\target\x86_64-pc-windows-msvc\release\r5-battery-estimator-v2.exe
```

El repositorio mantiene únicamente la aplicación nativa V2. Las aplicaciones anteriores Tauri/React y C# se retiraron del árbol actual; sus versiones siguen recuperables desde el historial Git. `src-tauri` conserva su nombre como carpeta Rust por compatibilidad de rutas, pero ya no hay dependencias Tauri ni WebView. El ejecutable `r5-battery-probe` es una herramienta de diagnóstico, no otra aplicación.

Requisitos para compilar: Rust fijado en `rust-toolchain.toml`, Visual Studio C++ Build Tools y Windows SDK. El arte de marca está en `assets/shark-battery.png`. Para distribuir el ejecutable, copiá ese archivo como `Assets/shark-battery.png` junto a él. Para preparar el portable y generar el instalador Inno Setup (si está instalado Inno Setup 6), ejecutá `powershell -File scripts/package-v2.ps1` desde la raíz del repo. La aplicación preparada queda en `outputs/R5BatteryEstimatorV2/R5BatteryEstimatorV2.exe`.

El panel escala uniformemente el diseño nativo original según los DPI y el área de trabajo del monitor, con coordenadas de botones equivalentes. Maximizar sigue deshabilitado. Se conservan el anillo, el brillo desenfocado y el PNG del tiburón completo; el ejecutable incorpora el icono del tiburón como respaldo si falta el PNG adyacente.

La opción **Iniciar con Windows** del menú de bandeja guarda la ruta del ejecutable entre comillas en `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, en la entrada `R5BatteryEstimator` del usuario actual. No necesita permisos de administrador. Desactivala desde el mismo menú antes de mover una copia portable, o desactivala y activala desde la ubicación nueva para actualizar la ruta.

### Roadmap

1. Validar el protocolo HID entre revisiones de firmware y estados del software oficial.
2. Leer polling rate, Competitive Mode, Motion Sync y reposo para perfiles automáticos de consumo.
3. Agregar intervalos de confianza y calidad de calibración.
4. Migrar el historial local a SQLite versionado cuando se necesiten migraciones por perfil.
5. Firmar el instalador por usuario existente; el autoinicio opcional por usuario ya está disponible.

## License / Licencia

This project is licensed under the [GNU General Public License v3.0](LICENSE). You may use, modify, and redistribute it under the GPL-3.0 terms; derivative distributions must preserve the same license. / Este proyecto se distribuye bajo la [GNU General Public License v3.0](LICENSE). Podés usarlo, modificarlo y redistribuirlo bajo sus términos; las distribuciones derivadas deben conservar la misma licencia.
