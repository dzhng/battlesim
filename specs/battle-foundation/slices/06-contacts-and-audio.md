# 06 — Player observations and uncertain evidence

**Status:** complete 2026-09-25 (tracer clipping moved to 08). **Dependencies:** 03, 05. **Milestone:** Village checkpoint.

## Contract and question

Can the player react to uncertain evidence without learning hidden truth?

User requirements owned or exercised: V02, V08, V09, V10, V11, V13. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::knowledge::observe(side) -> ObservationFrame` plus event-to-SoundCue projection; presentation consumes only this frame.

## Runnable artifact

/lab/contacts: firing behind a hill, last-seen fade, hidden movement, periodic firing, and audible approach. Add actual ground fog, uncertain red areas and synthesized categorized sound from quantized cues. Camera motion alone changes neither evidence nor audibility.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Metamorphic hidden-identity/health/order/motion mutations leave identical side payload until permitted evidence. A shot at empty ground reveals an area. Repeated shots do not produce independent position samples; stale marker never follows hidden movement. Clip tracers/muzzle effects; filter changed props/picking/HUD/AI and audio. Contact expiry and reidentification retire the right side-local record.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Identified/contact/unknown distinction**. Review crop/mask: **Same forest-edge crop as 05; contact and fade endpoints; audio cue transcript**. Explicitly out of scope: Artistic fog, photoreal atmosphere and final sound assets.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Glow color/shape and synthesized timbre within information/readability constraints. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

A player can pinpoint hidden motion or confuse contact with a real unit: reject and fix before targeting.

## Verdict — 2026-09-25

Accepted. What shipped:

- **Knowledge.** `sim::knowledge` gained side-scoped approximate contacts (`ContactId`, source `firing` or `last_seen`, centre, radius, evidence tick and expiry). They carry no class, velocity, cost or exact spot.
  - **Firing.** `Battle::record_fire` is the one firing-evidence seam; slice 08's weapons and the lab `fire` event both call it. It discloses map-wide whatever the line of sight. One report per firing episode, placed by an offset drawn once from the side's observation-uncertainty RNG stream and refreshed while the shooter stays inside the area. A shot from outside the area starts a new report. An identified shooter adds none.
  - **Last seen.** Losing identification leaves an area centred on the last sighting. It never moves and expires after the fixture's 8 s. Re-identification retires every area linked to that unit; the link is internal and never exported.
- **`sim::hearing`.** Every living unit sounds by class, idle or moving; shots are their own category. Each 0.5 s bucket, the nearest friendly listener within the category's range (infantry 200 m, vehicles 650 m, shots 1000 m) turns an *unseen* enemy's sound into one `SoundCue`: listener, category, one of 8 sectors, near/far band, moving flag. There is never a position, and never a visual contact.
- **`known_props`.** The observation now carries dynamic obstacles the side has seen or met, and presentation draws them (the movement lab's tick-150 wall now appears once learned).
- **Browser.**
  - `evidenceOverlay` draws contacts as banded translucent discs that fade toward expiry: red for firing, orange for last-seen. Overlays draw with fog switched off, so the side's own evidence reads over fogged ground.
  - `present/audio` synthesizes placeholder rumble, footfalls and cracks, panned by sector relative to the camera and scaled by band. Captions carry exactly the cue's information.

Tests:

- **8 native** (`contacts.rs`): a shot from hiding shows an area containing the shooter, not the shooter; repeated shots refresh one report at the same centre; an area never follows hidden movement across 1,200 ticks and a shot from outside it starts a new one; areas expire after their lifetime; an identified shooter adds none; last-seen areas centre on the last sighting, stay fixed, and are retired by re-identification; listeners hear a hidden moving tank east, near, one cue per bucket, and a 300 m shot, but not idle infantry at 300 m; obstacles become known by sight only.
- **The slice 05 metamorphic test** now moves the hidden tank out of earshot as well, because sound is legitimate evidence.
- **TypeScript.** A new round-trip test packs a real WASM `Battle` frame and decodes every group through the published layout. It caught the packer writing contact and cue counts without their rows. Audio caption and pan tests (2).
- **Browser (7 checks).** The hidden shooter shows an area and no unit; the area is drawn red; three shots keep one report; losing the tank leaves an area at its last sighting that doesn't follow it; unseen enemies produce captions with one sound per caption; moving the camera changes no evidence. The caption transcript is written to the evidence folder.

Visual gate (identified, contact or unknown). The unprimed critique found the first contact rendering dishonest: concentric bands with a darker core read as a bullseye on an exact spot. It also found red against orange hard to tell apart over mixed ground, no legend, and repetitive captions.

- **Fixed.** A firing area is now an even translucent fill whose rim fades, with no centre to aim at. A last sighting is a hollow amber ring, distinct by shape as well as colour. The panel has a legend with swatches. Repeated captions collapse to one line with a count, and fade times show whole seconds.
- **Left as is.** Friendly units are specks at this overview zoom, and unit labels belong to slice 14. The radius is shown because it is a property of the area, not hidden information.

Moved to slice 08: clipping tracers and muzzle effects to each side's observable flight segments. There are no in-battle projectiles until weapons fire.
