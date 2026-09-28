# Spike Conventions

Patterns and stack choices established across spike sessions. New spikes follow these unless the question requires otherwise.

## Stack

- No project-wide stack convention has been established yet.

## Structure

- Keep hardware probes inside the owning spike directory.

## Patterns

- Prefer read-only vendor HID queries reconstructed from the installed official driver.
- Report battery values as device-reported telemetry and distinguish them from calibrated estimates.
- Do not use Windows UI automation for this idea.

## Tools & Libraries

- `node-hid` 3.4.0 successfully enumerated and queried the R5 Ultra receiver on Windows.
