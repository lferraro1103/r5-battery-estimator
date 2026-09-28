# Spike Manifest

## Ideas

### r5-ultra-battery
Investigar cómo el software ATTACK SHARK GAMING obtiene el nivel actual de batería del Attack Shark R5 Ultra y determinar si puede consultarse directamente sin automatizar la interfaz de Windows.

**Requirements:**

- La investigación debe realizarse sin Git ni commits.
- La primera prioridad es obtener o descartar un porcentaje actual real.
- No se debe controlar la interfaz de Windows.

## Spikes

| # | Idea | Name | Type | Validates | Verdict | Tags |
|---|------|------|------|-----------|---------|------|
| 001 | r5-ultra-battery | current-battery-telemetry | standard | Given the connected R5 Ultra, when the correct driver protocol is inspected and queried, then its current battery percentage can be obtained or conclusively classified as unavailable | VALIDATED | battery, hid, attack-shark, r5-ultra |
