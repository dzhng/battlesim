# Battle foundation: village first, full game preserved

Build a browser combined-arms game where reconnaissance, physical fire and positioning matter, with clear controls and less routine management. The first meaningful deliverable is a replayable village assault with infantry, tanks, an AT ambush and finite supplies. The full agreed game remains specified here, including features deliberately scheduled after that checkpoint.

**Status: implementing the village checkpoint. Slices 01–14 complete; next pickup slice 15. Updated 2026-09-25.** This spec is self-contained for a new implementation session. Routes, tests and modules named in unfinished slices are planned, not existing.

## Next Agent Prompt

You are implementing this plan in `/Users/david/dev/battlegame`. **Next pickup: [slice 15](slices/15-village-encounter.md).** Compose the village from `village.json` (its map, spawns, `defender_policy`, `variants` and `encounter`). Build a red policy that reads only its own `ObservationFrame`, the four tactical scripts over the ten seeds, and `/battle/village` plus `/replay/village` on the production readouts (`web/src/battle/present/readouts.tsx`) and `useUnitControl`. Then run slice 16. Read requirements.md, architecture.md, contracts.md, research.md and validation.md before choosing implementation details; read encounter.md and the runtime fixture [fixtures/village.json](../../fixtures/village.json) before combat work. The latest user decisions are preserved in requirements.md; spec-authored resolutions are in decisions.md; implementation choices made without the user are in [choices.md](choices.md). Do not restart the interview or infer an alternative game from WARNO.

Build through the village checkpoint, slices 01–16, in dependency order. Slices 17–22 preserve the complete agreed continuation, but **do not automatically expand the first checkpoint into the entire game**. After the village checkpoint, report its evidence and use the user's next implementation instruction to continue. Networking, campaign, deck building and finished art require later scope; no backward compatibility or data migrations are required.

Standing facts (use, don't re-derive):

- **Gates.** `bun run check` covers fmt, clippy with oxlint, tsc, and cargo plus vitest (it builds the WASM first). `bun run verify` builds the WASM and runs every browser scene. `bun run --cwd web scene -- <fixture-id>` runs one scene; `-- --list` lists them. Lab fixtures are registered once in `apps/battle-lab/src/fixtures.json`, each with exactly one `web/scenes/<id>.mjs`. The runner starts its own Vite server. Headless Chromium gets the hardware Metal adapter. Evidence regenerates into gitignored `throwaway/evidence/<id>/`.
- **Renderer.** Every TypeGPU allocation is registered and destroyed explicitly, because `root.destroy()` in 0.12.5 does not free buffers. Overlays (orders, contacts, known props) draw unfogged.
- **Geometry.** Exported from Rust with a published layout (`world_layout()`). Traversability is per triangle, and `PropKind::blocks_movement` is the one movement-obstacle rule.
- **Authority.** It runs in `web/src/battle/sim`: a worker, or in-thread "direct" for replay and parity. The main thread renders and picks the public static map through its own Rust `WorldView`. New authoritative state enters `Battle::digest`. New observation fields enter `sim::publication` with layout names, and `web/tests/observation.test.ts` round-trips a real packed frame.
- **Player input.** Commands go through `web/src/battle/input/useUnitControl`. The camera pans with the arrow keys and the screen edge (see choices.md for the WASD conflict).
- **Knowledge.** Each side plans only with obstacles it knows (`movement::SideGeometry`). Enemies reach presentation only as `identified` (side-scoped handles), `contacts` (firing and last-seen areas), `audible` cues and `known_props`. Ground fog is the per-side `ground_visibility` bitset.
- **Flight.** `sim::flight` owns the one projectile store. `prepare_launch` (solve, then spread, then launch) is the only firing path. `Body` is the collider seam. `sim::rng::Rng` (SplitMix64) is the one seeded RNG, with separate streams per purpose. `Battle::record_fire` is the one firing-evidence seam.
- **Weapons.** `sim::weapons` owns every mount: one lock, one aim and one reload. `advance` returns shots, and each unit's `Reach` (attack-move halting, attack pursuit); nothing reads display reasons for behaviour. `village.json` `mounts` are named records of ammunition kinds. Enemy tracers are clipped to seen ground in `battle::clip_to_seen`.
- **Damage.** `sim::damage::resolve` runs inside `Battle::fly` on the tick's flight events, and `Battle::consequences` applies wrecks, return-fire grants and watched deaths. A soldier is alive while `hp > 0`; `corpse` records where it fell. A unit's rounds never strike its own soldiers.
- **Guidance.** Guided rounds (`turn_deg_s`) fly gravity-free under `flight::Guidance`. `Battle::guide` renews or releases them from the launching mount's `weapons::Support`; release is permanent and fixes the point on the ground.
- **Garrisons.** `sim::garrison` owns entry and exit, perimeter slots, building HP (`Structures`) and collapse into `ruin`. Occupants fire and see only from occupied slots.
- **Supply.** `sim::supply::service` runs after fire. Recipients are served in unit order by the first ready truck that can pay. A fighting unit (`weapons::engaged`) or one under a movement order waits. `UnitSetup.condition`/`stock` author worn starts.
- **Readouts.** `web/src/battle/present/readouts.tsx` owns player readouts (rings, panel, command bar, `REASON_TEXT`). Rings anchor through `LabViewport.onFrame`'s per-frame projector to interpolated poses.
- **Deployment.** `sim::deployment` owns one tick-count progress per deploying unit. `deployment::fully_deployed(&Unit)` is the readiness predicate for slice 13's service.
- **Reuse.** The sibling source was verified at the pinned revision and is recorded in [the reuse manifest](assets/reuse-manifest.json). Nothing imports the sibling at runtime.

Active warnings: TypeGPU documentation may differ from pinned APIs; current browser/GPU throughput is not measured; garrison collision and supported guidance have intentional gameplay abstractions; permanent wrecks and unlimited speculative fire are user choices. If a slice reveals a new consequential decision, update the owning spec before broadening the patch. Do not solve performance by silently deleting physical shots or remains.

Before ending any implementation pass, update this section's status/date/exact next pickup, the global checklist, the owning slice verdict, decisions.md and evidence links. Record blockers precisely. Do not mark unrun gates passed, and do not leave stale kickoff instructions in earlier artifacts. Implementation commits, if requested by the implementing workflow, should correspond to verified slices; this planning request did not authorize implementing the game now.

### Global TODO

- [x] [01 — Pinned stack and 3D reproduction](slices/01-renderer-replication.md)
- [x] [02 — Authoritative terrain and obstacles](slices/02-world-geometry.md)
- [x] [03 — Commands, observation transport and replay](slices/03-battle-authority.md)
- [x] [04 — Routing and group intent](slices/04-ground-movement.md)
- [x] [05 — Own sensors and shared identification](slices/05-sensor-visibility.md)
- [x] [06 — Player observations and uncertain evidence](slices/06-contacts-and-audio.md)
- [x] [07 — Physical flight and collision reproduction](slices/07-projectile-flight.md)
- [x] [08 — Independent weapons and engagement policy](slices/08-weapon-control.md)
- [x] [09 — Consequences of physical fire](slices/09-damage-and-suppression.md)
- [x] [10 — Supported AT ambush](slices/10-missile-guidance.md)
- [x] [11 — Buildings as abstract fighting positions](slices/11-garrisons-and-ruins.md)
- [x] [12 — Reversible deployment progress](slices/12-deployment.md)
- [x] [13 — Recovery with finite stock](slices/13-finite-supply.md)
- [x] [14 — Concurrent readiness and ammunition UI](slices/14-weapon-readouts.md)
- [ ] [15 — Replayable combined-arms encounter](slices/15-village-encounter.md)
- [ ] [16 — Current-host scale and late-battle verdict](slices/16-longevity.md)
- [ ] [17 — Transport lifecycle](slices/17-transports.md)
- [ ] [18 — Helicopters and layered observation](slices/18-air-movement.md)
- [ ] [19 — Radar support and self-guided AA](slices/19-radar-and-aa.md)
- [ ] [20 — Off-map strike lifecycle](slices/20-jet-sorties.md)
- [ ] [21 — Persistent objectives and reinforcements](slices/21-capture-economy.md)
- [ ] [22 — Full combined-arms scale and pacing](slices/22-full-battle.md)

## Read and review map

- [Interactive roadmap](visualizations/roadmap.html): milestone/filter view and each slice's focused question.
- [User requirements](requirements.md): all 80 interview decisions, later confirmations, original role intent and rejected alternatives.
- [Architecture](architecture.md): ownership, typed seams, observation/worker boundary and assets.
- [Implementation contracts](contracts.md): geometry, knowledge, weapons, flight, garrisons, deployment/service and controls.
- [Village encounter](encounter.md) and [provisional fixture](../../fixtures/village.json): concrete first playtest and numeric starting data.
- [Verification](validation.md): behavioral/visual/performance gates and honest evidence requirements.
- [Research](research.md): inspected sibling source, primary external sources and draft synthesis.
- [Decisions and OPEN-item resolution](decisions.md): what the planner chose, why, and what measurement remains.
- [Planning verification](planning-verification.md): coverage checks, audit corrections and roadmap browser/visual evidence.
- [Coverage ledger](coverage.md): every user rule assigned to a slice, including deferred scope.
- [Original brief](assets/original-brief.txt) and [archived interview map](assets/interview-map.html): preserved source inputs, superseded as kickoff instructions.

## Ladder and scope firewall

| 01 | [Pinned stack and 3D reproduction](slices/01-renderer-replication.md) | — | Village |
| 02 | [Authoritative terrain and obstacles](slices/02-world-geometry.md) | 01 | Village |
| 03 | [Commands, observation transport and replay](slices/03-battle-authority.md) | 01, 02 | Village |
| 04 | [Routing and group intent](slices/04-ground-movement.md) | 02, 03 | Village |
| 05 | [Own sensors and shared identification](slices/05-sensor-visibility.md) | 02, 03, 04 | Village |
| 06 | [Player observations and uncertain evidence](slices/06-contacts-and-audio.md) | 03, 05 | Village |
| 07 | [Physical flight and collision reproduction](slices/07-projectile-flight.md) | 02, 03 | Village |
| 08 | [Independent weapons and engagement policy](slices/08-weapon-control.md) | 03, 04, 05, 06, 07 | Village |
| 09 | [Consequences of physical fire](slices/09-damage-and-suppression.md) | 07, 08 | Village |
| 10 | [Supported AT ambush](slices/10-missile-guidance.md) | 05, 06, 07, 08, 09 | Village |
| 11 | [Buildings as abstract fighting positions](slices/11-garrisons-and-ruins.md) | 02, 04, 08, 09 | Village |
| 12 | [Reversible deployment progress](slices/12-deployment.md) | 03, 04, 08 | Village |
| 13 | [Recovery with finite stock](slices/13-finite-supply.md) | 08, 09, 11, 12 | Village |
| 14 | [Concurrent readiness and ammunition UI](slices/14-weapon-readouts.md) | 06, 08, 10, 12, 13 | Village |
| 15 | [Replayable combined-arms encounter](slices/15-village-encounter.md) | 04, 06, 09, 10, 11, 13, 14 | Village |
| 16 | [Current-host scale and late-battle verdict](slices/16-longevity.md) | 15 | Village |
| 17 | [Transport lifecycle](slices/17-transports.md) | 04, 08, 09, 11, 12, 16 | Continuation |
| 18 | [Helicopters and layered observation](slices/18-air-movement.md) | 05, 06, 07, 08, 10, 16 | Continuation |
| 19 | [Radar support and self-guided AA](slices/19-radar-and-aa.md) | 10, 12, 18 | Continuation |
| 20 | [Off-map strike lifecycle](slices/20-jet-sorties.md) | 07, 09, 18, 19 | Continuation |
| 21 | [Persistent objectives and reinforcements](slices/21-capture-economy.md) | 04, 06, 08, 17, 20 | Continuation |
| 22 | [Full combined-arms scale and pacing](slices/22-full-battle.md) | 16, 17, 18, 19, 20, 21 | Continuation |

Slices 01–03 establish real 3D and one authority. Slices 04–06 make maneuver and scouting inspectable. Slices 07–11 establish physical combat and buildings. Slices 12–14 make recovery/readiness readable. Slices 15–16 deliver the encounter plus the scale-risk verdict. Continuation adds transports, aircraft/counters, sorties, objectives/purchasing and the full match; these requirements are preserved rather than approximated inside the first village.

The agreed eventual scale is roughly 4 × 4 km, 60–100 units per side and 45–60-minute evenly matched battles. The 1.6 km village is a test scenario, not a change to those targets. Enemy AI in the village is a bounded defensive policy using the same observations and commands; a competitive general AI is not hidden in this plan. The current machine is the initial performance target, not a browser-support guarantee.

## Standing invariants

One Rust owner for authoritative geometry and combat, one side-knowledge owner for render/audio/picking/AI/target selection, one unit policy/queue, independent weapon aim/reload, one reversible deployment value, one supply stock, one camera/projection owner, one worker tick owner. Animation and display never control rules. The code should read as a game designed this way from the beginning, with no compatibility layers or a second old-world vocabulary. Fixture data becomes one runtime owner when moved, not duplicated mutable copies.

Every visual slice explicitly runs [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) when a prior/reference exists and [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) as its last visual acceptance check. [Preview-shots](../../.agents/skills/preview-shots/SKILL.md) offers non-blocking course correction; silence permits evidence-based reversible choices, not rule changes. Code changes also receive the repo's [review](../../.agents/skills/review/SKILL.md) closeout before being called done. Reslice when a seam accumulates unrelated decisions.

## Completion

A fresh agent can start slice 01 with these files. The plan is not a claim that any game code or prototype performance exists. After all authorized slices ship, follow [close-spec](../../.agents/skills/close-spec/SKILL.md) to preserve rationale and archive the build ladder; do not close the whole full-game spec when only the village milestone has shipped.
