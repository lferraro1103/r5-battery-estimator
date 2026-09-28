# Project Research Summary

**Project:** R5 Battery Estimator  
**Domain:** Utilidad local de Windows para telemetría HID y estimación empírica de autonomía  
**Researched:** 2026-09-28  
**Confidence:** MEDIUM-HIGH

## Executive Summary

R5 Battery Estimator es una aplicación residente, local y de instancia única para Windows que consulta por HID el porcentaje y estado de carga del Attack Shark R5 Ultra, conserva evidencia histórica y aprende estimaciones de autonomía por perfil. La forma correcta de construirla no es como un indicador que extrapola cada lectura, sino como una canalización auditable: un único dueño del dispositivo serializa las transacciones HID, registra observaciones inmutables, separa segmentos válidos de intervalos censurados y publica un único estado para bandeja, ventana y alertas.

La recomendación es Tauri 2.12.0 con núcleo Rust 1.98.1, `hidapi` sobre `hid.dll`, SQLite embebido mediante `rusqlite`, y una ventana React/TypeScript/Vite creada bajo demanda. Esta decisión reemplaza las referencias ilustrativas a Electron y `node-hid` de la investigación de arquitectura, pero conserva sus límites: host autoritativo, puerto HID estrecho, repositorio único, resolución canónica de perfiles, estimador reconstruible y presentación sin acceso directo a hardware o SQL. El probe `node-hid` existente permanece como oráculo diagnóstico y fixture de protocolo, no como dependencia distribuida.

El riesgo principal es producir una autonomía precisa en apariencia a partir de porcentaje entero, mesetas, sueño, cargas parciales o perfiles desconocidos. Debe mitigarse antes del pulido visual: caracterizar el firmware y la convivencia con el software oficial, preservar discontinuidades y razones de exclusión, validar la cadencia contra latencia de juego y tratar la incertidumbre como salida del producto. La interfaz siempre separará porcentaje informado, tiempo estimado y confianza; ausencia de respuesta nunca significa 0%, y ningún historial de otro perfil podrá actualizar el perfil activo.

## Key Findings

### Recommended Stack

La pila especializada de [STACK.md](STACK.md) prevalece sobre alternativas condicionales de otros documentos. El backend Rust es dueño exclusivo de HID, SQLite, sondeo, estimación, bandeja, autoinicio y notificaciones; la WebView consume DTOs tipados y comandos estrechos. Se deben fijar versiones y lockfiles, mantener los assets offline y validar cada actualización con hardware e instalador.

**Core technologies:**

- **Rust 1.98.1 + Tauri 2.12.0:** host residente, ciclo de vida, bandeja, IPC y empaquetado — menor huella que Electron y sin Chromium distribuido.
- **`hidapi` 2.6.7 (`windows-native`):** feature reports sobre `hid.dll` — porta directamente el intercambio ya validado y evita un sidecar Node/Python.
- **SQLite 3.53.2 + `rusqlite` 0.40.2 + `rusqlite_migration` 2.6.0:** evidencia local transaccional y migraciones reconstruibles — un escritor serializado, sin SQL desde el frontend.
- **React 19.3.0 + TypeScript 7.0.2 + Vite 8.3.1:** ventana detallada y tipada — el renderer es presentación, no motor de recolección.
- **Tauri plugins oficiales:** instancia única primero, autoinicio, notificaciones y logs rotados — updater se difiere hasta existir distribución recurrente.
- **NSIS por usuario + WebView2 Evergreen:** instalación Windows x64 sin elevación; Windows 11 es objetivo y Windows 10 compatibilidad secundaria si sigue siendo necesaria.

### Expected Features

La investigación de [FEATURES.md](FEATURES.md) confirma que el producto debe distinguir siempre porcentaje del firmware, estimaciones aprendidas y evidencia/confianza. Las decisiones explícitas de [PROJECT.md](../PROJECT.md) quedan bloqueadas: perfiles automáticos numerados definidos solo por polling rate, Competitive Mode, Motion Sync y tiempo de reposo; comparación al inicio y con `Revisar perfil`; autonomía cotidiana y juego continuo; almacenamiento local; umbral global inicial de 15%.

**Must have (table stakes):**

- Porcentaje y carga atribuidos al dispositivo, con frescura y estados explícitos de sueño, desconexión y recuperación.
- Bandeja útil y ventana detallada con autonomía cotidiana y juego continuo claramente diferenciadas.
- Valor provisional desde el primer uso, acompañado por estado `Inicial`, `Aprendiendo`, `Confiable` o `Antiguo`.
- Aprendizaje incremental persistente, aislado por perfil, con historial relevante y recuperación segura.
- Perfiles automáticos canónicos, comparación al inicio y revisión manual sin duplicados por lecturas parciales.
- Autoinicio visible y controlable, bajo impacto residente y alerta configurable/deduplicada al cruzar 15%.

**Should have (competitive):**

- Rangos de autonomía explicables para dos escenarios, corregidos progresivamente con evidencia real.
- Confianza basada en descarga útil, episodios independientes, cobertura activa, dispersión y recencia.
- Degradación honesta y autorrecuperable ante sueño, reanudación, cambio de perfil o pérdida del receptor.
- Trazabilidad compacta que explique cambios de estimación sin convertirse en una plataforma analítica.

**Defer (v2+):**

- Otros mouse, Bluetooth, administración manual de perfiles y modificación de ajustes del dispositivo.
- Nube, cuentas, telemetría remota, suite de periféricos, detección automática de juegos y salud física de batería.
- Exportación/analítica avanzada, retención indefinida, reglas complejas de alertas y actualización automática.

### Architecture Approach

La arquitectura de [ARCHITECTURE.md](ARCHITECTURE.md) se implementará como un solo ejecutable por usuario y de instancia única. El host adquiere el lock antes de abrir SQLite o HID; un actor de hardware mantiene un único handle y serializa batería/configuración; cada lectura aceptada se agrega antes de derivar segmentos y modelos. El perfil se resuelve desde una huella versionada de exactamente cuatro ajustes. Los modelos y vistas son caches reconstruibles de observaciones crudas. La ventana es perezosa y puede cerrarse mientras el host y la bandeja continúan funcionando.

**Major components:**

1. **Host runtime y puente de ciclo de vida** — instancia única, startup/shutdown, bandeja, suspensión, reanudación y eventos de dispositivo.
2. **Coordinador de adquisición** — agenda coalescente, un solo carril HID, timeout, backoff, circuit breaker y métricas.
3. **Puerto/adaptador R5 + codec** — selección VID `0x373E`/PID `0x0047`/interfaz 2, request exacto y validación estricta de respuestas.
4. **Repositorio y pipeline de observación** — SQLite local, eventos inmutables, relojes UTC/monotónico, migraciones y replay idempotente.
5. **Resolver de perfiles y constructor de segmentos** — huella canónica, numeración transaccional y clasificación válida/censurada/rechazada.
6. **Estimador por perfil** — tasas robustas cotidiana y activa, priors iniciales, rangos, confianza, deriva y versionado.
7. **Servicio de proyección y políticas** — snapshot único para bandeja/ventana, umbral, deduplicación e historial resumido.

### Battery-Estimation Methodology

- El porcentaje informado es una observación cuantizada/censurada, no estado electroquímico exacto. Las mesetas se acumulan hasta un descenso real; no cuentan como muestras independientes de consumo cero.
- Solo entrenan intervalos no cargando, dentro de una misma sesión continua, con perfil confirmado en ambos extremos, tiempo monotónico válido y descenso plausible.
- Suspensión, desconexión, reinicio, carga, cambio de perfil de momento desconocido, respuesta inválida y aumento no cargando cierran o censuran segmentos.
- La autonomía cotidiana usa la tasa robusta observada con la mezcla real de actividad/reposo. Juego continuo usa evidencia activa; hasta tenerla, presenta un prior de configuración claramente provisional o un rango amplio.
- Las actualizaciones deben ser robustas y ponderadas por episodios independientes, recencia y dispersión. Un cambio de algoritmo crea una nueva versión y reconstruye derivados desde evidencia cruda.

### Confidence and Uncertainty Policy

- Mostrar estados discretos explicables, no un porcentaje de confianza inventado.
- La confianza sube por caída total observada, diversidad de episodios, cobertura activa, rango de carga cubierto y validación temporal; no por cantidad de sondeos idénticos.
- La precisión visible debe corresponder a la evidencia: rangos o redondeo amplio con poca calibración, nunca minutos exactos por defecto.
- Datos antiguos conservan la última lectura con hora y causa, pero no continúan un countdown ficticio ni disparan alertas.
- Holdouts temporales e interval coverage deben poder degradar automáticamente un perfil antes considerado confiable; ausencia reciente y deriva reducen confianza.

### Critical Pitfalls

1. **Confundir porcentaje entero con medición continua** — caracterizar pasos, redondeo y cadencia; modelar cada descenso como intervalo, no instante exacto.
2. **Mezclar regímenes o perfiles** — separar actividad, reposo, carga y discontinuidades; exigir perfil completo y canónico antes de aprender.
3. **Crear confianza por volumen de muestras** — contar información independiente y validar error/cobertura sobre episodios posteriores.
4. **Permitir múltiples dueños HID u operaciones solapadas** — instancia única, un handle, cola serial y triggers coalescentes.
5. **Convertir fallos en 0% o atribuir tiempo dormido** — estados explícitos, doble reloj y nuevos baselines tras reanudar/reconectar.
6. **Conservar solo agregados derivados** — evidencia cruda inmutable, versiones de protocolo/perfil/modelo y reconstrucción determinista.
7. **Perturbar el juego con polling agresivo** — handle persistente, cadencia conservadora/adaptativa, backoff y gate de latencia/recursos.
8. **Alertas repetidas y shell frágil** — cruce descendente persistido con histéresis; restaurar bandeja tras `TaskbarCreated` y consultar estado real de autoinicio.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Caracterización del protocolo y contratos de dominio

**Rationale:** La matemática y los umbrales de confianza dependen de conocer granularidad, cadencia, carga y convivencia reales.  
**Delivers:** Fixtures byte a byte, trazas de descarga/carga, histograma de pasos/mesetas, prueba con software oficial abierto/cerrado, contratos de observación/segmento/estimación y simulador determinista.  
**Addresses:** Lectura directa, valor inicial y base para los cuatro ajustes de perfil.  
**Avoids:** Diseñar alrededor de cinco lecturas idénticas o tratar el spike como implementación de producción.

### Phase 2: Runtime, persistencia y colector de bajo impacto

**Rationale:** Un único dueño durable debe existir antes de multiplicar comandos, UI o aprendizaje.  
**Delivers:** Tauri/Rust, instancia única, tray de salud mínimo, SQLite/migraciones/replay, adaptador `hidapi`, cola serial, handle persistente, timeout/backoff y métricas de latencia.  
**Uses:** Tauri, `hidapi`, `rusqlite`, logs rotados y fixtures del probe.  
**Implements:** Host runtime, repository, protocol codec y acquisition coordinator.  
**Avoids:** Pollers duplicados, operaciones superpuestas, enumeración por tick y aprendizaje no durable.

### Phase 3: Perfiles y evidencia confiable

**Rationale:** Las observaciones necesitan etiquetas estables y discontinuidades explícitas antes de entrenar modelos.  
**Delivers:** Lectura validada de polling rate, Competitive Mode, Motion Sync y reposo; huella canónica; perfiles numerados; comparación al inicio/manual/periódica; sesiones, carga y segmentos válidos/censurados.  
**Addresses:** Perfil automático, `Revisar perfil`, aislamiento de aprendizaje e historial relevante.  
**Avoids:** Fragmentación, perfiles por estados parciales, contaminación cruzada y atribución de intervalos con momento de cambio desconocido.

### Phase 4: Estimador, calibración y confianza

**Rationale:** Solo la evidencia perfilada y censurada puede sostener dos proyecciones honestas.  
**Delivers:** Priors iniciales, tasas robustas cotidiana/activa, rangos, estados de calibración, evidencia explicable, holdouts temporales, deriva/recencia y rebuild por versión.  
**Addresses:** Autonomía cotidiana, juego continuo, aprendizaje incremental y requisito de confianza.  
**Avoids:** Falsa precisión, mesetas como cero consumo, confianza por conteo y dominio de datos antiguos.

### Phase 5: Experiencia de bandeja, ventana, startup y alertas

**Rationale:** La presentación debe consumir un snapshot estable y políticas persistentes, no definir la verdad del dominio.  
**Delivers:** Tooltip/iconos accesibles, ventana detallada perezosa, historial compacto, `Revisar perfil`, ajustes de umbral/autoinicio, notificación por cruce con histéresis, estados stale/disconnected y recuperación del tray.  
**Addresses:** Acceso rápido, ventana detallada, umbral inicial de 15%, autoinicio y operación silenciosa.  
**Avoids:** Tray/ventana divergentes, alert spam, ocultar estado deshabilitado y presentar datos antiguos como actuales.

### Phase 6: Soak, recuperación y release

**Rationale:** Las carreras de ciclo de vida y el impacto real no quedan demostrados con unit tests.  
**Delivers:** NSIS instalado, pruebas de sleep/hibernate/hotplug/Explorer restart/crash/migración/corrupción/carga parcial/vendor app, soak multi-día, límites de logs/DB y comparación de latencia/recursos con el monitor apagado/encendido.  
**Addresses:** Confiabilidad residente y compatibilidad Windows real.  
**Avoids:** Aprobar solo happy paths, asumir que notifications/autostart funcionan en `tauri dev` y liberar polling que afecte el mouse.

### Phase Ordering Rationale

- La verdad del protocolo precede al colector; el colector durable precede a los perfiles; perfiles y discontinuidades preceden al estimador.
- La UI y alertas se agrupan después del modelo porque deben consumir el mismo read model y estado persistente de deduplicación.
- El lock de instancia, storage y seams de ciclo de vida llegan temprano aunque el pulido de startup sea tardío: toda corrección posterior depende de un solo escritor/poller.
- El soak final valida cruces entre componentes, pero cada fase debe conservar gates propios de hardware, replay y degradación.

### Research Flags

Phases likely needing deeper research during planning:

- **Phase 1:** granularidad/refresh del firmware, semántica de carga y `100%`, trazas largas y coexistencia con `ATTACK SHARK GAMING.exe`.
- **Phase 2:** cadencia/timeout seguros, efecto a máximo polling rate y comportamiento de handle largo tras errores/suspensión.
- **Phase 3:** comandos reales de los cuatro ajustes aún no tienen la validación directa de batería.
- **Phase 4:** proxy ligero de actividad, priors y umbrales de calibración requieren sesiones reales y evaluación temporal.
- **Phase 6:** detalles finales de NSIS/autostart, firma y upgrade antes de congelar distribución.

Phases with standard patterns (skip research-phase):

- **Phase 5:** patrones Tauri/Windows de bandeja, IPC tipado, settings y notificaciones están documentados; requiere pruebas, no investigación exploratoria, salvo que cambie el mecanismo de packaging.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Versiones y APIs se contrastaron con fuentes oficiales; la selección Tauri/Rust/hidapi es coherente con huella y ownership. Seguridad de coexistencia y consumo solo se confirman con hardware. |
| Features | HIGH | La mayor parte deriva de decisiones explícitas del producto y estados observables; detalles de competidores no condicionan el MVP. |
| Architecture | MEDIUM-HIGH | Ownership, append/derive, SQLite y lifecycle son patrones sólidos; las menciones Electron/node-hid se reinterpretan como puertos y adaptadores, no como stack final. |
| Pitfalls | HIGH | Los riesgos de cuantización, censura, concurrencia, sleep, alertas y persistencia están bien respaldados; magnitudes específicas del R5 siguen sin medir. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **Telemetría real:** medir paso, redondeo, refresh, low-battery, charge-flag lag, rebote y significado de 100% en trazas controladas.
- **Configuración HID:** validar en hardware los cuatro comandos de settings y su estabilidad con la aplicación oficial activa.
- **Convivencia/latencia:** determinar sharing, timeout, cadencia y presupuesto de CPU/USB sin impacto perceptible en juego.
- **Clasificación activa:** elegir y validar un proxy de bajo costo; no usar hooks ni detección de procesos.
- **Política estadística:** fijar priors, thresholds y precisión visible solo después de múltiples episodios independientes y holdouts temporales.
- **Windows 10:** confirmar si sigue dentro del producto antes de convertir compatibilidad secundaria en gate de release.
- **Distribución:** definir firma y ruta de upgrades cuando el updater entre en alcance; no bloquear el MVP local con ello.

## Sources

### Primary (HIGH confidence)

- [Project definition](../PROJECT.md) — alcance, decisiones explícitas, restricciones y valor central.
- [Validated R5 telemetry spike](../spikes/001-current-battery-telemetry/README.md) — VID/PID/interfaz, request y lecturas directas.
- [Stack research](STACK.md) — versiones, APIs oficiales y recomendación Tauri/Rust.
- [Feature research](FEATURES.md) — table stakes, diferenciadores, anti-features y estados de aceptación.
- [Architecture research](ARCHITECTURE.md) — ownership, datos, flujos, estimación y orden de componentes.
- [Pitfall research](PITFALLS.md) — riesgos, señales tempranas, mitigaciones y gates por fase.
- [Microsoft HID feature reports](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidsdi/nf-hidsdi-hidd_getfeature) — semántica de feature reports.
- [SQLite WAL](https://www.sqlite.org/wal.html) y [corruption guidance](https://www.sqlite.org/howtocorrupt.html) — persistencia, sidecars y recuperación.
- [NIST censoring guidance](https://www.itl.nist.gov/div898/handbook/apr/section1/apr131.htm) — base para observaciones censuradas.

### Secondary (MEDIUM confidence)

- [Tauri documentation](https://tauri.app/) — shell, ciclo de vida y plugins de escritorio.
- [`hidapi` crate](https://crates.io/crates/hidapi) — transporte HID nativo de Windows.
- [Windows power events](https://learn.microsoft.com/en-us/windows/win32/power/system-power-management-events) y [device notifications](https://learn.microsoft.com/en-us/windows/win32/devio/registering-for-device-notification) — discontinuidades y recuperación.
- [Windows notifications guidance](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-ux-guidance) — comportamiento no garantizado y control del usuario.
- [Battery-estimation literature cited in PITFALLS.md](PITFALLS.md#storage-and-estimation-evidence) — cautela sobre cuantización, capacidad e histéresis; no es específica del firmware R5.

### Tertiary (LOW confidence)

- Comparaciones de productos competidores resumidas en [FEATURES.md](FEATURES.md) — útiles solo como contexto UX; no determinan alcance ni arquitectura.

---
*Research completed: 2026-09-28*  
*Ready for roadmap: yes*
