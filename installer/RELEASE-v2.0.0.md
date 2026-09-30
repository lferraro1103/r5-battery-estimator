# R5 Battery Estimator v2.0.0

## English

Native Windows x64 application for the Attack Shark R5 Ultra, connected through its 2.4 GHz receiver. No browser, WebView, .NET runtime or companion console.

Download **R5BatteryEstimatorV2-Setup.exe** below. The installer installs per user, includes the full shark icon, optional desktop shortcut, an English/Spanish wizard, GPL v3 license page and an uninstaller. The application's tray menu offers Windows startup, manual refresh, open panel and exit.

- Device-reported percentage and charging state, updated every 30 seconds.
- Battery history graph and estimated remaining hours, with explicit calibration/confidence labels. Estimates are not guaranteed runtime; the initial model is provisional and full-charge runtime requires sufficient validated full-discharge evidence.
- Bounded HID pending-response recovery. If the receiver cannot return a new value (including possible mouse sleep), the last known percentage is clearly identified as stale; current autonomy is withheld until a valid reading returns. No fabricated 0%.
- Data stays local. Automatic configuration profiles are **not implemented yet**; the menu marks them pending.
- Full shark branding is embedded in both the application and installer at multiple sizes; installer and native crate report version 2.0.0.

22 automated tests passed; actual receiver readings were verified during development. Complete matching source is available at this release tag under **GNU GPL v3**. The installer is **not code-signed**, so Windows may display an unknown-publisher/SmartScreen warning. Verify the SHA256 checksum before running it.

## Español

Aplicación nativa para Windows x64 y el Attack Shark R5 Ultra conectado por su receptor de 2,4 GHz. Sin navegador, WebView, runtime .NET ni consola adicional.

Descargá **R5BatteryEstimatorV2-Setup.exe** abajo. El instalador instala por usuario e incluye el icono del tiburón completo, acceso directo opcional al escritorio, asistente en español/inglés, pantalla de licencia GPL v3 y desinstalador. El menú de bandeja ofrece inicio con Windows, actualización manual, abrir panel y salir.

- Porcentaje y estado de carga informados por el dispositivo, actualizados cada 30 segundos.
- Gráfico de historial y horas restantes estimadas, con calibración y confianza explícitas. Las estimaciones no son autonomía garantizada: el modelo inicial es provisional y la duración de carga completa requiere suficientes descargas completas validadas.
- Recuperación acotada de respuestas HID pendientes. Si el receptor no devuelve un valor nuevo (incluido un posible reposo del mouse), el último porcentaje se identifica claramente como anterior; la autonomía actual se oculta hasta recibir una lectura válida. Nunca se inventa un 0%.
- Datos almacenados localmente. Los perfiles automáticos por configuración **todavía no están implementados**; el menú los identifica como pendientes.
- Icono del tiburón completo integrado en aplicación e instalador en varios tamaños; instalador y crate nativo informan versión 2.0.0.

Pasaron 22 pruebas automatizadas y se verificaron lecturas del receptor real durante el desarrollo. El código fuente completo correspondiente está disponible en la etiqueta de esta release bajo **GNU GPL v3**. El instalador **no está firmado digitalmente**, por lo que Windows puede mostrar un aviso de editor desconocido/SmartScreen. Verificá la suma SHA256 antes de ejecutarlo.
