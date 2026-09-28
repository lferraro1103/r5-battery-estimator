---
spike: 001
idea: r5-ultra-battery
name: current-battery-telemetry
type: standard
validates: "Given the connected R5 Ultra, when the correct driver protocol is inspected and queried, then its current battery percentage can be obtained or conclusively classified as unavailable"
verdict: VALIDATED
related: []
tags: [battery, hid, attack-shark, r5-ultra]
---

# Spike 001: Current Battery Telemetry

## What This Validates

Given the connected Attack Shark R5 Ultra, when the installed ATTACK SHARK GAMING driver and its device protocol are inspected and queried, then the current battery percentage can be obtained without controlling the Windows UI, or the absence of usable telemetry can be demonstrated.

## Research

The official Attack Shark support site identifies the R5 Ultra and distributes its software. The installed `ATTACK SHARK GAMING` 1.0.2 package was extracted locally and its WebHID implementation inspected.

| Approach | Tool/Library | Pros | Cons | Status |
|----------|--------------|------|------|--------|
| Windows battery API | WMI/PnP | No device writes | R5 Ultra exposes no standard battery object | Rejected |
| Driver UI | ATTACK SHARK GAMING | Already displays the value | Requires opening/controlling the UI | Not used |
| Direct feature report | `node-hid` | Read-only, scriptable, returns the same device value | Vendor-specific protocol | Chosen |

The driver's `getBatPer()` method sends a 64-byte WebHID feature payload with byte 2 = `0x02`, byte 3 = `0x02`, and byte 5 = `0x83`. A successful shifted-layout response contains `0xA1` at byte 1, `0x02` at byte 4, `0x83` at byte 6, charging state at byte 7, and percentage at byte 8.

## How to Run

```powershell
cd .planning\spikes\001-current-battery-telemetry\probe
npm install
node read-battery.js
```

## What to Expect

A JSON result naming `R5 Ultra Mouse 2.4G`, with `charging`, `percent`, protocol layout, and a record of attempted HID interfaces.

## Investigation Trail

- Identified the correct installed application as `C:\ATTACK SHARK GAMING\ATTACK SHARK GAMING.exe`.
- Confirmed that its application bundle is `C:\ATTACK SHARK GAMING\resources\app.asar`.
- Extracted application version 1.0.2 and found the home-card polling path: `hidHelper.getBatPer()`.
- Confirmed that the UI renders the returned number as `batPer + "%"`; it is not merely an estimated icon range.
- Reconstructed the read-only feature-report request from the driver's implementation.
- Enumerated the connected receiver as ATTACK SHARK `R5 Ultra Mouse 2.4G`, VID `0x373E`, PID `0x0047`.
- Vendor interfaces on HID interface 1 rejected feature writes as expected; interface 2 accepted the 65-byte report including report ID.
- Parsed response prefix `00 A1 00 02 02 00 83 00 5A`, where `0x5A` is decimal 90.
- Repeated the complete query five times; all five returned 90% and not charging.

## Results

**VALIDATED.** The connected R5 Ultra currently reports **90% battery** and **not charging**. This is the percentage emitted by the mouse/receiver firmware and consumed by the official software, not a time-based estimate.

The value is directly readable without Windows UI automation. Accuracy below the firmware's reporting granularity is not yet established; the current result should be described as “device-reported 90%,” not laboratory state-of-charge measurement.
