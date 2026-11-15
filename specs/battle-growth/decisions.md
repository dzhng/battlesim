# Decisions ledger

What was decided for [battle growth](README.md), by whom, and why. Add to it
whenever a slice settles something the plan left silent or confirms a fact the
plan only read from the code.

## From the planning walk, 2026-11-14

The player answered some of these; the agent answered others on the player's
behalf under the preferences below; the code answered the rest.

| # | Question | Decision and why | Closed by |
| --- | --- | --- | --- |
| 1 | What does "fixed" mean on screen? | Cost follows what is new. Work is spread over frames only if a measured multi-type arrival still stutters. The alternative, never a hitch whatever arrives, is a second design problem on top of reuse. | player |
| 2 | Preload the enemy faction? | Both: the incremental engine, then a background preload of the enemy faction. Preloading leaks nothing: the faction is in the battle link and drawing still follows what the side sees. Rejected: engine only (first contact still waits on a download), preload only (leaves the rebuild-everything path for every other art change). | player |
| 3 | What does memory have to prove? | Smoothness is the pass/fail gate. The engine must not raise peak memory. The preload's cost is measured with the archived whole-browser method and shown before the preload is on by default. | player |
| 4 | What is on screen while art loads? | A stand-in shape that swaps to the model. Rejected: nothing until it appears (today), a contact marker that pops or fades. | player |
| 5 | Is there a weakest machine to protect? | None named. The preload runs on every machine. | player |
| 6 | When does the preload run? | As soon as the battle is on screen, deployment included, one type at a time. | player |
| 7 | What does done look like? | The player plays a large mixed battle and never notices a type arriving, and `battle-growth` keeps it so. The armory and roster pages share the path and get faster without redesign. | player |
| 8 | How does geometry stop depending on the generation? | Each model keeps its material slots for the life of the scene; new art takes the next free ones. Vertex data never changes and no shader does. Rejected: a per-draw material offset in the shaders, which touches all twelve model pipelines and the instance record has no spare field. | agent, shown |
| 9 | What identifies unchanged art? | Its content hash from the runtime catalog, carried into Rust. Names are not enough: tooling replaces art under one name. | agent, shown |
| 10 | How is reuse proven? | By counting work in a deterministic test. A long-frame check is a loose tripwire. | agent, on the player's behalf (process recommendations accepted; numbers are tripwires) |
| 11 | What stands in for an infantry squad? | One block per soldier, where the simulation says each body is. | agent, on the player's behalf (something on screen over nothing) |
| 12 | Can a stand-in be selected and targeted? | Yes. It is the unit; only its art is missing. | agent, on the player's behalf (the complete fix) |
| 13 | How deep does the preload go? | By measurement after the engine: download-only if a one-type arrival then shows no visible wait, otherwise all the way to the GPU. | agent, on the player's behalf (memory is spendable, cost shown) |
| 14 | Backward compatibility or migrations? | Neither. Hard cutover; what is replaced is deleted. | the placeholder spec and the repository's own rule |

**The player's confirmed preferences:** the complete fix over the cheap one;
something on screen over nothing; memory may be spent for smoothness if the
cost is shown; process recommendations accepted as given.

**What the agent may decide on the player's behalf:** small taste and process
questions that follow from those preferences. A new trade-off, or a new visual
beyond the stand-in, goes back to the player.

## From the synthesis of three drafts

Three independent drafts (fewest slices, risk first, seam quality; two Claude,
one Codex) agreed on the store, content identity, shared lifetime and work
counters. Where they split:

| Split | Chosen | Why | The alternative |
| --- | --- | --- | --- |
| Where the stand-in is drawn | Rust presentation, on one reserved appearance row | The frame's picks are already Rust's, so one body box feeds both the block and the click box and they cannot differ. The battle's units are drawn there. | The browser's proxy path. It has no per-instance size, skips infantry, moves bodies through JSON model lists, and a new proxy name triggers an admission. |
| Ladder size | Twelve slices | One variable and one verdict each. Geometry reuse does not need stable texture layers, because the small materials table is rebuilt per generation, so the two are separate slices. | Four slices, with the whole engine as one; fourteen, with the allocators and the shared lifetime as slices of their own. |
| Cancelling an in-flight build | Not built | With reuse a build is as short as its new art. One candidate at a time stays. | A cancel handle through the bake. Revisit only if slice 08 measures a spotted type waiting behind a build. |
| Inflated-bytes cache for fielding | Rejected | It would hold the catalog's geometry a fourth time. Fielding keeps the small facts it derives instead. | Cache inflated files by hash in JS. |
| Texture growth | Arrays carry spare layers; each layer is written once; growth is a GPU-side copy | No CPU work for unchanged textures. The placeholder's rule "never overwrite a shared resource during preparation" is kept at the level of a layer: pending work writes only layers no committed table names. | Preallocate at the catalog's maximum, which spends memory unmeasured. |
| Preload to the GPU | Feed enemy types one at a time into the set `useBattleKinds` already owns | Each is then an ordinary one-type growth, with no second owner. | A separate preload admission path. |

## Facts read from the code, not yet exercised

Each is confirmed by the slice named, which then moves it to the section below.

| Fact | Slice |
| --- | --- |
| The art library installs twice per new type (`withUnits` then `withAppearances` in `gameAppearances.ts`) | 01 |
| The download prefetch is idle on a battle opened by link because only the menu enables it | 09 |
| Catalog admission and fielding derive the same facts from a bundle | 01 |
| A hull's size reaches presentation only for fielded types; the unit catalog has it for every type | 10 |
| An unfielded vehicle has a pick of no size today, so it cannot be clicked; an unfielded squad's soldiers can | 10 |
| An undrawn enemy still gets a HUD readout, because readouts follow picks | 10 |
| The archived WASM exhaustion predates texture sharing | closeout |

## Confirmed during implementation

Nothing yet.
