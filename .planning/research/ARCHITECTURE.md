# Architecture Patterns

**Project:** R5 Battery Estimator  
**Domain:** Windows local peripheral telemetry and empirical battery-runtime estimation  
**Researched:** 2026-09-28  
**Overall confidence:** MEDIUM-HIGH — the battery protocol is directly validated on the target hardware; lifecycle, storage, and shell patterns are backed by primary documentation; configuration reads and the safe polling cadence still need device-specific validation.

## Recommended Architecture

Build one per-user, single-instance background application whose authoritative state lives in the background host. The tray icon and optional detail window are presentation adapters, not separate owners of device or database state. Keep the hardware protocol behind a narrow port, append immutable observations before deriving anything, resolve a canonical consumption profile, then build learned models from explicitly valid discharge segments.

Do **not** introduce a Windows service, a second database-owning process, or a continuously running renderer in v1. A service complicates interactive-session HID access and notifications without helping this single-user product. The detail window should be created lazily and may be destroyed when closed; the tray-resident host continues sampling.

```text
Windows login / second launch / power and device events
                         |
                         v
+---------------------------------------------------------------+
| Single-instance background host                              |
|                                                               |
|  Host lifecycle ----> Acquisition coordinator                 |
|                           |                                   |
|                           v                                   |
|                    R5 device port                              |
|                           |                                   |
|                    node-hid adapter                            |
|                    + protocol codec                            |
|                           |                                   |
|                           v                                   |
|  Raw observation store -> Profile resolver -> Segment builder |
|          |                     |                  |            |
|          +---------------------+------------------+            |
|                                                v              |
|                                    Per-profile estimator       |
|                                                |              |
|                                    Read-model projector        |
|                                      /               \         |
|                              Notification policy     IPC API   |
+------------------------------------------------------|--------+
                                                       v
                                              Lazy detail window
```

The boxes above are logical components. They can live in one executable/main process. If Electron is selected, the renderer remains sandboxed/context-isolated and can access only a typed, task-specific IPC API. If a native Windows shell is selected, preserve the same boundaries.

### Component Boundaries

| Component | Responsibility | Owns | Communicates With |
|-----------|----------------|------|-------------------|
| `HostRuntime` | Acquire the single-instance lock, sequence startup/shutdown, own the tray lifetime, route a second launch to the first instance | Process lifecycle and service container | All background components |
| `WindowsLifecycleBridge` | Convert login activation, suspend/resume, session unlock, and device arrival/removal into internal events | OS subscriptions only | `HostRuntime`, `AcquisitionCoordinator` |
| `AcquisitionCoordinator` | Run one coalescing schedule, serialize HID work, apply timeouts/backoff, and publish connection state | Poll schedule, connection state machine, in-flight command | `R5DevicePort`, lifecycle bridge, observation pipeline |
| `R5DevicePort` | Stable application-facing contract: discover/open, read battery, read consumption configuration, close | No state beyond the contract | Implemented by `R5UltraHidAdapter`; faked in tests |
| `R5UltraHidAdapter` | Select VID/PID and the validated vendor interface; keep one handle; issue async feature transactions in order | HID handle and adapter-local retry state | `ProtocolCodec`, `node-hid` |
| `ProtocolCodec` | Construct exact feature-report payloads and validate/parse responses | Protocol constants and decoder versions | HID adapter only |
| `ObservationPipeline` | Timestamp, validate, persist, and emit accepted battery/config observations | Ingestion checkpoint | Repository, profile resolver, segment builder |
| `ProfileResolver` | Canonicalize the four profile-defining settings, find/create a numbered profile, and mark uncertain change windows | Active profile identity | Config snapshots, repository, acquisition coordinator |
| `SegmentBuilder` | Convert raw observations into valid, censored, or rejected discharge segments | Derivation checkpoint | Raw store, estimator |
| `Estimator` | Maintain independent learned rates per profile and produce everyday/continuous-active runtime projections with confidence | Versioned model state per profile | Valid segments, projection service |
| `Repository` | Own the SQLite connection, migrations, transactions, retention, and replay queries | All durable local state | Domain services; never the renderer directly |
| `ProjectionService` | Build immutable current-state snapshots for UI and tray | In-memory read model | Repository, estimator, UI adapters |
| `NotificationPolicy` | Detect threshold crossings, apply charging/staleness gates and deduplication | Notification re-arm/dedup state | Projection service, Windows notification adapter |
| `TrayAdapter` / `DetailWindow` | Render current snapshots and send explicit commands such as `refresh`, `reviewProfile`, and settings changes | View-local state only | Typed command/query API |

### Dependency Direction

Dependencies point inward:

```text
Windows / Electron / node-hid / SQLite
                -> adapters
                -> application services
                -> domain types and policies
```

The domain layer must not import `node-hid`, Electron, Windows APIs, SQLite, timers, or wall-clock functions. Inject `DevicePort`, `Clock`, `Scheduler`, `Repository`, `SystemIdleSource`, and `Notifier` interfaces. This makes the difficult cases—plateau telemetry, sleep gaps, disconnects, charging transitions, and profile changes—deterministically testable without the physical mouse.

## State Ownership

Separate four categories that are easy to accidentally blend:

| State category | Canonical owner | Durable? | Rule |
|----------------|-----------------|----------|------|
| Raw device observations | `Repository.raw_observations` / `config_snapshots` | Yes, immutable | Record what was observed, when, through which protocol decoder, with quality metadata. Never rewrite to match a later model. |
| Profile identity | `profiles` plus the runtime `ProfileResolver` | Yes | Identity is only polling rate, Competitive Mode, Motion Sync, and sleep time. A versioned canonical fingerprint has a uniqueness constraint. |
| Learned models | `model_snapshots`, keyed by profile and algorithm version | Yes, replaceable/rebuildable | Derived only from valid same-profile discharge segments. A profile never updates another profile's learned parameters. |
| Presentation | `ProjectionService` immutable snapshot | No, except user preferences and notification dedup state | Formatted time, labels, stale banners, confidence wording, and charts are projections, not facts. |

Runtime state machines should be explicit rather than inferred from nullable fields:

- App: `starting -> running -> suspending -> suspended -> resuming -> running`, with `degraded-storage` and `stopping` side states.
- Device: `absent -> discovering -> opening -> ready -> busy -> ready`, with `backoff` after failures.
- Power flow: `unknown -> discharging | charging`; every transition starts a new segment boundary.
- Profile: `unresolved -> resolved(profileId)`; a changed fingerprint creates/resolves a different profile and introduces an uncertainty boundary.

Only the background host mutates these machines. UI commands are requests; they do not directly set state.

## Durable Data Model

Recommended v1 tables:

| Table | Purpose | Important fields |
|-------|---------|------------------|
| `device_sessions` | Bound intervals of continuous process/device availability | start/end UTC, monotonic origin, end reason, adapter/protocol version |
| `raw_observations` | Immutable battery facts | UTC, session monotonic time, percent, charging, profile id nullable, source, latency, quality/error code |
| `config_snapshots` | Immutable reads of consumption-relevant configuration | four normalized values, observed UTC, completeness, protocol version |
| `profiles` | Stable automatic numbered identities | numeric label, fingerprint version/hash, four canonical fields, first/last seen |
| `discharge_segments` | Rebuildable training units | start/end observation, profile, elapsed awake seconds, percent delta, usage class, validity/rejection reason |
| `model_snapshots` | Rebuildable per-profile models | algorithm version, everyday rate, active rate, dispersion, evidence coverage, confidence, last input id |
| `settings` | User preferences | low-battery threshold (default 15%), startup preference, retention/diagnostic settings |
| `notification_state` | Persistent dedup/re-arm state | discharge episode, threshold, last notified UTC, armed flag |
| `diagnostic_events` | Bounded operational evidence | category, stable code, duration/count, redacted context |

Store both UTC and a monotonic duration within a process/device session. UTC supports history and UI; monotonic time prevents clock corrections from creating impossible discharge rates. Never calculate across different process sessions unless continuity is positively established; suspend and disconnect gaps are censored by default.

Use one repository-owned SQLite connection and one serialized write queue. Writes are small and infrequent, so multiple database writers add risk without value. If WAL is enabled for concurrent history reads, use a current SQLite release containing the March 2026 WAL-reset fix and treat the database, `-wal`, and `-shm` files as one unit for backup. With renderer queries routed through the host and short transactions, the default rollback journal is also sufficient for v1 and is the simpler starting point.

### Schema Evolution

- Run forward-only migrations before device services start.
- Use an application migration table and `PRAGMA user_version`; never manually write SQLite's `schema_version`.
- Execute each migration transactionally and set the application version only after success.
- Back up before destructive migrations; keep raw observation columns additive where possible.
- Store `protocol_decoder_version`, `profile_fingerprint_version`, and `model_algorithm_version` with derived state.
- When the estimator changes, rebuild `discharge_segments` and `model_snapshots` from immutable raw observations instead of rewriting history.
- On migration failure, do not poll or mutate the database. Start the tray in a clear degraded state with export/reset diagnostics; never silently create a second empty database.

## Data Flow

### 1. Cold Start and Login Start

1. Acquire the single-instance lock **before** opening SQLite or HID. A losing instance sends an activation command to the owner and exits.
2. Initialize minimal rotating logs and create the tray in `Starting…` state after the UI framework is ready. A login launch does not open the detail window.
3. Open the repository, validate the database header, run migrations, and replay any raw observations newer than the estimator checkpoint.
4. Subscribe to suspend/resume and device events.
5. After a short startup settle window, discover/open the validated R5 Ultra vendor interface.
6. Read a complete consumption configuration, canonicalize it, resolve/create the profile, then read battery state.
7. Persist the raw snapshot, update derived segments/models, publish the read model, and arm background schedules.

Ordering configuration before the first battery observation prevents the first sample from being attributed to the last profile saved on the prior Windows session. The prior active profile may inform the UI, but it is never trusted until the current configuration matches.

### 2. Scheduled Battery Observation

```text
timer fires
  -> coordinator coalesces duplicate/manual triggers
  -> one serialized feature-report transaction
  -> codec validates signature, charge flag, 0..100 range
  -> append raw observation
  -> segment builder classifies interval
  -> estimator updates only if the interval is valid
  -> publish immutable projection
  -> tray/UI refresh and notification policy evaluate
```

The validated protocol requires a feature write, a short device-processing delay, and a feature read. That entire exchange is one indivisible adapter operation. No other battery or configuration command may overlap it.

### 3. Profile Review and Automatic Change Detection

At startup, on `Review profile`, and on a low-frequency scheduled configuration check:

1. Read all four required settings through the same serialized HID lane.
2. Reject partial/malformed snapshots; retain the previous profile but mark it stale/unconfirmed.
3. Normalize units and booleans, build `v1|poll=<hz>|competitive=<0|1>|motionSync=<0|1>|sleep=<seconds>`, and hash it.
4. Resolve the unique fingerprint. If absent, allocate the next number transactionally.
5. If the resolved profile differs from the current profile, close the prior segment at its last confirmed observation, mark the interval since that point `profile-change-time-unknown`, and start a new segment only after a fresh battery observation under the new profile.

This conservative gap is essential: periodic detection establishes that a change happened, not exactly when it happened. Assigning the whole interval to either profile would contaminate learning.

### 4. Suspend, Resume, Disconnect, and Charging

| Event | Immediate action | Recovery action | Learning rule |
|-------|------------------|-----------------|---------------|
| Suspend | Cancel future timers, mark the current session boundary, enqueue only a quick durable flush, close/invalidates HID handle | None while suspended | Never attribute suspended wall time to discharge |
| Resume | Enter `resuming`; do not use the old handle | Wait briefly for USB stabilization, rediscover, read configuration, then battery | First post-resume sample is a new baseline |
| Device removal/error | Invalidate and close handle, publish `disconnected`, schedule bounded backoff | Prefer device-arrival event; use sparse enumeration fallback | Gap is censored, not a zero-rate sample |
| Device arrival | Coalesce repeated OS events | Open once, read configuration then battery | New device session/baseline |
| Charging starts | Persist transition and end discharge segment | Continue sparse monitoring | Charging samples never train discharge-rate models |
| Charging ends | Persist transition | Read configuration and establish a fresh battery baseline | New discharge episode; re-arm low-battery notification |

Windows power/device callbacks must return quickly. They should enqueue these transitions; they must not perform HID or database work inside the OS callback.

## Estimation Model Boundary

The project estimates runtime from device-reported percentage, not electrochemical state of charge. The estimator must never overwrite or relabel the reported percent.

### Inputs

- Consecutive accepted battery observations with UTC and same-session monotonic duration.
- Charging state and explicit lifecycle discontinuities.
- Resolved profile identity and its four configuration values.
- Usage class for each interval: `active`, `idle`, or `unknown`. Prefer a low-cost system-idle API; do not install mouse hooks or inspect click/movement events. Interpret `active` as an observable proxy, not proof that a game is running.
- Quality metadata: protocol validity, staleness, sample latency, gap reason, and whether the profile was confirmed at both endpoints.

### Segment Rules

A training segment is valid only when both endpoints are non-charging, in the same continuous device session, under the same confirmed profile, with monotonic time and a plausible non-increasing percentage. Firmware plateaus are expected: retain every raw sample, but accumulate plateau time until a percentage drop instead of treating each equal reading as an independent zero-consumption example.

Reject or censor intervals spanning suspend, disconnect, process downtime, unknown profile changes, charging, clock anomalies, malformed reports, percentage increases while non-charging, or implausible single-step drops. Keep a stable rejection reason so calibration can be audited.

### Outputs

For each profile and algorithm version, publish:

```typescript
type RuntimeEstimate = {
  reportedPercent: number;
  everyday: { hours: number | null; low: number | null; high: number | null };
  continuousActive: { hours: number | null; low: number | null; high: number | null };
  confidence: "initial" | "learning" | "calibrated" | "stale";
  basis: "seed" | "profile-history" | "blended";
  evidence: { dischargePoints: number; observedDropPercent: number; activeHours: number };
  asOfUtc: string;
  staleReason?: string;
};
```

- **Everyday runtime** uses the profile's robust wall-clock discharge rate over valid observed usage, including its real active/idle mix.
- **Continuous gaming runtime** uses the profile's robust active-only rate. Until sufficient active coverage exists, use a clearly labeled configuration-based seed or return a broad range; do not disguise the everyday rate as gaming evidence.
- Use robust weighted rates (for example, median/trimmed weighting followed by bounded exponential updating), because a few coarse percentage steps should not swing the forecast.
- Confidence is evidence-based: total observed percentage drop, number of independent discharge episodes, active coverage, dispersion, and recency. Calibration thresholds are policy constants that must be validated against the firmware's actual percentage granularity.
- Initial priors may be selected by configuration, but learned updates are stored and applied only to the exact profile. No learned segment from Profile 1 updates Profile 2.

The UI receives an estimate snapshot; it does not calculate countdowns independently. When input is stale, retain the last estimate with a stale badge and timestamp rather than decrementing it as if the device remained connected.

## Safe Polling and Resource Budget

`node-hid` documents enumeration/open as relatively costly operations that can slow parallel USB work. Therefore:

- Keep one validated interface handle while connected; do not enumerate or reopen on every poll.
- Use the async HID API and one serialized operation queue. Never run battery and configuration commands concurrently.
- Start conservatively at one battery transaction every **5 minutes** while connected and discharging; at most tighten to **2 minutes** below the configured warning region. Treat these as validation defaults, not proven safe limits.
- Check configuration every **15 minutes**, plus startup, post-resume/reconnect, and `Review profile`. Coalesce a configuration check with the next battery cycle where possible.
- Coalesce triggers: if a read is in flight, a manual refresh awaits that result or schedules exactly one follow-up.
- Use event-driven arrival/removal when the chosen shell exposes it. Enumeration fallback backs off approximately 15 seconds, 1 minute, 5 minutes, then 15 minutes with jitter.
- Apply a bounded operation timeout and circuit breaker. Repeated access-denied/busy errors—possibly caused by the vendor application—must not cause a tight retry loop.
- Record transaction latency and missed deadlines. A release gate must compare mouse latency and dropped-input behavior at the R5 Ultra's highest supported polling rate with the estimator off versus on.

If native HID calls prove capable of wedging the host during soak tests, move only `R5UltraHidAdapter` behind a restartable worker-thread/process boundary. Keep this as an escalation seam, not the default complexity.

## Failure Handling

| Failure | User-visible state | Internal response | Data rule |
|---------|--------------------|-------------------|-----------|
| Receiver absent | `Mouse disconnected` with last-seen time | Backoff and await arrival | No synthetic 0% observation |
| Permission/busy/vendor app conflict | `Battery temporarily unavailable` | Close handle, circuit-break, sparse retry | Preserve prior snapshot as stale |
| HID timeout | `Updating…` then unavailable if repeated | Invalidate handle after threshold; rediscover | Do not train from late response |
| Malformed signature/out-of-range percent | Diagnostic error | Reject response; reopen only after repeated failures | Store bounded diagnostic metadata, not a valid observation |
| Partial configuration read | `Profile unconfirmed` | Retry later through same queue | Do not create a profile or attribute new segments |
| SQLite busy/I/O failure | `Data storage error` | Stop acquisition writes; preserve diagnostics; retry only safely | Never continue learning in memory as if durable |
| Corrupt database | Recovery screen/tray state | Preserve file, offer export/reset path | Never overwrite automatically |
| Estimator exception | Percent still shown; estimate unavailable | Rebuild derived state from raw observations | Raw history remains authoritative |
| Renderer crash | Tray/background collection continue | Recreate window on demand | No device or model state is lost |
| Duplicate launch | Existing instance surfaces/focuses detail view | New process exits before DB/HID access | Exactly one poller/writer |

## Notification Policy

Notifications are derived side effects, not observations. Fire only on a downward threshold crossing during a confirmed non-charging, connected episode. Persist a dedup key such as `(episodeId, threshold)` so restart does not repeat the same alert. Re-arm when charging begins or the battery rises above the threshold plus a small hysteresis. Suppress warnings for stale/offline state and while charging. Notification failure must never fail ingestion.

## Observability

Keep observability local and bounded:

- Structured rotating logs with stable event codes, UTC, session id, component, duration, outcome, and redacted context.
- In-memory counters surfaced in a Diagnostics section: last successful battery/config read, p50/p95 transaction latency, consecutive failures, reconnect count, next retry, queue depth, current profile confirmation time, rejected-segment counts by reason, model version/confidence, database schema version.
- Persist only bounded diagnostic events needed across restarts. Raw HID buffers are off by default and allowed only in an explicit temporary debug mode because they are noisy and protocol-sensitive.
- A lightweight health snapshot should distinguish `device unavailable`, `profile unresolved`, `storage degraded`, and `estimate learning`; do not collapse all failures into `Unknown`.

## Patterns to Follow

### Pattern 1: Append Raw, Derive Idempotently

**What:** Persist an immutable observation first, then derive segments and model state using a `last_processed_observation_id` checkpoint.  
**When:** Every accepted battery/config snapshot.  
**Why:** A crash between ingestion and learning can be replayed, and estimator versions can be rebuilt without losing evidence.

### Pattern 2: Serialized Hardware Actor

**What:** One coordinator owns the HID handle and processes commands sequentially.  
**When:** All discovery, battery, and configuration operations.  
**Why:** The vendor protocol is request/delay/response over one interface; overlap can mismatch responses and add unnecessary USB work.

### Pattern 3: Explicit Discontinuity Events

**What:** Suspend, resume, disconnect, reconnect, charge transitions, and unknown profile changes close segments.  
**When:** Any condition invalidating elapsed-time attribution.  
**Why:** Runtime estimation is primarily a data-labeling problem; hidden gaps create confidently wrong rates.

### Pattern 4: Read Model for Presentation

**What:** Project background state into one immutable UI snapshot.  
**When:** Tray refresh, detail window query/subscription, and notification evaluation.  
**Why:** It keeps rendering/formatting out of persistence and prevents tray/window disagreement.

## Anti-Patterns to Avoid

### UI-Owned Polling

**What:** The tray and detail window each call HID or SQLite.  
**Why bad:** Duplicates traffic, breaks single ownership, and stops learning when the window closes.  
**Instead:** Both consume the host's read model.

### Enumerate/Open/Close on Every Tick

**What:** Reuse the spike's one-shot lifecycle unchanged in production.  
**Why bad:** `node-hid` states enumeration/open are costly and can slow other USB operations.  
**Instead:** Keep one handle, use OS device events, and enumerate only at lifecycle boundaries/backoff.

### Plateau Equals Zero Consumption

**What:** Treat every repeated integer percentage as a zero-rate sample.  
**Why bad:** Coarse firmware granularity biases runtime upward.  
**Instead:** Accumulate time until a real drop and estimate over multi-point segments.

### Attribute Unknown Gaps

**What:** Divide percentage change by wall time across sleep, disconnect, app downtime, or unobserved profile change.  
**Why bad:** Produces invalid rates and cross-profile contamination.  
**Instead:** Mark intervals censored and start fresh baselines.

### Model State as Source of Truth

**What:** Store only the current hours-remaining number or mutate old observations when the algorithm changes.  
**Why bad:** Calibration becomes unauditable and unrecoverable.  
**Instead:** Raw observations are canonical; model snapshots are versioned caches.

## Scalability Considerations

This is a single-user local application; scale is observation volume, not users.

| Concern | ~100 observations | ~10K observations | ~1M observations |
|---------|-------------------|-------------------|-------------------|
| Writes | Direct short transaction | Batch adjacent derived writes | Partition/archive raw history by month; keep ingestion indexed and append-only |
| History UI | Query directly | Time-bucket/downsample charts | Precomputed daily aggregates and bounded default ranges |
| Model rebuild | Full replay | Full replay acceptable | Incremental checkpoints plus background rebuild with progress |
| Retention | Keep all | Keep all raw and compact diagnostics | Retention policy for diagnostics; preserve calibration aggregates before raw archival |
| Indexing | Primary keys only | Index UTC, profile/time, and derivation checkpoint | Validate query plans; avoid per-row ORM hydration |

At the recommended cadence, 10K observations already represents weeks to months. Do not optimize for million-row scale before correctness and latency tests pass.

## Build and Dependency Order

### Strict Component Order

1. **Domain contracts and deterministic test harness** — define observations, profile fingerprints, discontinuities, segment validity, estimate outputs, fake clock/device/scheduler.
2. **Single-owner runtime and repository** — acquire the instance lock; create settings, raw/profile/derived schemas; implement migrations, transactions, replay checkpoints, and degraded-storage behavior.
3. **Validated battery adapter** — port the proven feature-report codec to the async adapter; add fixture tests for shifted/normal layouts, malformed replies, timeouts, disconnect, and access conflict.
4. **Acquisition coordinator** — long-lived handle, serialization, cadence, coalescing, timeout, backoff, and metrics. Pass the highest-polling-rate mouse-latency gate before adding more device commands.
5. **Configuration reader and profile resolver** — validate all four read commands on hardware, canonicalize identity, startup/manual/periodic reconciliation, automatic numbering, and unknown-change gap handling.
6. **Lifecycle-aware observation pipeline** — power/device events, device sessions, charging state machine, raw append, valid/censored segment derivation, and crash replay.
7. **Per-profile estimator** — seed model, robust everyday and active-only rates, confidence/calibration state, algorithm versioning, and rebuild from raw history.
8. **Presentation and notification adapters** — tray, lazy detail window, typed IPC/query API, history, profile review button, startup preference, threshold crossing/dedup.
9. **Packaging and soak hardening** — login startup, upgrades/migrations, renderer/native crashes, vendor-app coexistence, multi-day sleep/reconnect tests, bounded logs/database, and latency/resource budgets.

### Suggested Roadmap Phases

| Phase | Deliverable | Dependency rationale |
|-------|-------------|----------------------|
| 1. Runtime foundation | Single instance, local store/migrations, domain contracts, simulator, minimal tray health | Prevents duplicate pollers and gives every later phase durable/testable seams |
| 2. Low-impact battery telemetry | Async adapter, coordinator, percent/charging in tray, latency benchmark | Proves production-safe acquisition before multiplying HID commands |
| 3. Profiles and trustworthy history | Four config reads, canonical profiles, startup/manual/periodic reconciliation, sessions and discontinuities | Establishes clean labels before any learning |
| 4. Estimation and calibration | Segment builder, per-profile everyday/active models, confidence/ranges, rebuild/versioning | Depends on trustworthy profile-tagged history |
| 5. Product experience | Detailed window, history, startup toggle/default, low-battery notifications and dedup | Consumes stable projections; does not own domain state |
| 6. Reliability release gate | Suspend/resume, hotplug, charging, corruption/migration recovery, vendor conflict, long soak and gaming-latency tests | Validates the cross-component failure paths that define background-app quality |

**Phase ordering rationale:** telemetry safety precedes richer config polling; profile identity and lifecycle segmentation precede estimation; estimation precedes confidence-focused UI; startup/notification polish consumes, rather than defines, the core state. Although final startup UX is late, the single-instance lock and lifecycle seams belong in the foundation because all later storage/HID correctness depends on one owner.

## Research Flags

- **Phase 2 needs hardware research:** establish the highest safe battery cadence and timeout behavior while gaming at maximum mouse polling rate; validate long-lived handle behavior alongside the official vendor application.
- **Phase 3 needs protocol research:** the extracted official software exposes `getPollingRate`, `getMotionSync`, `getTrackingMode` (the UI's Competitive Mode), and `getSleepTime`, but these reads have not yet been validated on the connected device like battery telemetry has.
- **Phase 4 needs data research:** measure firmware percentage step size and determine evidence thresholds for `learning` versus `calibrated`; test whether system-idle time is an adequate active-use proxy for continuous gaming.
- **Phase 6 needs packaging research:** confirm the chosen installer/update technology's exact login-item path and upgrade behavior before locking startup registration.

## Sources

- [Local validated battery telemetry spike](../spikes/001-current-battery-telemetry/README.md) — direct target-hardware evidence; HIGH confidence.
- [node-hid official repository and API guidance](https://github.com/node-hid/node-hid) — async API, ordered HID operations, feature reports, and enumeration/open cost; MEDIUM confidence via verified web retrieval.
- [Electron `app` API](https://www.electronjs.org/docs/latest/api/app) — single-instance lock and Windows login-item behavior; MEDIUM confidence via verified web retrieval.
- [Electron tray guide](https://www.electronjs.org/docs/latest/tutorial/tray) — tray lifetime and keeping the host alive without a window; MEDIUM confidence via verified web retrieval.
- [Electron `powerMonitor`](https://www.electronjs.org/docs/latest/api/power-monitor/) — suspend/resume events and idle-time source; MEDIUM confidence via verified web retrieval.
- [Electron context isolation guidance](https://www.electronjs.org/docs/latest/tutorial/context-isolation) — narrow renderer bridge; MEDIUM confidence via verified web retrieval.
- [Microsoft: registering for device notification](https://learn.microsoft.com/en-us/windows/win32/devio/registering-for-device-notification) and [`RegisterDeviceNotification`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerdevicenotificationw) — arrival/removal subscription and fast callback handling; MEDIUM confidence via verified web retrieval.
- [Microsoft: `WM_POWERBROADCAST`](https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast) and [suspend/resume registration](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registersuspendresumenotification) — lifecycle sequencing; MEDIUM confidence via verified web retrieval.
- [SQLite WAL documentation](https://www.sqlite.org/wal.html), [isolation](https://www.sqlite.org/isolation.html), and [`PRAGMA user_version`](https://www.sqlite.org/pragma.html#pragma_user_version) — transaction/read behavior and schema-version ownership; MEDIUM confidence via verified web retrieval.

