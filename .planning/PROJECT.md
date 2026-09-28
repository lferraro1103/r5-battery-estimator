# R5 Battery Estimator

## What This Is

Una aplicación local para Windows que lee la batería informada por el Attack Shark R5 Ultra y la convierte en estimaciones comprensibles de autonomía. Funciona desde la bandeja del sistema y ofrece una ventana detallada con porcentaje, carga, consumo aprendido, perfiles automáticos y tiempo restante tanto para uso cotidiano como para juego continuo.

Está pensada inicialmente para el dueño de un R5 Ultra conectado por receptor 2.4 GHz. Aprende de la descarga real del mouse para reemplazar progresivamente la estimación inicial por datos personalizados.

## Core Value

Mostrar una estimación útil y cada vez más precisa del tiempo de batería restante del R5 Ultra, basada en su telemetría real y en el patrón de consumo correspondiente a la configuración activa.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Leer directamente el porcentaje y el estado de carga informados por el R5 Ultra mediante su receptor HID 2.4 GHz.
- [ ] Mostrar porcentaje y tiempo estimado desde un icono en la bandeja de Windows.
- [ ] Ofrecer una ventana detallada con autonomía, consumo, calibración e historial relevante.
- [ ] Calcular por separado autonomía de uso cotidiano y horas de juego continuo.
- [ ] Comenzar con una estimación razonable y corregirla automáticamente al observar descargas y cargas reales.
- [ ] Iniciarse automáticamente con Windows y continuar recolectando datos en segundo plano.
- [ ] Emitir avisos de batería baja mediante un umbral configurable cuyo valor inicial sea 15%.
- [ ] Identificar perfiles de consumo usando polling rate, Competitive Mode, Motion Sync y tiempo de reposo.
- [ ] Al iniciar Windows, comparar la configuración actual con el perfil activo y continuar en él si coincide.
- [ ] Crear automáticamente un perfil numerado nuevo cuando cambie algún ajuste que afecte el consumo.
- [ ] Incluir un botón `Revisar perfil` que ejecute manualmente la misma comparación.
- [ ] Aprender y estimar el consumo de cada perfil de manera independiente.

### Out of Scope

- Compatibilidad con otros modelos de mouse en la primera versión — primero se validará el flujo completo con el R5 Ultra.
- Modificar la configuración del mouse — la aplicación solo la consulta para estimar consumo.
- Usar DPI, debounce, LOD, Angle Snap o Ripple Control para distinguir perfiles — el usuario eligió limitar los perfiles a los ajustes de consumo principales.
- Renombrar, eliminar o administrar perfiles manualmente — los perfiles serán automáticos y numerados.
- Sincronización en la nube o cuentas de usuario — los datos permanecerán locales.

## Context

- El software instalado correcto es `C:\ATTACK SHARK GAMING\ATTACK SHARK GAMING.exe`, versión 1.0.2.
- La investigación previa está documentada en `.planning/spikes/001-current-battery-telemetry/`.
- El receptor conectado se identifica como `R5 Ultra Mouse 2.4G`, VID `0x373E`, PID `0x0047`.
- Se reconstruyó el comando HID de lectura de batería usado por el software oficial.
- La prueba directa obtuvo 90% y estado no cargando en cinco lecturas consecutivas, sin controlar la interfaz de Windows.
- La telemetría del firmware puede tener granularidad limitada; la aplicación debe distinguir el porcentaje informado de la estimación temporal aprendida.
- Cambios en polling rate, Competitive Mode, Motion Sync y reposo pueden alterar el consumo y no deben contaminar el aprendizaje de otro perfil.

## Constraints

- **Compatibility**: Windows y Attack Shark R5 Ultra por receptor 2.4 GHz en v1 — es el hardware disponible y validado.
- **Privacy**: Datos de uso y calibración almacenados localmente — no hay necesidad de transmitir telemetría.
- **Accuracy**: La interfaz debe comunicar nivel de confianza y estado de calibración — una cifra temporal sin suficiente historial no debe parecer exacta.
- **Background operation**: Debe consumir pocos recursos y no interferir con la latencia del mouse — la lectura periódica no puede degradar el juego.
- **Profiles**: Solo polling rate, Competitive Mode, Motion Sync y tiempo de reposo definen identidad de perfil — decisión explícita del usuario.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Bandeja de Windows y ventana detallada | Consulta rápida más acceso a información de calibración e historial | — Pending |
| Estimación inicial seguida de aprendizaje | Ofrece valor inmediato sin esperar varios ciclos completos | — Pending |
| Mostrar uso cotidiano y juego continuo | Responde a dos preguntas distintas sobre autonomía | — Pending |
| Inicio automático con Windows | Permite observar suficiente tiempo de uso y reposo para calibrar | — Pending |
| Avisos configurables con 15% inicial | Evita un umbral rígido y ofrece un valor útil desde el inicio | — Pending |
| Perfiles automáticos numerados | Separa consumos sin imponer administración manual | — Pending |
| Comparar perfiles al inicio y con un botón | Cubre detección automática al arrancar y revisión bajo demanda | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition**:
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone**:
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-09-28 after initialization*
