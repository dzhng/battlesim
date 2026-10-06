# 06 — Capture, proportional score and victory

Status: not started. Dependencies: 02,04. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Simulation objective owner consumes physical eligible ground combat positions: REC/INF/VEH, squad living member or hull center within 50 m, garrisons by actual member position. Logistics/air/drones cannot capture; combat units with no ammo or suppression still can. Extra units do not speed capture. All sites start neutral; one 20-second uncontested hold captures neutral/enemy ownership.

Enemy presence pauses capture and scoring. Preserve capture progress while both sides remain; reset incomplete progress when the capturing side leaves. Defender interruption does not erase remaining attackers' progress. Ownership changes only at capture completion. Empty owned sites keep scoring. Public ownership/progress/contest observation must not publish hidden unit identity/location.

Each uncontested owned flag earns 1,000/(1,800 × (floor(N/2)+1)) points/second. Both sides score; no majority-only threshold or objective income. First authoritative 1,000 crossing wins; exact simultaneous crossing draws. All flags owned and uncontested at capture completion wins immediately. Resolve event time/finish ordering deterministically, with no browser timer or extra unit-elimination defeat. No hard 30-minute cutoff or implicit anti-stall rule.

## API seam and ownership

Admitted objective geography + authoritative physical membership → objective state/score/result. Referee owns rates and transitions; public match observation carries display values. UI and AI never rescore from card counts.

## Narrow verification

Test-first cheap no-combat position timelines: pause/resume/reset, empty scoring, garrison boundary, extra units, ineligible logistics, takeover, minority accumulation, majority 30-minute calibration, all-flags contest and exact tie. Digest/replay/native–WASM evidence for state transitions. One focused objective browser fixture for public markers/readouts.


For every visual shot produced in this slice: load game-ui and renderer as
applicable; compare candidate against its declared crop/reference and pre-change
baseline with compare-screenshots. Show the real artifact with preview-shots.
Run an **unprimed screenshot-critique as the last visual check before acceptance**.
The human checkpoint is non-blocking: leave a short (~5 minute) opportunity to
correct reversible choices while continuing independent work, decide from evidence
if silent, record rationale and close opened Preview shots. Never infer permission
for a new mechanic from silence. GPU jobs serialize under the existing harness.


## Review surface and verdict

Objective timeline probe, map objective readout and one town/intersection capture. Compare objective-site/marker/readout crops; unrelated weapons/models are out of scope. Verdict: holding territory drives victory and every displayed timer/score follows simulation state.

## Delegated freedoms and invariants

Internal type/function/file names and clean decomposition within the named owner
are delegated. Starter numerical tuning is delegated, with rationale and narrow
proof recorded in choices.md; physical/model facts require references. Cosmetic
spacing/icon fit may be adjusted reversibly under game-ui. Do not add mechanics,
change selected economy/victory/visibility rules or expand excluded capabilities.
If this slice exposes an unlisted material choice, record it in the choices ledger
and reslice the focused uncertainty rather than broadening implementation silently.

Keep existing developer fixtures, command sequencing, native/WASM agreement,
side-only knowledge and asset admission green. Verify changed contracts test-first;
non-behavior refactors preserve digest/replay outcomes. Review locally, update the
README pickup and commit the coherent pass, then proceed to the next independent
ready slice. No adversarial review or legacy compatibility scaffolding.

## Handoff evidence

- [ ] Red behavior/admission case observed where applicable.
- [ ] Narrow green proof and any allowed digest change recorded.
- [ ] Artifact/visual gates resolved where applicable.
- [ ] Choices, status and next pickup updated; focused pass committed.
