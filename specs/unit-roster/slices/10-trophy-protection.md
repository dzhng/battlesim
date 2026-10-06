# 10 — Swept interception, finite service and weapon-row presentation

Status: not started. Dependencies: 03,04. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Add active-protection capability/threat properties in contract; defensive systems are not fake offensive weapons with dummy ballistics or target commands. First prove the interception seam in ordinary swept flight before plumbing charges. Capacity four; three-second per-unit cooldown. Eligible anti-tank guided missiles and RPG rounds, including future top attack; bullets/tank shells/artillery bypass. Intercept an imminent hull collision at 8 m surface standoff with all-around/top coverage; passing projectiles do not spend charges. Resolve by event time then stable projectile ID, rechecking guided path/moving hull/cover before the hull.

A successful interception spends one charge and starts cooldown. Threats during cooldown hit normally. Ordinary early blast/suppression occurs at interception point; nearby infantry/props may be affected. Do not also apply the prevented direct hull hit. Charges/cooldown/source state enter digest/replay.

Supply restores one charge every ten seconds for 20 finite stock under existing recipient/source conditions, capped at four; refill never resets cooldown. Intercepting does not count as an offensive shot for service eligibility. Extend existing deterministic service order explicitly: finite offensive ammo, Trophy charge, vehicle HP, soldiers. No instant refill or stock-free APS item.

Project Trophy into the shared weapon-row UI with own charges/timer and enemy admitted equipment only. Reuse generated icon/amount/timer vocabulary; no separate armor counter or offensive fire button. Enable the corresponding variants only after capability and model bindings admit.

## API seam and ownership

Contract defensive capability + swept threat event → APS state/ordinary premature detonation; supply service → restored charge. Observation projects defensive readiness into existing weapon rows, without forcing it through an offensive firing owner.

## Narrow verification

Test-first flight spike: fast/moving collision, near pass, cover before hull, changed guidance, simultaneous threats and exact cooldown edge. Then four-charge exhaustion, ineligible bypass, no duplicate damage, collateral blast, finite supply depletion/refill, cooldown unchanged and enemy readiness masking. Native/WASM/replay plus panel rows specimen and one narrow Trophy/supply browser scene.


For every visual shot produced in this slice: load game-ui and renderer as
applicable; compare candidate against its declared crop/reference and pre-change
baseline with compare-screenshots. Show the real artifact with preview-shots.
Run an **unprimed screenshot-critique as the last visual check before acceptance**.
The human checkpoint is non-blocking: leave a short (~5 minute) opportunity to
correct reversible choices while continuing independent work, decide from evidence
if silent, record rationale and close opened Preview shots. Never infer permission
for a new mechanic from silence. GPU jobs serialize under the existing harness.


## Review surface and verdict

Physical interception fixture and Trophy weapon-row specimen (ready/cooling/empty/resupplying), then actual tank encounter. Compare the weapon-panel crop and interception moment only. Verdict: saturation works physically and panel readiness mirrors simulation state.

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
