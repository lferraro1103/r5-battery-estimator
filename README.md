# R5 Battery Estimator

Native Windows battery telemetry and runtime estimator for the Attack Shark R5 Ultra.

[English](#english) · [Español](#español)

---

## English

### Overview

R5 Battery Estimator is a native Windows desktop utility for the **Attack Shark R5 Ultra** connected through its 2.4 GHz receiver. It turns receiver battery telemetry into battery percentage, charging state, remaining runtime, estimated full-charge duration, and a local history chart.

The vendor software exposes mouse settings but does not provide a user-specific remaining-time estimate. This project fills that gap locally, without Electron, Chromium, Edge, WebView2, cloud services, or analytics.

### How it works

1. A Rust probe sends a HID feature-report request to the receiver and validates the returned percentage and charging state.
2. A native .NET 8 WinForms host renders the dashboard, notification-area menu, chart, and estimate.
3. Valid observations are stored locally every 30 seconds: timestamp, percentage, and charging state.
4. The estimator analyzes continuous discharge segments and ignores charging transitions, percentage increases, and gaps longer than ten minutes.
5. After collecting at least 3% of discharge across 30 minutes or more, it estimates a personalized full-charge duration.

Before that evidence exists, the application uses the manufacturer's provisional **up to 200-hour** reference and explicitly shows **Learning** instead of implying calibration.

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
| Tray lifecycle | Close/minimize hides the panel; left-click exposes Open panel, Update profile, and Exit program. |
| Icons | Separate shark application icon and color-coded battery-and-fin tray icon. |
| Safety | No missing or invalid report is displayed as 0%. |

### Privacy, scope, and limitations

No account, network request, cloud service, analytics, or telemetry upload exists. Fourteen days of observations remain only on the computer at `%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json`.

The current scope is Windows x64 and R5 Ultra 2.4 GHz receivers. Polling rate, Competitive Mode, Motion Sync, sleep settings, and usage pattern affect consumption, so the initial estimate is provisional. Unknown firmware reports fail safely instead of producing invented readings.

### Architecture and build

```text
R5 Ultra receiver → Rust / hidapi probe → JSON snapshot → .NET 8 WinForms host
                                                      ├─ tray and dashboard
                                                      ├─ local history
                                                      ├─ runtime estimator
                                                      └─ 24-hour chart
```

Requirements: Windows 10/11 x64, .NET 8 Desktop Runtime, Rust, and an R5 Ultra receiver for hardware validation.

```powershell
C:\Users\Leaan\.cargo\bin\cargo.exe build --manifest-path src-tauri\Cargo.toml --release --bin r5-battery-probe
dotnet publish R5BatteryEstimator.Native\R5BatteryEstimator.Native.csproj -c Release -r win-x64 --self-contained false -o outputs\native-winforms
.\outputs\native-winforms\R5BatteryEstimator.Native.exe
```

### Roadmap

1. Validate HID protocol behavior across firmware revisions and vendor-app states.
2. Read polling rate, Competitive Mode, Motion Sync, and sleep timeout for automatic consumption profiles.
3. Add calibration confidence intervals and quality indicators.
4. Move local history to versioned SQLite when profile-level migrations are needed.
5. Ship a signed per-user installer and optional autostart.

---

## Español

### Resumen

R5 Battery Estimator es una utilidad de escritorio nativa para Windows destinada al **Attack Shark R5 Ultra** conectado por su receptor 2.4 GHz. Convierte la telemetría de batería del receptor en porcentaje, estado de carga, autonomía restante, duración estimada de carga completa y un gráfico local de historial.

El software oficial expone ajustes del mouse, pero no proporciona una estimación de tiempo restante específica para el usuario. Este proyecto cubre esa necesidad de manera local, sin Electron, Chromium, Edge, WebView2, servicios en la nube ni analítica.

### Cómo funciona

1. Un probe Rust envía una solicitud de *feature report* HID al receptor y valida el porcentaje y estado de carga devueltos.
2. Un host nativo .NET 8 WinForms renderiza el panel, menú de bandeja, gráfico y estimación.
3. Cada 30 segundos se guardan observaciones válidas de forma local: hora, porcentaje y estado de carga.
4. El estimador analiza tramos de descarga continua e ignora transiciones de carga, aumentos de porcentaje y cortes de más de diez minutos.
5. Tras reunir al menos 3% de descarga a lo largo de 30 minutos o más, estima una duración de carga completa personalizada.

Antes de contar con esa evidencia, la aplicación usa la referencia provisional del fabricante de **hasta 200 horas** y muestra explícitamente **Aprendiendo**, en lugar de insinuar calibración.

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
| Ciclo de bandeja | Cerrar/minimizar oculta el panel; clic izquierdo ofrece Abrir panel, Actualizar perfil y Cerrar programa. |
| Iconos | Icono de aplicación de tiburón y un icono de bandeja independiente de batería con aleta y color. |
| Seguridad | Ningún reporte faltante o inválido se muestra como 0%. |

### Privacidad, alcance y límites

No existe cuenta, solicitud de red, servicio en la nube, analítica ni subida de telemetría. Catorce días de observaciones quedan sólo en el equipo, en `%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json`.

El alcance actual es Windows x64 y receptores R5 Ultra 2.4 GHz. Polling rate, Competitive Mode, Motion Sync, reposo y patrón de uso afectan el consumo; por eso la estimación inicial es provisional. Los reportes de firmware desconocidos fallan de forma segura, sin inventar lecturas.

### Arquitectura y compilación

```text
Receptor R5 Ultra → probe Rust / hidapi → snapshot JSON → host .NET 8 WinForms
                                                      ├─ bandeja y panel
                                                      ├─ historial local
                                                      ├─ estimador de autonomía
                                                      └─ gráfico de 24 horas
```

Requisitos: Windows 10/11 x64, .NET 8 Desktop Runtime, Rust y un R5 Ultra para validar el hardware.

```powershell
C:\Users\Leaan\.cargo\bin\cargo.exe build --manifest-path src-tauri\Cargo.toml --release --bin r5-battery-probe
dotnet publish R5BatteryEstimator.Native\R5BatteryEstimator.Native.csproj -c Release -r win-x64 --self-contained false -o outputs\native-winforms
.\outputs\native-winforms\R5BatteryEstimator.Native.exe
```

### Roadmap

1. Validar el protocolo HID entre revisiones de firmware y estados del software oficial.
2. Leer polling rate, Competitive Mode, Motion Sync y reposo para perfiles automáticos de consumo.
3. Agregar intervalos de confianza y calidad de calibración.
4. Migrar el historial local a SQLite versionado cuando se necesiten migraciones por perfil.
5. Publicar instalador firmado por usuario y autoinicio opcional.

## License / Licencia

This project is licensed under the [GNU General Public License v3.0](LICENSE). You may use, modify, and redistribute it under the GPL-3.0 terms; derivative distributions must preserve the same license. / Este proyecto se distribuye bajo la [GNU General Public License v3.0](LICENSE). Podés usarlo, modificarlo y redistribuirlo bajo sus términos; las distribuciones derivadas deben conservar la misma licencia.
