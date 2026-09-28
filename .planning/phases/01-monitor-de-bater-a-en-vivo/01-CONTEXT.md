# Phase 1: Monitor de batería en vivo - Context

**Gathered:** 2026-09-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Esta fase entrega la lectura en vivo del porcentaje y estado de carga del Attack Shark R5 Ultra por receptor 2.4 GHz, una autonomía inicial claramente provisional y una consulta rápida desde la bandeja y una ventana. El sueño, las lecturas antiguas y las desconexiones deben representarse explícitamente y nunca como 0%.

</domain>

<decisions>
## Implementation Decisions

### Información en la bandeja
- **D-01:** El icono comunica el estado mediante colores y no incrusta el porcentaje como texto diminuto.
- **D-02:** El tooltip usa un resumen compacto con el patrón `R5 Ultra · 73% · 18 h restantes`.
- **D-03:** El clic izquierdo abre un menú pequeño junto a la bandeja, no la ventana detallada directamente.
- **D-04:** El menú contiene la acción **Abrir panel**.
- **D-05:** El menú debe admitir **Actualizar perfil** cuando esa capacidad se implemente en la fase 2; la fase 1 no debe simular una actualización de perfil inexistente.

### the agent's Discretion
- Paleta exacta de colores, iconografía, textos accesibles y comportamiento del clic derecho, siempre que el estado no dependa exclusivamente del color.
- Presentación de los estados de carga, sueño, lectura antigua y desconexión dentro de la ventana detallada.
- Formato y redondeo de la autonomía provisional, manteniendo inequívoca su condición inicial.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Producto y alcance
- `.planning/PROJECT.md` — define el valor central, las restricciones y las decisiones generales del producto.
- `.planning/REQUIREMENTS.md` — define BATT-01, BATT-02, BATT-03, EST-01, UI-01 y WIN-02 para esta fase.
- `.planning/ROADMAP.md` — fija el objetivo, límites y criterios de éxito de la fase 1.

### Investigación técnica
- `.planning/research/SUMMARY.md` — sintetiza la arquitectura, riesgos HID y tratamiento de datos inciertos.
- `.planning/research/STACK.md` — fija Tauri, Rust, hidapi y el resto de la pila tecnológica.
- `.planning/research/ARCHITECTURE.md` — define el dueño único de HID y el snapshot compartido por bandeja y ventana.
- `.planning/spikes/001-current-battery-telemetry/README.md` — documenta la lectura HID validada con el hardware real.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Probe HID validado en `.planning/spikes/001-current-battery-telemetry/probe/`: referencia reproducible para portar la transacción a Rust.

### Established Patterns
- Aún no existe código de aplicación. La investigación establece un backend Rust como dueño exclusivo de HID y una UI Tauri que consume snapshots tipados.
- La bandeja y la ventana deben presentar el mismo snapshot para evitar estados contradictorios.

### Integration Points
- Actor HID Rust para lectura y reconexión.
- Estado de dominio compartido para porcentaje, carga, frescura y autonomía provisional.
- Bandeja Tauri para icono, tooltip y menú; ventana React para el panel detallado.

</code_context>

<specifics>
## Specific Ideas

- Tooltip deseado: `R5 Ultra · 73% · 18 h restantes`.
- El menú de clic izquierdo debe mantenerse mínimo: **Abrir panel** y, desde la fase 2, **Actualizar perfil**.

</specifics>

<deferred>
## Deferred Ideas

- **Actualizar perfil** pertenece a la fase 2, donde se leerá la configuración y se resolverán perfiles automáticos numerados.

</deferred>

---

*Phase: 1-Monitor de batería en vivo*
*Context gathered: 2026-09-28*
