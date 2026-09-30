# Choices

Target selection belongs to a mount, while firing timing belongs to each physical gun. Stable soldier IDs keep casualty and replacement state separate; a transferable special weapon retains its cycle on handoff. Finite ammunition counts actual launched rounds, not squad volleys. Simulation launches drive the existing shooter-tagged flashes, recoil, sound and tracers.

## Burst timing

The user chose three-round bursts with an independently sampled zero-to-one-second aim delay before each burst, including the first. A small startup stagger alone leaves identical magazine cycles clustered. Fresh re-aim draws use the replay-seeded RNG and the existing gun cooldown, with no squad scheduler or new mutable state. The physical shot interval remains the lower bound when the aim draw is shorter.

Infantry rifles deliberately have zero magazine reload time. Refill happens on the firing tick, so every rifle remains loaded at the publication boundary and the unit readout never flickers between soldiers' reloads. HMGs retain belt reloads. Tank cannon and missile cycles are unchanged. These are game rules, not a claim of real weapon specifications.

The regression covers repeated full magazines, partial final bursts, fresh bounded pauses for each gun across three seeds, bounded startup and exact replay digests. A separate zero-reload regression crosses repeated magazine boundaries and requires continuous loaded readiness with no pause beyond the burst aim limit. The old startup-only test could not establish sustained independence.

The final paired quick report changes flank captures from 2/3 to 3/3 and blue losses from 630 to 728; ambush captures stay 0/3, losses change from 200 to 120 and tank losses from one to zero. No compensating balance adjustment was made. Reports and timestamp evidence live under `throwaway/aim-offset/`. The earlier `throwaway/fire-cadence/` report measures the previous magazine implementation, not the final burst rules.

## Contact anchors

Contact leaders use the reported uncertainty center at its ground elevation, never a hidden unit position. Panel placement remains separate. Matched native captures at 65 m and 180 m camera distances showed the old right-edge attachment displaced by 234 and 85 pixels; the corrected endpoint was within 0.001 pixels of the projected center. A DOM regression varies contact radius and projection while requiring the endpoint to remain on the reported ground center.

Firing reports and visual memories remain separate contacts for knowledge and targeting. The observation publishes one `primaryLabel` flag per emitter: Last Seen takes priority, then the freshest report of the same source, with contact ID breaking ties. Only that report gets a panel. The private emitter identity is never exported; when Last Seen expires, Heard becomes eligible again. A regression covers coexistence, retained evidence and expiry fallback. Fresh visual review noted obscured ground under foliage and distinct endpoints for overlapping contacts; projection coordinates establish the attachment that pixels alone cannot. Minor contrast loss over red fill and occasional squad-leader overlap with soldiers remain existing presentation limits, not changes to the contact report rules.

## Validation boundaries

The damage experiment isolates one HE shell from incidental machine-gun damage. The visibility experiment requires actual hidden blasts and checks every published blue blast against blue's visibility; it does not assume every scattered impact lands behind a wall. The friendly-fire collision experiment pins unrelated gun cadence. The scar scene selects an actual exclusive crater clear of prop footprints instead of applying a crater-contrast assertion to scorch or a roof; its pixel threshold is unchanged.

Tracer world widths and pixel floors remain independently authored. The prior width pass measured 3→1 px at tactical zoom and 6→2 px close, with minor diagonal raster aliasing when enlarged. The user's [report](assets/reference/user-rifle-volley.png) is a viewport crop documenting the problem, not an approved target, and never ships. No further tracer-width changes were made for burst timing.

Temporal claims need multiple complete cycles and per-actor launch timestamps. Sampled stills establish thin readable tracers and individual flashes, not exact rendered timing or absence of every between-frame flicker. The renderer skill records this boundary, was tested with a fresh small-model scenario, and was pushed to the shared skills repository.

Review covers ownership, diff and documentation. The external Codex CLI review could not run because its configured model was rejected by the account service; it is not counted as a passing review.

Final code gates passed: 365 Rust tests and 474 web tests, formatting, lint and type checks. The zero-reload gameplay capture spans 24 seconds; fresh visual review found readable thin tracers and plausibly attached flashes, with no clear effect malfunction. Soldier contrast against grass and occasional panel-leader overlap remain minor existing limitations. Source-event timing and replay assertions, rather than still-image inference, establish burst behavior.

The final full browser suite passed, including the mixed-report capture: only preferred contact IDs have panels, and leaders match projected ground centers at both tested zooms. The benchmark recorded 71.1 FPS average; the frame-cost ledger records it as an unpaired floor check. All verification refers to the final zero-reload rifle and label-priority implementation.

The final fresh contact review found no visible duplicate callouts or label-to-label overlap. It noted minor interference where a bright fog boundary crosses text; exact center attachment and source priority are established by the projection and identity-aware regression checks, not the screenshot alone.
