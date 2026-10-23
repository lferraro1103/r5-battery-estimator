---
status: resolved
trigger: "duracion de carga completa por mas que lo estoy usando hace semanas y ya cargue el mouse 2 veces no anda y no se si realmente hace una estimacion de bateria buena tipo patron de uso"
created: 2026-10-23
updated: 2026-10-23
---

## Symptoms
- Expected: "Duración de carga completa" shows a learned value after weeks of use and two recharges.
- Actual: the card stays on "Aprendiendo · Sin ciclo completo · confianza baja"; "Autonomía restante" keeps using the 200 h placeholder.
- Evidence: local history `%LOCALAPPDATA%\R5 Battery Estimator\rust-v2-history.json` (3530 samples, 2026-10-09 19:22 → 2026-10-23 19:22).

## Evidence from the real history
- Range seen: 94% → 43%. Never ≥95%, never ≤5%, and **zero samples with `charging = true`**.
- Learning-segment breaks (rules in `learned_hours`): 83 gaps >10 min and 318 one-percent upward bounces while not charging. 402 segments; the longest lasts 0.73 h.
- Gap histogram: 2912 ≤35 s, 364 36–120 s, 170 2–10 min, 57 10–60 min, 26 >1 h. Only ~32 h of 336 wall-clock hours had a live reading.
- Observed drain: 49% in 14 days ≈ 3.5%/day → **~28 days per charge** at this usage pattern; ≈1.5% per observed active hour → **~65 active hours per charge**.
- The placeholder (`remaining_hours` → 200 h per charge) is about 3× too optimistic for active use.

## Root causes (native_v2.rs)
1. `learned_hours` only accepts one uninterrupted discharge from ≥95% to ≤5%, with no gap >600 s and no +1% bounce. With weeks per charge, nightly PC shutdowns, mouse sleep and ±1% firmware jitter, a cycle like that cannot happen in practice.
2. `normalize_history` drops samples older than 14 days. A charge lasts ~28 days, so the start of a cycle is deleted before its end is reached.
3. Learning requires reaching ≤5%. Normal users recharge at 10–30%.
4. Charging is never observed: either the recharge happens while the app or receiver can't read the mouse (by cable it may enumerate with another PID, or the receiver returns pending), or the charging byte is unvalidated. No hardware test has confirmed `charging = true` (spike 001 and receiver-hid-reading only saw "not charging"). Recharges should also be inferred from a jump in percent (for example +10% across a gap).
5. "Autonomía restante" mixes units: hours per charge × percent, without saying whether it means active hours or calendar days. The 200 h baseline doesn't come from data.

## Eliminated
- hypothesis: the history file fails to save or load.
  evidence: the file is valid, current to the last minute, and normalized.

## Resolution
- Fixed by quick task 261023-qz3 (commit a4374e6). The real history now yields 69.5 h active / 28.6 days per charge, high confidence.
- Still open: hardware validation of the charging byte.

## Proposed fix direction
- Replace full-cycle learning with rate estimation across all discharge data: sum the percent drop and time inside tolerant segments (allow gaps up to ~30 min, ignore +1/+2% jitter, split only on charging or a jump ≥5%). Weight recent data more and require a minimum total drop (e.g. ≥15%) before claiming confidence.
- Report two figures: active-use hours per charge, and calendar days per charge at the user's real pattern.
- Keep a compact aggregate (e.g. hourly buckets or per-segment summaries) beyond 14 days instead of raw samples only.
- Detect recharges by percent jumps, and record them as cycle boundaries even when `charging` is never seen.
- Validate the charging byte on hardware (cable and receiver) with `r5-battery-probe` while charging.
- Replace the 200 h baseline with the learned rate as soon as one exists.
