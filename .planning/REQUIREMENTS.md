# Requirements: R5 Battery Estimator

**Defined:** 2026-09-28
**Core Value:** Mostrar una estimación útil y cada vez más precisa del tiempo de batería restante del R5 Ultra, basada en su telemetría real y en el patrón de consumo correspondiente a la configuración activa.

## v1 Requirements

### Battery and device state

- [ ] **BATT-01**: El usuario puede ver el porcentaje de batería y el estado de carga que informa su R5 Ultra por receptor 2.4 GHz, junto con la hora de la última lectura válida.
- [ ] **BATT-02**: Si el mouse duerme, deja de responder o se desconecta el receptor, el usuario ve el estado correspondiente y la antigüedad de la última lectura; la app no presenta el fallo como 0%.
- [ ] **BATT-03**: Tras despertar el mouse, reconectar el receptor o reanudar Windows, el usuario vuelve a recibir lecturas válidas sin reiniciar la app.

### Consumption profiles

- [ ] **PROF-01**: La app identifica una configuración de consumo mediante exactamente cuatro valores: polling rate, Competitive Mode, Motion Sync y tiempo de reposo.
- [ ] **PROF-02**: Al iniciar, la app activa el perfil numerado que coincide con esos cuatro valores o crea uno nuevo si la combinación es inédita; una lectura incompleta no crea un perfil.
- [ ] **PROF-03**: El usuario puede pulsar `Revisar perfil` y ver si la configuración sigue igual, si se reactivó un perfil previo o si se creó uno nuevo.
- [ ] **PROF-04**: El historial y el modelo de consumo de un perfil no modifican los de otro perfil; al volver a una combinación anterior se recupera su aprendizaje.

### Runtime estimation

- [ ] **EST-01**: Desde la primera lectura válida, el usuario ve una estimación inicial claramente marcada como provisional.
- [ ] **EST-02**: El usuario ve por separado el tiempo restante para uso cotidiano y para juego continuo, con una descripción breve de lo que representa cada cifra.
- [ ] **EST-03**: La app actualiza cada estimación con episodios válidos de descarga del perfil activo y conserva el aprendizaje al cerrar o reiniciar Windows.
- [ ] **EST-04**: El usuario ve un estado explicable de calibración o confianza y una precisión visual acorde con la evidencia disponible.
- [ ] **EST-05**: La app excluye de la tasa de descarga los periodos de carga, sueño, desconexión, suspensión, lecturas inválidas y cambios de perfil de momento incierto.

### Windows experience

- [ ] **UI-01**: El usuario puede consultar rápidamente batería, estado y autonomía estimada desde la bandeja de Windows, y abrir una ventana detallada.
- [ ] **UI-02**: La ventana muestra ambas estimaciones, el perfil activo y sus cuatro ajustes, el avance de calibración y un historial reciente de eventos relevantes.
- [ ] **WIN-01**: La app inicia con Windows, permanece disponible en segundo plano al cerrar la ventana y muestra si el inicio automático está activo.
- [ ] **WIN-02**: La app evita instancias duplicadas y consulta el mouse con una cadencia que no perjudica perceptiblemente su respuesta durante el juego.
- [ ] **ALERT-01**: El usuario puede cambiar o desactivar un umbral global de aviso de batería baja, cuyo valor inicial es 15%.
- [ ] **ALERT-02**: La app envía una sola notificación al cruzar hacia abajo el umbral con una lectura reciente y permite otro aviso solo después de un nuevo episodio de carga/descarga.

### Local reliability

- [ ] **DATA-01**: Los porcentajes observados, eventos de carga, perfiles, parámetros del estimador y preferencias se guardan localmente y sobreviven al reinicio.
- [ ] **DATA-02**: Si los datos guardados están dañados o tienen una versión incompatible, la app puede arrancar, informar la recuperación y volver a calibrar sin inventar una estimación confiable.

## v2 Requirements

### Distribution and analysis

- **DIST-01**: Distribución con actualizaciones automáticas verificadas.
- **ANAL-01**: Exportación de historial o análisis detallado del consumo.
- **DEV-01**: Compatibilidad con más modelos o Bluetooth tras validar el R5 Ultra 2.4 GHz.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Modificar ajustes, macros o firmware del mouse | El producto solo observa el dispositivo; evita interferir con su configuración. |
| Administrar manualmente perfiles | Los perfiles son automáticos y numerados por decisión del usuario. |
| DPI, debounce, LOD, Angle Snap o Ripple Control como identidad de perfil | El usuario eligió únicamente los cuatro ajustes de consumo. |
| Cuentas, nube o telemetría remota | Aplicación personal y local. |
| Detección automática de CS2 u otros juegos | `Juego continuo` es una proyección de uso activo, sin vigilar procesos. |
| Salud física o capacidad nominal de la batería | El porcentaje HID no permite medir esos datos con fiabilidad. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BATT-01 | Phase 1 | Pending |
| BATT-02 | Phase 1 | Pending |
| BATT-03 | Phase 1 | Pending |
| PROF-01 | Phase 2 | Pending |
| PROF-02 | Phase 2 | Pending |
| PROF-03 | Phase 2 | Pending |
| PROF-04 | Phase 3 | Pending |
| EST-01 | Phase 1 | Pending |
| EST-02 | Phase 3 | Pending |
| EST-03 | Phase 3 | Pending |
| EST-04 | Phase 3 | Pending |
| EST-05 | Phase 3 | Pending |
| UI-01 | Phase 1 | Pending |
| UI-02 | Phase 3 | Pending |
| WIN-01 | Phase 4 | Pending |
| WIN-02 | Phase 1 | Pending |
| ALERT-01 | Phase 4 | Pending |
| ALERT-02 | Phase 4 | Pending |
| DATA-01 | Phase 4 | Pending |
| DATA-02 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 20 total
- Mapped to phases: 20
- Unmapped: 0

---
*Requirements defined: 2026-09-28*
*Last updated: 2026-09-28 after roadmap mapping*
