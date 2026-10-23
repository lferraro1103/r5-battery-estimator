---
quick_id: 261023-rmb
slug: ajustes-de-espaciado-y-textos-del-panel
mode: quick
created: 2026-10-23
depends_on: 261023-rbh
---

Correcciones de la revisión visual del usuario sobre 261023-rbh.

## Task 1 — Espaciado
- action: el gráfico vuelve a y=530 (posición original). El anillo terminaba en y≈473 y el gráfico empezaba en 480. Las tarjetas no cambian.

## Task 2 — Textos
- action:
  - Título "R5 Ultra" y subtítulo "Autonomía estimada".
  - "A tu ritmo" → "Duración en días" / "Días restantes".
  - El estado del anillo "Último valor conocido" desbordaba el arco; pasa a "En reposo".
- verify: `--render-png` despierto y `--asleep`; `cargo test`; release; reinstalación.
