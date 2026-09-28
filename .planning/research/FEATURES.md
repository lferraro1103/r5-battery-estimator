# Feature Landscape

**Domain:** Utilidad local de bandeja para estimar la autonomía de un mouse gaming inalámbrico
**Project:** R5 Battery Estimator
**Researched:** 2026-09-28
**Overall confidence:** HIGH para las decisiones del producto y los estados derivados de la telemetría validada; MEDIUM para patrones de UX de Windows; LOW para detalles no documentados públicamente de competidores

## Product Principle

La interfaz debe mantener tres conceptos visibles y separados:

1. **Porcentaje informado por el dispositivo:** dato del firmware, no una medición física independiente de capacidad.
2. **Tiempo restante estimado:** resultado del modelo para el perfil activo, mostrado por separado para uso cotidiano y juego continuo.
3. **Confianza/calibración:** indica cuánto historial útil respalda cada estimación; nunca debe disfrazarse como precisión del porcentaje.

Una lectura válida puede mostrar `90% informado por el dispositivo` aun cuando la autonomía siga `En calibración`. Una lectura ausente nunca debe convertirse en `0%`, y una estimación antigua nunca debe aparecer como actual sin una marca de antigüedad.

## Table Stakes

Features users expect. Missing = product feels incomplete or misleading.

| Feature | Observable, testable behavior | Why Expected | Complexity | Notes |
|---------|-------------------------------|--------------|------------|-------|
| Estado de batería claramente atribuido | Con una respuesta HID válida, bandeja y ventana muestran el porcentaje con una etiqueta o ayuda que lo identifica como **informado por el dispositivo**, junto con `Cargando`/`No cargando` y hora de última lectura. | Es el dato base validado y evita que el usuario confunda firmware con capacidad real. | Low | El spike confirmó porcentaje y bit de carga en el receptor 2.4 GHz. |
| Consulta rápida desde la bandeja | El icono/tooltip permite conocer porcentaje y una autonomía resumida sin abrir la ventana; un clic abre el detalle. El menú incluye como mínimo `Abrir`, `Revisar perfil` y `Salir`. | Una utilidad residente debe ser útil con una interacción mínima. | Medium | El icono también debe distinguir normal, cargando y sin datos sin depender solo del color. |
| Ventana detallada con jerarquía estable | La vista principal separa `Batería informada`, `Uso cotidiano estimado`, `Juego continuo estimado`, `Perfil activo`, `Calibración/confianza`, consumo aprendido y actividad reciente relevante. | Hace auditables las estimaciones sin saturar el acceso rápido. | Medium | No mezclar ambos tiempos en una sola cifra. |
| Dos estimaciones con significado explícito | Para cada perfil, la app muestra autonomía cotidiana y horas de juego continuo como campos distintos, con texto breve que explica que cotidiano incorpora reposo/uso intermitente y juego representa descarga activa sostenida. | Responde a dos preguntas diferentes y evita que una cifra ambigua parezca universal. | High | La semántica debe permanecer constante entre perfiles y versiones. |
| Valor inmediato en primer inicio | En la primera lectura válida se muestra el porcentaje de inmediato y estimaciones iniciales marcadas `Provisionales` o `En calibración`; la UI indica qué observación falta para mejorar, sin exigir un ciclo completo antes de ser útil. | El aprendizaje no puede dejar una aplicación vacía durante días. | Medium | No mostrar minutos exactos con confianza baja; preferir `aprox.` o un rango amplio si el modelo lo permite. |
| Confianza y progreso de calibración | Cada estimación muestra un estado discreto (`Inicial`, `Aprendiendo`, `Confiable`) y una explicación accesible basada en evidencia observable, por ejemplo cantidad de descarga útil o sesiones registradas. | Es un requisito explícito de exactitud y evita falsa precisión. | Medium | No usar un porcentaje de confianza inventado si no existe una métrica calibrada. |
| Aprendizaje incremental por perfil | Tras observar descargas/cargas válidas, el tiempo estimado y consumo aprendido se actualizan sin acción manual. Cerrar/reabrir o reiniciar Windows conserva el aprendizaje. | Es el valor central del producto. | High | Las muestras cargando, inválidas o pertenecientes a otro perfil no contaminan la tasa de descarga. |
| Perfiles automáticos y numerados | La identidad del perfil usa exclusivamente polling rate, Competitive Mode, Motion Sync y tiempo de reposo. Al detectar una combinación nueva crea `Perfil N`; si coincide con una existente la reactiva. | Conserva comparabilidad sin imponer administración al usuario. | High | DPI, debounce, LOD, Angle Snap y Ripple Control no participan en la identidad. |
| Comparación al inicio y revisión bajo demanda | Al iniciar Windows se consulta la configuración y se selecciona el perfil coincidente; `Revisar perfil` ejecuta la misma comparación y comunica `Sin cambios`, `Perfil N activado` o `Perfil N creado`. | Cubre cambios realizados mientras la utilidad no estaba activa y ofrece recuperación manual. | High | No crear perfiles duplicados por errores transitorios; solo después de una lectura de configuración completa y válida. |
| Estado de carga seguro | Mientras carga, se muestran porcentaje y `Cargando`; se pausa el aprendizaje de descarga. Las estimaciones pueden permanecer visibles como referencia pero deben rotularse `estimación al desconectar`, o mostrarse no disponibles si el modelo no puede sostenerlas. | Cargar rompe la relación entre tiempo y descenso de porcentaje. | Medium | La transición a carga termina el episodio de batería baja y permite un aviso futuro después de otra descarga. |
| Estado dormido/sin respuesta | Si el mouse no responde temporalmente, la app muestra `Mouse dormido o no disponible`, conserva la última lectura solo como `Última lectura: X% hace Y`, marca el tiempo estimado como antiguo/no disponible y reintenta sin exigir reinicio. | Un mouse inalámbrico duerme con frecuencia; tratarlo como error fatal haría la utilidad poco confiable. | Medium | No emitir aviso de batería baja basándose únicamente en una lectura antigua. |
| Estado desconectado y recuperación automática | Si falta el receptor o no hay interfaz compatible, la bandeja y la ventana muestran `Receptor no encontrado`; al reconectarlo, la app recupera lecturas, compara el perfil y continúa el historial correspondiente sin reiniciarse. | Desconexiones USB y reanudación de Windows son normales. | High | Los fallos repetidos deben usar reintentos moderados y no generar notificaciones repetitivas. |
| Inicio automático visible y controlable | Después de instalar/activar la utilidad, esta inicia con Windows y continúa en segundo plano. La ventana de ajustes muestra el estado de inicio automático y respeta si el usuario lo desactiva. | El aprendizaje necesita continuidad, pero el comportamiento residente no debe ser oculto. | Medium | Si el inicio automático falla, mostrar un estado corregible, no asumir que está activo. |
| Aviso de batería baja configurable | Umbral inicial de 15%; el usuario puede cambiarlo o desactivar avisos. Se envía una sola notificación al cruzarlo hacia abajo con una lectura fresca, y el clic abre el detalle. No se repite en cada sondeo. | Previene pérdida de carga sin fatiga de notificaciones. | Medium | Respetar controles y No molestar de Windows. Rehabilitar el aviso tras cargar o volver claramente por encima del umbral. |
| Historial relevante, no telemetría cruda | La ventana muestra eventos recientes útiles: cambios de porcentaje, inicio/fin de carga, cambio de perfil, periodos de datos ausentes y avance de calibración. Los eventos incluyen hora y perfil. | Permite entender por qué cambió una estimación y diagnosticar datos pobres. | Medium | Un registro resumido basta en v1; no se requiere una plataforma de análisis. |
| Persistencia local y recuperación de estado | Reiniciar la aplicación o Windows conserva perfiles, muestras aceptadas, umbral y estado de calibración. Un archivo corrupto o versión incompatible no impide arrancar: se aísla el dato inválido, se informa la recuperación y se vuelve a calibrar. | Perder aprendizaje destruye la propuesta de valor. | High | La recuperación debe preservar datos sanos cuando sea posible y nunca inventar confianza. |
| Bajo impacto en segundo plano | Durante uso normal y juego, la utilidad sondea a una cadencia moderada, no bloquea entrada del mouse y permanece silenciosa salvo eventos accionables. | La utilidad no puede perjudicar la latencia del periférico que observa. | High | El usuario no necesita configurar la frecuencia de sondeo en v1. |

## Differentiators

Features that set product apart. Not universally expected, but materially valuable.

| Feature | Value Proposition | Observable, testable behavior | Complexity | Notes |
|---------|-------------------|-------------------------------|------------|-------|
| Autonomía aprendida para dos escenarios | Convierte un porcentaje de firmware en una respuesta práctica para trabajo diario y sesiones de juego. | Con el mismo porcentaje y perfil, la UI puede mostrar dos tiempos distintos; ambos cambian cuando se incorpora evidencia nueva y conservan su propia explicación. | High | Diferenciador principal. |
| Confianza explicable en vez de precisión teatral | Permite decidir si confiar en la cifra y entiende por qué aún varía. | Al abrir el detalle, el usuario puede ver el estado de calibración, evidencia disponible y última actualización sin interpretar una fórmula interna. | Medium | La confianza debe bajar o volver a `Aprendiendo` cuando un perfil nuevo carece de datos. |
| Segmentación automática por consumo real esperado | Evita promediar configuraciones incompatibles sin pedir al usuario que administre perfiles. | Cambiar cualquiera de los cuatro ajustes definidos y pulsar `Revisar perfil` activa o crea el perfil correcto; volver a una combinación anterior recupera su aprendizaje. | High | Mantener perfiles numerados y automáticos es una decisión explícita. |
| Degradación honesta y autorrecuperable | Sigue siendo comprensible cuando el mouse duerme, el receptor se desconecta o Windows reanuda. | Cada estado muestra causa probable, última lectura y próximo comportamiento; una lectura válida posterior limpia el estado de error automáticamente. | High | Diferencia una herramienta confiable de un indicador que se queda congelado. |
| Corrección progresiva sin borrar el punto de partida | Ofrece valor inmediato y converge hacia el consumo particular del equipo. | La estimación inicial queda identificada como base; después de suficiente descarga válida el perfil pasa a `Aprendiendo` y luego `Confiable`, manteniendo separación entre perfiles. | High | Los criterios exactos requieren diseño del modelo y pruebas con ciclos reales. |
| Trazabilidad compacta de cambios | Da contexto suficiente para explicar saltos sin convertir la utilidad en un dashboard. | El historial permite relacionar una revisión de perfil, carga o periodo sin datos con la estimación actual. | Medium | Limitar retención y densidad visual en v1. |

## Anti-Features

Features to explicitly NOT build in v1.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Compatibilidad con otros mouse o Bluetooth | Multiplica protocolos y estados antes de validar el flujo completo del R5 Ultra por receptor 2.4 GHz. | Diseñar límites internos extensibles, pero enviar y probar un solo dispositivo/transporte. |
| Escribir o modificar ajustes del mouse | Eleva el riesgo de interferir con el periférico y duplica el software oficial. | Leer los cuatro ajustes de consumo únicamente para clasificar el perfil. |
| Usar DPI, debounce, LOD, Angle Snap o Ripple Control como identidad | Contradice la decisión explícita y fragmentaría el aprendizaje en perfiles poco útiles. | Identificar por polling rate, Competitive Mode, Motion Sync y reposo solamente. |
| Renombrar, borrar, fusionar, fijar o editar perfiles | Introduce un sistema de administración y decisiones de migración antes de validar el aprendizaje automático. | Crear, reactivar y numerar perfiles automáticamente; mostrar sus cuatro ajustes en modo lectura. |
| Cuenta, nube, sincronización o telemetría remota | No aporta al caso de un solo usuario/dispositivo y amplía privacidad, seguridad y soporte. | Guardar configuración e historial solo en el equipo. |
| Suite de periféricos: macros, RGB, remapeo, biblioteca de juegos o actualizador de firmware | G HUB y Synapse muestran cuánto crece ese alcance; no mejora la pregunta central de autonomía. | Integrarse de forma no invasiva con la configuración ya aplicada por el software oficial. |
| Horas exactas con pocos datos | La granularidad del firmware y las variaciones de uso no sostienen precisión minuto a minuto. | Mostrar estado de calibración, `aprox.` y redondeo coherente; ocultar o ampliar el rango cuando la evidencia sea insuficiente. |
| Estimación activa durante datos antiguos como si fuera actual | Un mouse dormido o desconectado puede dejar una cifra plausible pero obsoleta. | Mostrar la última lectura con antigüedad y marcar estimaciones como no disponibles/antiguas hasta recuperar telemetría. |
| Predicción de salud, ciclos o degradación física de batería | Porcentaje y estado de carga no aportan capacidad, voltaje ni temperatura suficientes para afirmaciones confiables. | Limitarse a consumo observado y tiempo restante bajo el perfil activo. |
| Detección de juego por procesos, hooks o monitoreo de ventanas | Añade permisos, falsos positivos y costo en segundo plano sin ser necesaria para calcular el escenario continuo. | Definir `juego continuo` como una proyección del consumo activo aprendido, no como detección automática de juegos. |
| Sondeo HID de alta frecuencia configurable | Puede competir con el dispositivo, consumir recursos y sugerir una precisión inexistente. | Usar una cadencia interna conservadora, adaptable ante sueño/error, validada contra latencia. |
| Notificaciones por cada lectura baja, desconexión breve o cambio de porcentaje | Produce fatiga y conflictos con No molestar. | Notificar solo cruces de umbral frescos y eventos realmente accionables, con deduplicación por episodio. |
| Gráficos densos, exportación CSV, importación, edición de muestras o retención indefinida | Consume el presupuesto de v1 en análisis y soporte de datos en vez de mejorar la estimación. | Mostrar historial reciente resumido y conservar internamente solo lo necesario para aprendizaje/diagnóstico. |
| Umbrales diferentes por perfil o reglas avanzadas de alertas | Crea complejidad de configuración sin evidencia de necesidad. | Un umbral global, inicialmente 15%, con opción de desactivar. |
| Actualizaciones automáticas silenciosas como requisito de v1 | Agrega distribución, firma y recuperación de actualización al camino crítico. | Entregar una instalación estable; tratar actualizaciones como una fase posterior. |

## Required State Behavior

| Situation | Tray / quick view | Detailed window | Learning | Notifications |
|-----------|-------------------|-----------------|----------|---------------|
| Primera ejecución, lectura válida | Porcentaje + tiempo provisional | `Inicial`/`En calibración`, dos estimaciones claramente aproximadas | Crea/selecciona perfil y empieza a registrar | Ninguna salvo cruce real del umbral |
| Descarga estable | Porcentaje + resumen actualizado | Dos tiempos, consumo, confianza y última lectura | Acepta muestras válidas del perfil activo | Una al cruzar el umbral hacia abajo |
| Cargando | Icono/etiqueta de carga + porcentaje | `Cargando`; estimación rotulada para después de desconectar o no disponible | Pausa tasa de descarga; registra evento de carga | No repetir batería baja; rearma el episodio |
| Mouse dormido / sin respuesta temporal | Estado neutral, no `0%` | Última lectura con antigüedad; causa probable y reintento | No añade muestra ni degrada otro perfil | Ninguna por el dato antiguo |
| Receptor ausente | `Desconectado` | Instrucción breve para reconectar; última lectura separada | Pausado | Sin spam; una desconexión no es emergencia |
| Perfil cambió | Indica `Perfil N` tras validar | Ajustes del perfil y estado de calibración propio | Activa existente o crea nuevo; nunca mezcla datos | Aviso dentro de la app; toast no necesario en v1 |
| Reanudación/reconexión | Vuelve a estado normal sin reinicio | Limpia error, actualiza lectura y confirma perfil | Continúa el perfil coincidente | Evalúa umbral solo con lectura fresca y deduplicada |
| Persistencia dañada | Estado recuperable, no cierre silencioso | Explica qué se restableció y que debe recalibrar | Conserva datos sanos o crea base nueva con confianza inicial | Solo aviso local accionable; no bucle de toasts |

## Feature Dependencies

```text
Lectura HID validada
  -> Estado de porcentaje/carga
  -> Bandeja + ventana detallada
  -> Muestreo persistente
      -> Estimación inicial
      -> Aprendizaje por perfil
          -> Autonomía cotidiana
          -> Juego continuo
          -> Confianza/calibración
          -> Historial explicativo

Lectura de los 4 ajustes de consumo
  -> Huella estable del perfil
      -> Comparación al inicio
      -> Revisar perfil
      -> Crear/reactivar Perfil N
      -> Aislamiento del aprendizaje

Máquina de estados de conectividad/carga
  -> Suspensión de muestras inválidas
  -> Recuperación tras sueño/reconexión
  -> Aviso de umbral sin duplicados
```

## MVP Recommendation

Prioritize:

1. **Estados confiables de dispositivo:** lectura fresca, cargando, dormido/no disponible, receptor ausente y recuperación automática.
2. **Bandeja y detalle honestos:** porcentaje informado, dos estimaciones, antigüedad, calibración/confianza e historial mínimo.
3. **Persistencia y estimador base:** valor provisional inmediato, aprendizaje de descarga válido y protección contra datos corruptos.
4. **Perfiles automáticos aislados:** los cuatro ajustes acordados, comparación al inicio y botón `Revisar perfil`.
5. **Operación residente:** inicio con Windows, bajo impacto y aviso global configurable con 15% inicial y deduplicación.

Defer:

- **Soporte de otros dispositivos/transporte:** hasta validar exactitud y estabilidad de extremo a extremo con R5 Ultra 2.4 GHz.
- **Administración manual de perfiles:** contradice la experiencia automática elegida y añade migraciones/errores de usuario.
- **Analítica avanzada y exportación:** no mejora primero la calidad del estimador.
- **Actualizador, nube y cuentas:** son sistemas independientes sin valor para el núcleo local.
- **Detección automática de juegos:** proyectar consumo activo es suficiente para `juego continuo` en v1.

## Acceptance-Oriented Notes

- Nunca mostrar `0%` por ausencia de respuesta.
- Nunca presentar una estimación de tiempo sin estado de confianza/calibración y hora de actualización accesibles.
- Una combinación de los cuatro ajustes debe mapear siempre al mismo perfil; cambiar un ajuste debe impedir que la descarga siguiente actualice el perfil anterior.
- Un cruce de 16% a 15% con umbral 15% genera un único aviso; lecturas posteriores de 14%, 13% y 12% no generan otros hasta que el episodio se rearme.
- Tras suspensión, sueño del mouse o desconexión USB, una lectura válida debe restaurar el servicio sin reiniciar la aplicación.
- Cerrar y volver a abrir debe conservar umbral, perfiles, confianza y aprendizaje aceptado.
- La app debe seguir siendo útil si Windows oculta el icono en el área de desbordamiento o suprime banners: el estado permanece consultable al abrirla.

## Confidence and Gaps

| Area | Confidence | Reason / Roadmap implication |
|------|------------|------------------------------|
| Porcentaje y carga del R5 Ultra | HIGH | Protocolo reconstruido y cinco lecturas directas coherentes; falta medir granularidad y comportamiento durante carga/sueño. |
| Perfiles y alcance | HIGH | Decisiones explícitas de PROJECT.md. |
| Estados de bandeja/notificación | MEDIUM | Alineados con documentación oficial de Windows; el comportamiento exacto depende del framework elegido. |
| Estimaciones cotidiana/juego y confianza | MEDIUM | Valor y separación están decididos; fórmula, umbrales de calibración y redondeo necesitan investigación/pruebas de fase. |
| Patrones exactos de alertas de competidores | LOW | No se halló documentación pública fiable suficiente; no se usaron como requisito. |

Open questions for phase-specific research:

- ¿Qué granularidad y cadencia real tiene el porcentaje a lo largo de una descarga completa?
- ¿Cómo responde el HID mientras el mouse duerme, carga, se desconecta y vuelve de suspensión?
- ¿Se pueden leer de forma estable los cuatro ajustes sin abrir ni interferir con el software oficial?
- ¿Qué evidencia mínima hace que una estimación pase de `Inicial` a `Aprendiendo` y a `Confiable`?
- ¿Qué redondeo o rango reduce falsa precisión sin volver inútil la cifra?

## Sources

- [Project decisions](../PROJECT.md) — alcance, decisiones explícitas y restricciones (HIGH).
- [Validated current-battery telemetry spike](../spikes/001-current-battery-telemetry/README.md) — porcentaje, carga, interfaz HID y limitación de granularidad (HIGH).
- [Microsoft: Notifications and the Notification Area](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area) — la bandeja como superficie de estado y notificación; incluye batería como caso apropiado (MEDIUM, fuente oficial).
- [Microsoft: App notifications overview](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/) — capacidades y activación de notificaciones locales (MEDIUM, fuente oficial).
- [Microsoft Support: Notifications and Do Not Disturb in Windows](https://support.microsoft.com/en-us/windows/change-notification-and-quick-settings-in-windows-ddcbbcd4-0a02-f6e4-fe14-6766d850f294) — control del usuario, banners, centro de notificaciones y No molestar (MEDIUM, fuente oficial).
- [Logitech G HUB](https://www.logitechg.com/en-us/software/ghub) and [Razer Synapse 4](https://www.razer.com/synapse-4) — evidencia oficial de la amplitud de las suites de configuración que v1 debe evitar replicar (MEDIUM para alcance; no usadas para afirmar comportamiento de batería).

