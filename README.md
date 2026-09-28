# R5 Battery Estimator

Native Windows telemetry and battery-runtime estimator for the Attack Shark R5 Ultra.

**Español** · **English below**

## Español

### Contexto

R5 Battery Estimator resuelve una limitación del software oficial del Attack Shark R5 Ultra: muestra configuraciones del mouse, pero no traduce el porcentaje de batería en tiempo restante ni aprende cuánto dura realmente la carga para un usuario concreto. Esta aplicación lee la telemetría del receptor 2.4 GHz, la conserva localmente y la presenta como un panel de autonomía interpretable.

Es una aplicación Windows nativa, construida sin Electron, Chromium, Edge ni WebView2.

### Funcionamiento

1. Un probe Rust envía un *feature report* HID al receptor y valida el porcentaje y estado de carga recibidos.
2. Un host WinForms/.NET muestra el porcentaje, la carga, la autonomía restante y un gráfico de las últimas 24 horas.
3. Cada lectura válida se guarda localmente cada 30 segundos.
4. El estimador analiza tramos de descarga continua: ignora carga, saltos largos y aumentos de porcentaje.
5. Al acumular al menos 3% de descarga y 30 minutos de evidencia, calcula una duración de carga completa personalizada.

Mientras aprende, la autonomía usa como referencia provisional el valor comercial de hasta **200 horas**. La interfaz indica **Aprendiendo** para no presentar esa cifra como una calibración real.

```text
full charge hours = 100 / (percentage drop / elapsed hours)
remaining hours   = current percentage × full charge hours / 100
```

### Arquitectura

```text
R5 Ultra 2.4 GHz receiver
       │ Windows HID feature report
       ▼
Rust / hidapi / hid.dll probe
       │ JSON snapshot
       ▼
.NET 8 WinForms desktop host
  ├─ panel y bandeja de Windows
  ├─ historial local de 14 días
  ├─ modelo de descarga
  └─ gráfico hora / porcentaje
```

| Decisión | Motivo |
| --- | --- |
| WinForms/.NET 8 | Controles nativos, bandeja y bajo consumo idle. |
| Rust + hidapi | Límite seguro y pequeño para acceso HID de Windows. |
| Proceso JSON | La UI no obtiene acceso HID genérico. |
| Historial local | Privacidad y simplicidad para un único mouse/equipo. |
| Muestreo de 30 s | Tendencia útil sin sondeo agresivo ni impacto perceptible en juego. |

### Funcionalidades

- Círculo verde/amarillo/rojo/gris según el estado de batería.
- Autonomía restante en horas y duración completa aprendida.
- Gráfico local de 24 horas.
- Bandeja: minimizar/cerrar oculta la ventana; clic izquierdo muestra abrir panel, actualizar perfil y cerrar programa.
- Icono de aplicación y de bandeja separados; el de bandeja comunica el color de estado.
- Una lectura ausente o inválida nunca se representa como 0%.

### Privacidad y límites

No existen cuentas, nube, telemetría ni conexiones a servidores. Los datos quedan en `%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json` y se conservan 14 días.

La primera versión está enfocada en Windows x64 y R5 Ultra por receptor 2.4 GHz. Polling rate, modo competitivo, Motion Sync, reposo y patrón de uso cambian el consumo: por eso toda estimación inicial es provisional. Firmwares que devuelvan un reporte HID no reconocido fallan de forma segura y no generan porcentajes inventados.

## English

### Overview

R5 Battery Estimator is a native Windows utility that turns Attack Shark R5 Ultra 2.4 GHz receiver telemetry into battery percentage, charge state, remaining runtime, learned full-charge duration, and a local history chart. It fills the gap left by vendor software, which exposes configuration but not a user-specific runtime estimate.

It uses no Electron, Chromium, Edge, or WebView2.

### Design

The Rust HID probe validates feature-report responses and emits a JSON snapshot. A .NET 8 WinForms host owns the dashboard, notification-area lifecycle, local history, and discharge model. Valid readings are sampled every 30 seconds. The model ignores charging transitions, percentage increases, and long gaps; after 3% of observed discharge across at least 30 minutes, it derives a personalized full-charge duration. Until then, it uses an explicit provisional 200-hour fallback.

Local data is retained for 14 days at `%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json`; there are no accounts, analytics, cloud services, or telemetry uploads.

### Build

Requirements: Windows 10/11 x64, .NET 8 Desktop Runtime, Rust, and an R5 Ultra receiver for hardware validation.

```powershell
C:\Users\Leaan\.cargo\bin\cargo.exe build --manifest-path src-tauri\Cargo.toml --release --bin r5-battery-probe
dotnet publish R5BatteryEstimator.Native\R5BatteryEstimator.Native.csproj -c Release -r win-x64 --self-contained false -o outputs\native-winforms
.\outputs\native-winforms\R5BatteryEstimator.Native.exe
```

### Roadmap

1. Validate the HID protocol across firmware revisions and vendor-app states.
2. Read polling rate, Competitive Mode, Motion Sync, and sleep timeout to create automatic consumption profiles.
3. Add confidence intervals and calibration-quality indicators.
4. Move history to versioned SQLite when profile-level migrations become necessary.
5. Ship a signed per-user installer and optional autostart.

## License

Personal project; no open-source license has been assigned yet.
