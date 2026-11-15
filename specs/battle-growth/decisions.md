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

**Slice 00, 2026-11-14.** One growth in `battle-growth` at the scene's own
graphics profile, from the new report:

| Count | One growth today |
| --- | --- |
| Bundles decoded in Rust | 201 |
| Vertex bytes packed | 1.13 GB |
| Model buffer bytes uploaded | 1.31 GB |
| Model texture layers written | 175 (123 MB) |
| Cards baked | 189 |
| Scenery and grass bytes uploaded | 22 MB |
| Scenery placement runs | 1 |
| Bundles encoded and sent by the page | 201 (710 MB) |
| Runtime contents inflated | 272 (1.27 GB) |
| Art library installs | 2 |

- **The art library does install twice per new type, and fielding runs
  twice:** 272 contents is two passes over the same 136. There was one
  admission for the growth. Slice 01 owns both.

Decisions made where slice 00 was silent:

| Decision | Why |
| --- | --- |
| The report's fields: bundles decoded; vertex bytes packed and buffer bytes uploaded for model geometry and the materials table; texture layers and bytes for the model texture arrays; cards baked; scenery and grass bytes; scenery placement runs; and a rebuild reason | Each engine slice needs its own number, so the kinds of work are not mixed |
| A shell hands in work it did before staging (decoding its draft, the first scenery placement) as the starting report of `Runtime::new` and `stage_assets` | Decoding happens in the library before the runtime sees the art, and the count belongs where the work is |
| The page's counts are those since the previous `art committed` | The event must say what this admission cost |
| The scene reads the event's detail from the console call's argument | It needs no test hook in the page, and `LabViewport.tsx` is over its size budget |
| The first scene publishes no `art committed` event | Nothing gates on it; the Rust test covers a first admission |
| The swap test compares the live report and the world's appearance names, not pixels | It proves a dropped, stale or failed candidate changes nothing live without a frame readback; pixel equality starts with slice 03 |

**Slice 00's probe, 2026-11-14.** The reference battle at high graphics in the
scene harness, five growths, run with
[the probe](assets/growth-probe.mjs):

| Growth | Freeze at staging | Staging to commit | Cards baked | Bundles sent | Installs |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 (staged twice) | 2.8 s, then 3.2 s | 103 s | 189 | 216 | 4 |
| 2 | 3.3 s | 66 s | 189 | 218 | 2 |
| 3 | 3.0 s | 64 s | 189 | 220 | 2 |
| 4 | 3.1 s | 62 s | 189 | 222 | 2 |
| 5 | 3.5 s | 69 s | 207 | 228 | 2 |

- **The long wait is the card bake.** After the freeze, the only work left
  before the commit is baking every card. The same 189 cards took 1.4 s in the
  small skirmish and over 60 s here, so the cost is not GPU work: each card
  waits on three GPU reads in turn, and each read waits behind the battle's
  own frames, about 0.1 s apiece. Slice 05 follows from this (see its seam).
  Inferred from what the phase contains; no timer isolated the bake.
- **A superseded build is thrown away and redone.** Growth 1 was staged twice
  six seconds apart. The first build ran to the end of its bake and was
  discarded, then the second began with its own freeze. That is the 103 s, and
  it matches the 40 s waits in the player's report. Slices 01 and 08 remove
  the doubled request; with reuse a build is short enough that a wasted one
  costs little.
- **Scenery and grass at growth:** one placement run and 24 MB uploaded, of
  1.4 GB in all. Its time on this map is not measured; slice 06 times it first.
- The player's own freezes were longer (2.9–9.9 s) on a larger window with the
  battle running in real time.

Known imprecision: the library's decode count is cumulative, so a runtime
rebuilt after a lost GPU device reports decodes it did not redo. Slice 02
reworks the library and settles the count's owner.

**Slice 10, first pass, 2026-11-14 (not merged).** Confirmed on screen by the
implementer: an identified enemy with no art still gets its HUD readout,
because readouts follow the observation; an unfielded vehicle has a click box
of no size and cannot be clicked; an unfielded squad's soldiers can. The
observation carries every seen member's position for a type with no art.

Review findings to fix before it merges:

1. It fields every type in the unit catalog. In the real game every type's art
   is in the runtime catalog, so fielding would read the whole catalog and no
   type would ever be unfielded. Facts that need no art cover every type;
   appearance sets and anything read from art cover fielded types only.
2. Its fixture deletes the art from the catalog file through a hook in
   production code, and proves the swap by reloading the page. The real case
   is a type requested after the first scene whose files have not arrived:
   hold those responses from the scene, and swap in the same session.
3. It deleted comments from `presentation.rs` to fit the size ratchet.
4. It hardcodes the browser mesh's size inside presentation. The shell that
   builds the mesh states its size in the battle input.
5. The showcase invents a soldier's body to satisfy a parameter it never uses.
6. Hull sizes have two owners after it (the effects section and the new
   table), and the new table defaults silently when missing.
7. Its "art arrives" test starts with the art already present.
