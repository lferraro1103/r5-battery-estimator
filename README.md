# R5 Battery Estimator

[English](#english) · [Español](#español)

---

## Español

### Qué es

**R5 Battery Estimator** es una utilidad local para Windows pensada para el mouse **Attack Shark R5 Ultra** por receptor 2.4 GHz. Lee el porcentaje de batería informado por el receptor y lo convierte en una estimación fácil de entender de la autonomía restante.

La aplicación es nativa de Windows: no usa Chromium, Edge ni WebView2.

### Qué muestra

- Porcentaje de batería y estado de carga.
- Círculo de estado: verde, amarillo, rojo o gris si no hay una lectura válida.
- Autonomía restante en horas.
- Duración estimada de una carga completa.
- Gráfico local de las últimas 24 horas: hora frente a porcentaje.
- Estado de calibración: empieza como **Aprendiendo** y mejora con descargas reales.

### Cómo calcula las horas

Al principio usa una referencia provisional de hasta **200 horas**. Mientras la aplicación está abierta, guarda lecturas válidas de forma local y analiza las bajadas de porcentaje. Tras observar al menos un 3% de descarga durante 30 minutos o más, calcula una duración de carga completa basada en tu uso y usa ese valor para la autonomía restante.

Las cifras se presentan como estimaciones: polling rate, modo competitivo, Motion Sync, reposo y el tipo de uso pueden cambiar el consumo.

### Uso

1. Conectá el receptor 2.4 GHz del R5 Ultra.
2. Abrí `R5BatteryEstimator.Native.exe`.
3. La ventana se actualiza cada 30 segundos; también podés usar **Actualizar ahora**.
4. Minimizar o cerrar la ventana la envía a la bandeja de Windows.
5. Clic izquierdo sobre el icono de bandeja abre el menú: **Abrir panel**, **Actualizar perfil** y **Cerrar programa**.

El icono de bandeja usa color según el nivel: verde (>50%), amarillo (21–50%), rojo (≤20%) y gris cuando no existe una lectura válida.

### Privacidad

No hay cuenta, nube, telemetría ni conexión a un servidor. El historial se guarda sólo en este equipo, en:

`%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json`

### Requisitos y compilación

- Windows 10/11 x64.
- .NET Desktop Runtime 8 para ejecutar el build liviano.
- Rust para compilar el lector HID desde el código fuente.

```powershell
C:\Users\Leaan\.cargo\bin\cargo.exe build --manifest-path src-tauri\Cargo.toml --release --bin r5-battery-probe
dotnet publish R5BatteryEstimator.Native\R5BatteryEstimator.Native.csproj -c Release -r win-x64 --self-contained false -o outputs\native-winforms
```

Ejecutable generado: `outputs\native-winforms\R5BatteryEstimator.Native.exe`.

---

## English

### What it is

**R5 Battery Estimator** is a local Windows utility for the **Attack Shark R5 Ultra** connected through its 2.4 GHz receiver. It reads the battery percentage reported by the receiver and turns it into an understandable remaining-runtime estimate.

It is a native Windows application: it does not use Chromium, Edge, or WebView2.

### What it shows

- Battery percentage and charging status.
- A green, yellow, red, or gray status circle when a valid reading is unavailable.
- Remaining battery life in hours.
- Estimated duration of a full charge.
- A local 24-hour hour-versus-percentage chart.
- Calibration status: it starts as **Learning** and improves from real discharge data.

### How the estimate works

The first estimate uses a provisional reference of up to **200 hours**. While the app is running, it stores valid readings locally and observes battery drops. Once it has observed at least 3% of discharge over 30 minutes or more, it estimates full-charge duration from actual use and applies it to the remaining-runtime value.

All time values are estimates. Polling rate, competitive mode, Motion Sync, sleep settings, and usage pattern can change power consumption.

### Usage

1. Connect the R5 Ultra 2.4 GHz receiver.
2. Open `R5BatteryEstimator.Native.exe`.
3. It refreshes every 30 seconds; **Actualizar ahora** triggers a manual refresh.
4. Minimizing or closing the window sends the app to the Windows notification area.
5. Left-click the notification-area icon for **Open panel**, **Update profile**, and **Exit program**.

The notification-area icon is green above 50%, yellow from 21–50%, red at 20% or lower, and gray with no valid reading.

### Privacy

There are no accounts, cloud services, analytics, or telemetry uploads. History stays on the local machine at:

`%LOCALAPPDATA%\R5 Battery Estimator\battery-history.json`

### Requirements and build

- Windows 10/11 x64.
- .NET Desktop Runtime 8 to run the lightweight build.
- Rust to build the HID reader from source.

```powershell
C:\Users\Leaan\.cargo\bin\cargo.exe build --manifest-path src-tauri\Cargo.toml --release --bin r5-battery-probe
dotnet publish R5BatteryEstimator.Native\R5BatteryEstimator.Native.csproj -c Release -r win-x64 --self-contained false -o outputs\native-winforms
```

Generated executable: `outputs\native-winforms\R5BatteryEstimator.Native.exe`.
