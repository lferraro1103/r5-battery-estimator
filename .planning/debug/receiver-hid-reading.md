---
status: awaiting_human_verify
trigger: "pero eso es todo lo q hace el programa , corregilo urgente"
created: 2026-09-30
updated: 2026-09-30
---

## Symptoms
- Expected: real R5 receiver battery percentage; previously worked repeatedly.
- Actual: latest physical diagnostic returns MarkerMismatch and two incorrect-function HID errors.
- Reproduction: cargo run --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --bin r5-battery-probe.
- Timeline: discovered during audit verification; onset and relation to audit not established. User confirms working prior versions.

## Current Focus
hypothesis: receiver returns A0 until its wireless mouse query has completed, while code reads only once at 120ms.
bug_class: heisenbug-mandelbug (leading candidate; state-dependent receiver response).
test: inspect captured vendor getBatPer implementation and repeat original Node probe, then compare delayed reads.
expecting: A0 transitions to A1 within a bounded delay if readiness is the cause; persistent A0 favors wireless/device state.
candidate_causes: code/single early response read; environment/mouse asleep, disconnected or receiver not ready; data/A0 is non-battery status.
and_gate: possible receiver state plus single early read; not confirmed.
next_action: parent package/deploy built release and confirm real battery with mouse awake on 2.4GHz; if receiver remains pending, investigate physical receiver state rather than widening parser.
reasoning_checkpoint:
  hypothesis: an A0 battery-command response is rejected after one read and converted to generic HID failure because the transport does not implement the official pending reread path.
  confirming_evidence:
    - five original Node runs physically returned the exact A0 battery frame; later same request returned A1, proving both states occur on the supported receiver.
    - official retrySetGet rereads responses below A1 before resending; current Rust reads once and then probes two incompatible collections.
  falsification_test: a replay A0 then validated A1 must return the A1 percentage with one send; persistent A0 must never return zero or attempt unrelated collections.
  fix_rationale: keep the same request, validated A1 parser and process ownership while rereading pending frames within a bounded window; exhausted pending becomes reading_unavailable.
  blind_spots: the physical trigger changing A0 to A1 has not been identified; no claim that a timing increase caused current physical success.
  candidate_causes:
    - code/single-shot read and generic aggregation mask pending receiver state.
    - environment/receiver or wireless mouse state determines whether A0 persists.
    - data/unsupported changed successful layout ruled out by current A1 success.
  and_gate: yes for observed failure; nonready receiver response plus single-shot handling, though precise physical state trigger is unknown.

## Eliminated
- hypothesis: Rust backend report framing differs from original successful Node exchange.
  evidence: original Node now returns the same nonvalidated marker state; request is unchanged.
  timestamp: 2026-09-30

## Evidence
- Prior spike validated prefix 00 A1 00 02 02 00 83 00 5A five times.
- Current protocol still uses the same 65-byte request and 120ms delay.
- checked: git history through initial HID commit 10e261f and current source.
  found: request, candidate filter, parser and 120ms delay are identical; recent changes only serialize worker ownership.
  implication: direct request/parser regression is not supported; compare backend behavior against Node.
- checked: process inventory and project skills/config.
  found: no R5 or Shark process in inventory; project skills absent, agent_skills empty; no debug knowledge base exists.
  implication: no known pattern to assume; no evident R5 app contention.
- checked: SBFL availability.
  found: no per-test coverage and only physical receiver failure available.
  implication: SBFL skipped; use differential raw report evidence.
- checked: physical original Node probe, then current Rust probe, sequentially.
  found: Node MI_02 writes 65 and reads 00 A0 00 02 02 00 83 00 00 (not prior 00 A1 00 02 02 00 83 00 5A); other two vendor collections reject SetFeature with incorrect function. Rust gives identical marker/error sequence.
  implication: receiver state differs independent of implementation; A0 plus zero must not be accepted as a valid battery percentage.
- checked: five sequential original Node battery probes.
  found: all five repeat exactly 00 A0 00 02 02 00 83 00 00 and two incorrect-function errors.
  implication: failure is repeatable in present receiver state; single one-shot incompatibility inference is excluded.
- checked: installed official driver app.asar getBatPer and getBatPerRetry methods.
  found: official code accepts only A1 markers, same offsets; waits 100ms; retry method delegates retrySetGet. Invalid replies return [0,0] in official software, not a validated battery.
  implication: A0 is not a successful official battery layout; preserve strict validation and examine bounded retry.
- checked: one physical MI_02 request with rereads at 120/250/500/1000/2000ms.
  found: all reads now return 00 A1 00 02 02 00 83 00 5C, 92%, not charging, starting at the first 120ms read.
  implication: real hardware success exists; delay change itself is not proven causal because first read already succeeded and user may have woken mouse.
- checked: official driver's retrySetGet implementation.
  found: status below A1 uses delayed rereads (up to 30) before resend; status above A1 resends; A1 is accepted immediately.
  implication: a non-A1 reply may be transient, but readiness recovery must be proven before calling it the root cause.
- checked: five unchanged Rust physical queries followed by original Node query after successful state appeared.
  found: Rust succeeds five times with 93%, not charging; Node returns valid A1 with 92%, not charging. No query/read changes were required for receiver success.
  implication: supported hardware works; pending recovery is a robustness defect, while exact physical state trigger remains unknown.
- checked: agent-authored replay of captured pending A0 then successful A1 before fix.
  found: test fails with Protocol(Pending) at first read, never consumes successful second frame.
  implication: regression directly demonstrates missing recovery; unchanged parser strictness is retained.
- checked: full Rust test suite after fix.
  found: 21 automated tests pass, including A0->A1, exhaustion, last allowed read, immediate legitimate zero, invalid frames/I/O, process lane timeout, native UI/history and protocol tests.
  implication: pending reread behavior and adjacent logic verified.
- checked: real receiver smoke and five fixed Rust queries.
  found: hardware returned to pending state; all six queries exhaust bounded rereads and return ReadingUnavailable (about 1.49s), without incorrect-function fallback errors.
  implication: pending handling now accurate, but bounded rereads do not resolve persistently pending physical state; do not claim actual battery restored yet.
- checked: targeted revert-and-reconfirm of reread loop, leaving test and parser intact.
  found: reducing loop to one read reproduces ReadingUnavailable failure for A0->A1 regression; restoring 12 reads passes.
  implication: bounded reread behavior is causal in replay; this does not prove persistent physical A0 was eliminated.
- checked: final optimized build and release hardware query.
  found: cargo build --release --target x86_64-pc-windows-msvc --bins passes; final release query correctly reports reading_unavailable while receiver remains pending; git diff --check passes.
  implication: release binaries ready for parent packaging; real active-mouse confirmation remains necessary.

## Resolution
root_cause: transport lacks the official pending-response reread path and aggregates A0 pending as generic HID failure with errors from unrelated collections; physical source of A0 state is not yet established.
oracle_type: derived (installed official retrySetGet contract plus captured A0/A1 frames).
fix: recognize exact A0 battery-command frames as pending; reread same handle/request up to 12 times at 120ms (1.44s settle budget); exhausted pending emits reading_unavailable, and protocol replies stop unrelated collection fallback.
files_changed: [src-tauri/src/battery/protocol.rs, src-tauri/src/battery/transport.rs, src-tauri/src/lib.rs, src-tauri/tests/protocol.rs]
verification:
  target_test: { result: pass, details: agent-authored captured A0-to-A1 replay }
  mutation_check: { result: skipped, reason_if_skipped: no Stryker configured for Rust; one-read counterfactual was independently killed by replay test }
  no_op_deletion: { result: pass, deletion_justified_by_rca: true, details: only removes unrelated fallback after an authoritative pending/protocol reply; adds actual rereads }
  adjacent_tests: { result: pass, suites_run: [Rust library, native V2, protocol integration] }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, scope: deterministic pending replay }
  guardrail_verdict: accepted for pending handling; original live-reading symptom remains unconfirmed
  physical: five original Rust successes before fix; receiver later returns persistent A0, including final fixed release query; real awake-mouse workflow still requires confirmation

## Parent integration and sleep behavior
- User confirmed configured sleep after 15 seconds idle. Pending replies must not imply a hardware fault, and sleep is a supported possible explanation, not conclusively distinguished from a wireless disconnect by A0 alone.
- Optimized fixed probe subsequently returned a valid 92%, then five consecutive valid 93% replies (not charging). Live reading is verified with the fixed code; precise cause of receiver readiness transitions remains unknown.
- Native panel preserves last valid percentage only for reading_unavailable, explicitly labels it Sin lectura nueva, and withholds current remaining hours while pending. Tooltip says último percentage, age in seconds, possible sleep and pending autonomy. A physically absent receiver still displays no current percentage.
- Cache is process-local and stores only successful actual readings; pending replies do not append history or fabricate observations. Existing 30-second scheduled polling resumes valid readings automatically; manual refresh remains available. No global mouse hook or extra wake traffic added.
- Added stale/sleep/fresh/disconnected regression. Parent verified 22 automated tests pass. Portable and installer rebuilt; personal H executable updated with matching SHA256, previous binary preserved in pre-hid-recovery backup.
- Still requires visual sleep/wake acceptance by owner; do not call every A0 a confirmed sleep state.
