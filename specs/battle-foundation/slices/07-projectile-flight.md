# 07 — Physical flight and collision reproduction

**Status:** complete 2026-09-25. **Dependencies:** 02, 03. **Milestone:** Village checkpoint.

## Contract and question

Can fast physical rounds hit the earliest actual obstruction without tunneling?

User requirements owned or exercised: P01, P02, P03, P04, P09. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::flight::{solve_launch, advance_projectiles} -> ordered Impact/NearMiss events`; one projectile store, spatial query owner world.

## Runnable artifact

/lab/ballistics: gravity arcs, moving thin target, hill crest, low/high trajectory, crossing bodies and a fast bullet. Reproduce analytic flight/relative-motion examples before connecting live weapon control. A debug emitter belongs only to the fixture, produces the same launch input as weapons, and is not a second firing path.



## Verification and verdict

Known gravity endpoint within tolerance, unequal launch/target heights, lead on steady motion, dodge after launch, no solution for unreachable target, 900 m/s thin-target hit, moving-body collision between ticks, terrain-first impact, deterministic earliest tie, high arc opt-in, bounded lifetime/subsegments. Add 200-unit-equivalent emitter load report immediately; no silent projectile cap.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Trajectory and impact location**. Review crop/mask: **Flight corridor plus impact close-up; timing sequence**. Explicitly out of scope: Combat balance, damage effects and weapon rings.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Broad-phase choice and query internals; document reproduced oracle and bounded error. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Visible arc or collision differs from geometry, or workload already exceeds frame/tick budget: resolve before gameplay composition.

## Verdict — 2026-09-25

Accepted. `sim::flight` owns the one projectile store (`Projectiles`), `solve_launch`, `prepare_launch` (solve → spread → launch, the path weapons will use) and `advance_projectiles`, which emits ordered `Impact`/`NearMiss`/`Expired` events. Fixture reads are `contract::ballistics::{FlightRules, WeaponBallistics}` (the `physics` section and a weapon row's flight fields). `Body` (id, unit, capsule or box, pose at tick start and end) is the collider seam later slices feed from units. Firing is not wired into `Battle`; slice 08 does that and adds the store to `Battle::digest` (`Projectiles::digest` exists).

Reproduced oracle and chosen algorithm:

- **Integration:** `p + v·h + ½a·h²`, then `v + a·h`, exact under constant gravity. Chords per tick `n = ⌈√(g·dt²/8e)⌉`. The village's 9.81 m/s² at 30 Hz sags 1.4 mm per tick, so `n = 1` within the 2 cm bound. Authoring that needs more than 32 fails `FlightConfig::new`. Lifetimes are bounded by `max_unguided_lifetime_s`.
- **Collision:** each chord is swept against terrain and props through `WorldGeometry::raycast`, and against bodies in relative motion over the same interval. Capsule hits are exact. A turning box is bounded by its mid-heading box grown by the most any corner turns, and pieces that hit are halved until that growth is under 1 mm, so hits are conservative by at most 1 mm. A 16 m bucket grid of swept footprints, rebuilt each tick, narrows the body candidates. The earliest hit wins; exact ties go terrain, then props, then bodies, each by id.
- **Launch solving:** the intercept quartic `|D + W·t + A·t²|² = s²·t²` has its real roots on (0, lifetime] isolated exactly (critical points recursively, then bisection). The earliest root is the low arc and the latest the high arc. Direct weapons take only the low arc; indirect weapons try high, then low. The chosen arc is flown through the same chords against the static world, and an obstruction more than 0.5 m short of the intercept reports `Blocked`.
- **Spread:** a truncated Gaussian (±3σ) per perpendicular axis, drawn once at launch from a seeded SplitMix64 `sim::rng::Rng`. The draw becomes an aim-plane displacement, and the scattered point is solved on the intended arc.

The seam has 20 native tests (`flight_ballistics` 11, `flight_collision` 8, `flight_load` 1). Each pins an analytic result:

- **Gravity endpoint:** within the 2 cm chord error, at the analytic time and velocity.
- **Analytic elevations:** low and high elevations match the closed form to 1e-9 rad, including with unequal launch and target heights. The flown rounds hit.
- **Lead:** leading steady observed motion hits; aiming at the present position misses.
- **Dodge:** a target that reverses after launch dodges; the round stays on its launch line (P04).
- **No solution:** beyond v²/g, beyond the lifetime, and for a target outrunning the round.
- **Fast bullet, thin target:** a 900 m/s round hits a 0.3 m sliding board, and a 0.3 m capsule closing at 3 m/s, mid-chord, at the analytic face position (1e-6).
- **Crossing hull:** a hull that crosses the line only between ticks is hit at tick fraction 0.600 ± 1e-9.
- **Turning hull:** a hull reached only at its mid-tick heading is hit within 1 mm of its surface; held at either end heading, it is missed.
- **Terrain first:** the crest (terrain) and a wall (prop) are struck before a body behind them.
- **Earliest tie:** goes to the lowest id whatever the input order.
- **Allegiance:** a squadmate in the line is struck first (P09); the shooter's own capsule and unit are exempt. *Superseded by slice 09: a unit's rounds never strike its own soldiers.*
- **Near misses:** one per round per unit per tick, at the analytic surface distance.
- **Bounds:** the chord-error sag is within 1e-5 at `n = 12`; 37 chords are refused against the bound of 32; the lifetime ends at exactly tick 900; a round leaving the map ends there.
- **Spread:** sd 0.9866σ, as for the ±3σ truncated normal, with none beyond 3σ; the same seed reproduces.
- **Replay:** events and store digests are identical.

The TypeScript seam test `flightMesh.test.ts` checks that trace tubes hug the reported chords and that rejected arcs draw translucent.

**Load report** (`cargo test -p sim --release --test flight_load -- --nocapture`, Apple M5 Pro, native). The emitters are 100 eight-soldier rifle squads and 100 turning tanks (900 bodies) on the village map, firing through `prepare_launch` at authored cycle rates, with no projectile cap:

| Fire rate | Launched | In flight (peak) | Advance ms/tick p50 / p95 / p99 / max | Launch solving ms/tick p50 / p95 |
|---|---|---|---|---|
| ×1 | 2,497 rounds/s | 1,247 | 0.21 / 0.24 / 0.24 / 0.26 | 0.38 / 0.45 |
| ×2 | 4,763 rounds/s | 2,503 | 0.42 / 0.46 / 0.48 / 0.48 | 0.76 / 0.83 |
| ×3 | 6,832 rounds/s | 3,693 | 0.53 / 0.67 / 0.68 / 0.68 | 1.18 / 1.30 |

Every launch ended in an impact or expiry, or was still in flight. Roughly a third of fire decisions were refused as blocked by the ridge or forest trunks (that is solving, not culling). The figures are native. WASM cost is unmeasured until slice 16 runs the authority in the browser.

**Browser scene** (`bun run --cwd web scene -- ballistics`, 13 checks), on the geometry-lab map:

- Grenade arcs at 60, 120 and 180 m land within 2 cm of their aim points.
- The HMG hits the sliding board's face at tick 5, mid-chord.
- The walker is hit; the dodger's round lands on its launch line.
- Direct fire over the crest reports blocked at the crest; a lab-only indirect row (45 m/s, since no village weapon is indirect) lands its high arc within 0.4 mm of the aim point.
- A tank crossing late takes the round, while a soldier crossing early logs near misses.
- Rendered marks sit at the reported points.
- A salvo with weapon spread replays identically from its seed.

The other scenes (foundation, geometry, authority) stay green. `bun run check` is green: 39 cargo tests and 29 vitest.

**Visual gate:**

- **Diagnostics:** single-image compare-screenshots diagnostics (no predecessor) ran on 11 frames and 6 crops. All frames had content. The one flat-looking crop (crossing, contrast 8) is ground-dominated; its trace, marks and hull read clearly.
- **Unprimed critique, acted on:**
  - The rejected arc's obstruction mark was the same red as impacts, so it now draws near-black.
  - The legend contradicted the yellow ground-hit marks, so it was rewritten as three lines naming each colour.
  - White meant both "in flight" and "aim plate", so aim plates are now navy.
  - The tank's impact mark was buried in the hull, so marks now sit outside the surface along the reported normal.
  - Overview traces were cut at the frame edge, so the camera was widened.
- **Unprimed critique, kept with reason:**
  - Low arcs look nearly straight: they are true scale (a 6 m apex over 180 m), and the side views and crops show the curvature.
  - Struck bodies drift away from their marks by the end: flight does not stop bodies, and damage is slice 09.
  - Overview bodies and near-miss clusters are tiny: the close-up and crossing views carry them.
  - The panel covers the top-left, as in every lab route.
  - The arcs side view is mostly empty below the map edge: it is viewed from off-map so no other trace stands in front.
  - Tube shading varies with orientation (one lit pipeline), and translucent joints overlap.
  - There are no on-screen per-round labels, beyond the panel list.
- **Preview-shots** was not offered: this run's orchestrator instructions forbade opening Preview windows.

**Deviations:**

- `NoSolution::Blocked` carries the blocked arc so the lab can draw it.
- The store adds an `Expired` event (lifetime, or left the closed map), so no round vanishes silently.
- Near misses exclude the shooter's own unit and the struck body.
- The contract names `FlightRules`, `WeaponBallistics` and `Trajectory` are new. `trajectory` defaults to `direct` per P03, and no village row sets it yet.
