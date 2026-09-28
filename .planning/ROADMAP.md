# Roadmap: R5 Battery Estimator

## Overview

El MVP avanza desde una lectura HID visible y recuperable en la bandeja hacia perfiles automáticos, estimaciones que aprenden de evidencia local y, por último, operación cotidiana con inicio automático y avisos. La caracterización del protocolo, el runtime y la persistencia se integran en las capacidades que habilitan; las pruebas prolongadas de Windows y hardware se aplican en cada fase y se cierran al completar la experiencia residente.

## Phases

**Phase Numbering:** fases enteras secuenciales para el primer hito; las inserciones urgentes usan decimales.

- [ ] **Phase 1: Monitor de batería en vivo** - Consultar el R5 Ultra y mostrar un estado fresco o explícitamente antiguo con autonomía provisional desde la bandeja.
- [ ] **Phase 2: Perfiles automáticos reconocibles** - Identificar la configuración de consumo y permitir revisar o recuperar su perfil numerado.
- [ ] **Phase 3: Autonomía aprendida por perfil** - Mostrar dos proyecciones calibradas desde evidencia local, aislada y recuperable.
- [ ] **Phase 4: Uso residente y avisos** - Arrancar con Windows y avisar de batería baja según una preferencia configurable.

## Phase Details

### Phase 1: Monitor de batería en vivo
**Goal**: El usuario puede consultar en la bandeja el estado real de batería del R5 Ultra y una primera autonomía provisional, incluso después de interrupciones normales del dispositivo.
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: BATT-01, BATT-02, BATT-03, EST-01, UI-01, WIN-02
**Success Criteria** (what must be TRUE):
  1. Con el receptor 2.4 GHz conectado, el usuario ve el porcentaje y estado de carga informados por el mouse, la hora de la última lectura válida y una estimación inicial marcada como provisional en la bandeja; puede abrir una ventana para consultar el mismo estado.
  2. Si el mouse duerme, no responde o se desconecta, el usuario ve la causa y la antigüedad de la última lectura, sin que el fallo aparezca como 0%.
  3. Al despertar el mouse, reconectar el receptor o reanudar Windows, las lecturas vuelven sin reiniciar la aplicación.
  4. Si intenta abrirla de nuevo, el usuario sigue teniendo una sola instancia; la consulta periódica no perjudica perceptiblemente la respuesta del mouse durante el juego.
**Plans**: TBD
**UI hint**: yes
**Risk notes**: Caracterizar los bytes, la granularidad y la cadencia real del firmware, incluida la convivencia con `ATTACK SHARK GAMING.exe`; verificar en hardware que un solo dueño HID, los timeouts y la recuperación no añadan latencia perceptible.

### Phase 2: Perfiles automáticos reconocibles
**Goal**: El usuario puede saber qué configuración de consumo está activa y recuperar su perfil numerado al iniciar o al solicitar una revisión.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: PROF-01, PROF-02, PROF-03
**Success Criteria** (what must be TRUE):
  1. Al iniciar, el usuario ve un perfil numerado correspondiente exactamente al polling rate, Competitive Mode, Motion Sync y tiempo de reposo actuales; una lectura incompleta deja el perfil sin confirmar y no crea otro.
  2. Tras cambiar cualquiera de esos cuatro ajustes y pulsar `Revisar perfil`, el usuario ve si la configuración permanece igual, recupera un perfil previo o crea un número nuevo.
  3. Al volver a una combinación anterior, incluso tras reiniciar la aplicación, el usuario vuelve a ver el mismo número de perfil.
**Plans**: TBD
**UI hint**: yes
**Risk notes**: Los comandos de lectura de estos cuatro ajustes aún requieren validación directa en el R5 Ultra; un valor parcial o un cambio de momento desconocido debe conservar la identidad anterior como no confirmada, sin fragmentar perfiles.

### Phase 3: Autonomía aprendida por perfil
**Goal**: El usuario puede comparar autonomía cotidiana y juego continuo con estimaciones que mejoran con descargas válidas y muestran la calidad de su evidencia.
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: PROF-04, EST-02, EST-03, EST-04, EST-05, UI-02, DATA-02
**Success Criteria** (what must be TRUE):
  1. La ventana detallada muestra ambas autonomías con su significado, el perfil activo y sus cuatro ajustes, el avance de calibración y los eventos recientes relevantes.
  2. Tras descargas válidas, las proyecciones del perfil activo cambian; al cerrar o reiniciar Windows se conserva el aprendizaje, y al volver a otro perfil se recupera su propio modelo sin mezclar historiales.
  3. El usuario ve un estado explicable de calibración o confianza, y las cifras se presentan con una precisión acorde a la evidencia disponible.
  4. Al revisar eventos de carga, sueño, desconexión, suspensión, lecturas inválidas o cambios de perfil inciertos, el usuario puede comprobar que esos intervalos no alteraron la tasa de descarga aprendida.
  5. Si los datos locales están dañados o son incompatibles, la aplicación vuelve a abrir, informa la recuperación y presenta la autonomía como provisional mientras recalibra.
**Plans**: TBD
**UI hint**: yes
**Risk notes**: El porcentaje entero puede permanecer estable durante horas; no contar mesetas como consumo cero ni intervalos discontinuos como descarga. Validar el prior, los rangos y la confianza con episodios independientes y comparaciones temporales; conservar evidencia reconstruible.

### Phase 4: Uso residente y avisos
**Goal**: El usuario recibe un monitor disponible al iniciar Windows y avisos de batería baja oportunos, configurables y sin repeticiones molestas.
**Mode:** mvp
**Depends on**: Phase 3
**Requirements**: WIN-01, ALERT-01, ALERT-02, DATA-01
**Success Criteria** (what must be TRUE):
  1. Tras iniciar Windows, el usuario encuentra el monitor en segundo plano, puede cerrar la ventana sin detenerlo y puede ver si el inicio automático está activo.
  2. El usuario puede cambiar o desactivar el umbral global de batería baja, que empieza en 15%.
  3. Al cruzar el umbral hacia abajo con una lectura reciente, el usuario recibe un solo aviso; no recibe otro hasta un nuevo episodio de carga y descarga.
  4. Tras reiniciar Windows, el usuario conserva su umbral y los porcentajes, eventos de carga, perfiles y parámetros de estimación observados anteriormente en el historial y las proyecciones.
**Plans**: TBD
**UI hint**: yes
**Risk notes**: Comprobar autoinicio, bandeja y notificaciones en una instalación Windows real; completar pruebas prolongadas de suspensión, reconexión, reinicio de Explorer, recuperación de datos y carga parcial, junto con mediciones de recursos y latencia frente al monitor apagado.

## Progress

**Execution Order:** Phase 1 → Phase 2 → Phase 3 → Phase 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Monitor de batería en vivo | 0/TBD | Not started | - |
| 2. Perfiles automáticos reconocibles | 0/TBD | Not started | - |
| 3. Autonomía aprendida por perfil | 0/TBD | Not started | - |
| 4. Uso residente y avisos | 0/TBD | Not started | - |
