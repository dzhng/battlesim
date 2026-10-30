# One owner per game setting

**Status:** in progress: 01–14 done. **Updated:** 2026-10-29.

[`fixtures/rules.json`](../../fixtures/rules.json) owns battle rules,
[`fixtures/presentation.json`](../../fixtures/presentation.json) owns visual settings,
and [`fixtures/audio.json`](../../fixtures/audio.json) owns the mix. The
[audit](audit.md) found dead numbers, numbers with two owners, weapon-keyed
tables nobody checks, and live bugs that hid because of it. The
[decision map](map.md) settles every design question. This README is the
build plan.

## Next Agent Prompt

You are implementing this plan on PR #16 (dzhng/battlesim, branch
`t3/prioritize-spec-next-steps`, base `staging`) from the checkout at
`/Users/server/.t3/worktrees/battlesim/t3-791ccb95`. Read
[`AGENTS.md`](../../AGENTS.md), [`development.md`](../../development.md),
[`fixtures/README.md`](../../fixtures/README.md), this README, then the
[decision map](map.md) (decisions are settled: don't reopen them) and the
slice you are on. Use
[`implement-spec`](../../.agents/skills/implement-spec/SKILL.md).

**Pick up at [15](slices/15-content.md).** Slice 14 makes the sound workbench
save only `weapon_looks.<look>.sound` in presentation, retaining reviewed-generation
publication and byte-preserving no-ops. Its exact probe matches `after-13-merged/`;
workbench save/reopen evidence lives in shared `evidence-14/`. Mix authoring remains
in `audio.json`; the workbench's mix views remain read only. Slice 13 assigns the final
looks and removes all transient keys. Its per-weapon level ledger and exact
probe delta report live in shared `after-13/`; paired look sheets and firefight
WAVs live in `evidence-13/`. The folded levels use per-weapon medians in dB,
with median near/far switch distances. The heavy RPG has its own restored levels.
Slice 15 can edit look values without changing any appearance assignment or
rules hash. Carry forward the content observations in
[11](slices/11-named-fixes.md), [12](slices/12-firing-look.md), and
[13](slices/13-fold-looks.md)'s visual checkpoint. The corrected sheet in
`evidence-13/final/` is the content reference; earlier sheets exposed capture
mismatches and are diagnostic only. Shared review follow-ups through 12 remain
closed in their owning slices.

Proof baseline: slice 01's dumps in
`/Users/server/dev/battlesim/throwaway/split-game-json/before/` (native plates
re-captured from the re-recorded paris-corner run). Since 08, compare against
`after-08-merged/` (its `SHA256SUMS`): it differs from `before/` only by 08's
named light-cause renames in `effects.jsonl`. Composed shaders changed in 06 by
design; `after-08-merged/` holds the current ones. Slice 12 took 02's
recommended published `WeaponPose.round` option on the user's behalf after the question went unanswered, as the integrator directed.

Before you end a pass, tick what landed below, write what you learned into the
slice file, and rewrite this prompt to say where the next pass starts.

Warnings:

- Other sessions merge and delete checkouts; never share `target/` with
  another checkout, and hold the GPU lock
  (`lockf -k /Users/server/dev/battlesim/throwaway/gpu.lock`) for captures.
- Baselines and vectors are re-blessed only as a named change in the slice
  that owns it, never in bulk.
- Push only after `/review`; after pushing, confirm with
  `git merge-base --is-ancestor`.

### TODO

- [x] [01 Proof controls](slices/01-proof-controls.md)
- [x] [02 Spike: which round a hull fired](slices/02-hull-launch-spike.md): no candidate works without a published value; the user is asked (see 12)
- [x] [03 Spike: sound sources](slices/03-sound-sources.md)
- [x] [04 Three files, same shape](slices/04-split-files.md)
- [x] [05 Typed presentation readers](slices/05-typed-readers.md)
- [x] [06 One table of shader constants](slices/06-shader-constants.md)
- [x] [07 One admission owner](slices/07-admission.md)
- [x] [08 Looks that reproduce today](slices/08-looks.md) (moves `rules_hash`)
- [x] [09 Rules cleanups](slices/09-rules-cleanups.md) (moves `rules_hash`, `config_hash`)
- [x] [10 Presentation reads the battle's rules](slices/10-battle-rules.md)
- [x] [11 Named fixes: kick, impact size, RPG level](slices/11-named-fixes.md)
- [x] [12 The firing look follows the round](slices/12-firing-look.md)
- [x] [13 Fold to the final looks](slices/13-fold-looks.md) (moves `rules_hash`)
- [x] [14 Sound workbench keyed by look](slices/14-sound-workbench.md)
- [ ] [15 Content pass](slices/15-content.md)
- [ ] [16 Closeout](slices/16-closeout.md)

## Slice ladder

```
01 proof controls ──┬─> 04 split files ─┬─> 05 typed readers ──> 06 shader constants
                    │                   ├─> 07 admission ──────> 09 rules cleanups
                    │                   └─> 08 looks ──> 10 battle rules ──> 11 named fixes
02 hull spike ──────┼──────────────────────────────────────────────────────> 12 firing look
                    │                                                         │
03 sound sources ───┘                     13 fold looks <─────────────────────┘
                                           ├─> 14 sound workbench
                                           └─> 15 content ──> 16 closeout
```

- **Lanes that can run on separate checkouts** once 04 lands: shaders (05,
  06), rules (07, 09), looks (08, 10–13). Never share build output between
  them.
- **One integrator for stored records.** 08, 09 and 13 each rewrite stored
  records with their own one-shot script; land them one at a time and rerun
  the next script on the previous one's output.
- **What each slice may move.** 01–07, 10 and 14 move no battle digest and no
  `rules_hash`. 08 and 13 move `rules_hash` because a weapon's `appearance`
  changes. 09 moves `rules_hash` and the map generator's `config_hash`. No
  slice moves a battle digest. 11, 12, 13 and 15 move presentation output,
  each as named deltas. Any edit to sim source changes `engine_build`
  (`sim/build.rs`); that is expected, not a regression.

## Invariants: one owner per concept

| Concept                             | Owner                                                                                                                   | Deleted when it lands                                                                                                       |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Battle rules data                   | `fixtures/rules.json`, parsed only as contract `Rules`                                                                  | mixed fixture; `UncheckedRules`' skip list                                                                                  |
| Presentation data                   | `fixtures/presentation.json`, one typed struct per section with `deny_unknown_fields` (05)                              | every `game["presentation"][..]` read by hand                                                                               |
| The mix                             | `fixtures/audio.json` (`AudioMix`)                                                                                      | `presentation.audio`; `sounds.json`'s `effects` role map                                                                    |
| The sound library                   | `fixtures/sounds.json`, checked by Rust `SoundCatalog` (05)                                                             | the TS-only checks in `packages/battle-audio`; per-weapon `defaults` and impacts (08)                                       |
| What a weapon looks and sounds like | its `appearance`, naming `presentation.json` `weapon_looks.<look>`, read through one `presentation::looks` owner (08)   | every weapon-keyed table, every `extends` walk in presentation and audio, `kick_of`'s exact-id match, `"default"` fallbacks |
| Refusing bad rules                  | `sim::admit(&Rules) -> Result<(), RulesError>`, called at every entry (07)                                              | `fixtures::admit`'s own sequence, `Arsenal::new`'s `.expect`, range asserts as the only check                               |
| Refusing a bad look                 | the per-battle look reader at fielding: unknown look, parts physics needs, recordings exist, burst cadence (08, 10, 15) |                                                                                                                             |
| Catalog limits                      | `contract::HullLimits::SHIPPED` and the air-speed equivalent (09)                                                       | `hull_limits`, `air_speed_limits_kmh` in rules                                                                              |
| Helicopter routing margin           | a constant in `sim/navigation/air.rs`, 10 m (09)                                                                        | `routing_margin(rules)`                                                                                                     |
| Numbers shared by CPU and GPU       | one Rust table feeding the WGSL prelude and `records/typescript.rs` (06)                                                | duplicated literals; the fog uniform's `tile_px`/`radial_bins`                                                              |
| Numbers only the GPU reads          | named constants in the WGSL (06)                                                                                        | the 162 terrain, 23 grass and 2 glass fixture numbers (05)                                                                  |
| Rules presentation sees             | the battle's resolved `Rules`, parsed through `resolve_weapons` (10)                                                    | `presentationRules`; native `fielded()` reading the raw file                                                                |

Transitional seams, each with the slice that removes it:

- the transient looks of 08 (as many as reproduce today exactly, about 37),
  removed by 13;
- the proof probe of 01, removed by 16.

The end state reads as if designed today: files have independent readers, and no
presentation code walks `extends`, keys a table by weapon id, or falls back to
`default`.

## Standing gates

- **Prove before change.** A slice that should not change what is seen or
  heard passes 01's unrounded comparison and byte-identical native plates. The
  committed vectors alone don't prove it: sound tolerates relative error
  (`presentation/tests/sound.rs:175`) and effects round to six decimals
  (`effects.rs:39`).
- **Named deltas.** A slice that changes output lists the expected deltas
  before running, then re-blesses only those (`UPDATE_SOUND_VECTORS`,
  `BLESS_PARITY`, `UPDATE_RECORDS`), one commit per cause.
- **Visual shots.** Every slice that produces a shot runs
  [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md)
  against its before, and then
  [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check before acceptance.
- **Audible changes.** These ship as before/after WAVs from
  `bun run --cwd web scene -- sound`, plus a level ledger.
- **Human checkpoints are non-blocking.** Open the shots with
  [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md), wait about 5
  minutes, then decide on the evidence, record why in the slice, close the
  windows and go on.
- **Tests test mechanics.** Use
  [`write-tests`](../../.agents/skills/write-tests/SKILL.md). Write regression
  tests first: a derived weapon fired through a battle, an encounter-added
  weapon, a bad row through every entry. Don't write tests that pin a look's
  numbers.
- **File-size ratchet.** Over budget, may not grow:
  - `sim/src/battle.rs`
  - sim `weapons.rs`
  - `contract/src/catalog.rs`
  - `contract/src/scenario.rs`
  - `game-wasm/src/lib.rs`
  - `presentation/src/sound/frame.rs`
  - `sound/synth.rs`
  - `units/driver.rs`
  - `fog/sight.rs`

  Move code out before adding to any of them.

- **Run everything once**, at 16.

## Corrections to the map

The four drafts behind this ladder checked the map against the code. These
corrections win over [`map.md`](map.md):

1. **A hull's rounds carry no shooter.** A vehicle gun's segment publishes
   shooter -1 (`sim/src/publication/layout.rs:140`). Soldier launches match on
   member id (`effects/launches.rs:74-81`), so hulls can't be matched "the way
   soldiers are" as the map says. A mount fires at most one weapon per tick
   (`weapons.rs` shot loop; `weapons/tick.rs` ammo choice). Slice 02 decides
   the mechanism.
2. **The burst-cadence check is a fielding check, not `sim::admit`.** The
   simulation never sees looks or recordings.
3. **`mapgen` takes `sim` as a normal dependency** so its rules reader can call
   `sim::admit`. Today it has `sim` only as a dev-dependency, and `sim` doesn't
   depend on `mapgen`.
4. **Derived `range_m` must survive the rules round trip.**
   `Rules::overridden` re-reads `to_value(&self)` (`scenario.rs:805`), and wasm
   handles re-parse serialized rules. So:
   - the parser refuses a `range_m` that differs from the derived value;
   - shipped data carries none;
   - resolved rows emit it, for the TS readers (range ruler, panels, labs).
5. **A look retune does not move `rules_hash`.** Only changing which look a
   weapon names does (`replay_identity.rs:39` hashes `Rules`).
6. **Exact reproduction needs about 37 looks, not 18.** That's how many
   distinct combinations today's tables give the 49 rows, bugs included.
   Slice 13 folds them.
7. **Composed WGSL can't stay byte-identical once literals get names.**
   Deleting the dead fixture numbers leaves WGSL byte-identical (05). Naming
   is proven another way (06): a diff with each name substituted back to its
   literal text, plus byte-identical native plates.
8. **The lab seeds stay.** `authority.tsx:15`, `ballistics.tsx:34`,
   `streetScenario.ts:12`, `web/scenes/ballistics.mjs:368` and the
   `battle-lab` fixture URL are reproducibility inputs, like the Rust RNG
   seeds (`ricochet_trace.rs:74`, `flight_load.rs:41,102`). Only the lobby
   default (`battleLinks.ts:137`, `lobby/model.ts:101`, `ground.tsx:73`,
   `street.tsx:20`, `_battleLook.mjs:135`) moves to one owner.
9. **Settled facts:**
   - **Overlay and HUD colours.** The overlay target (`Rgba8Unorm`) and the
     display (`Bgra8Unorm`) both store display-encoded values, as CSS does
     (`renderer-core/src/lib.rs:50,60`, `hudTheme.ts:95`). The x-ray keeps
     its own premultiplied alpha (`models_geometry.wgsl:64`). The duplicated
     values already match, so deriving them must leave plates byte-identical.
   - **naga.** naga 30.0.1 (the pinned version) accepts a `u32` prelude
     constant in `@workgroup_size` and as an array length; an `f32` is
     refused. The browser's compiler is checked in 06.
   - **Capacity.** The shipped maximum `capacity_soldiers` is exactly 32.
   - **Over-budget files.** The ratchet list above is longer than the map's.

## Later, not this plan

- Light and heavy tank guns get different looks (a smaller explosion for the
  light gun); with looks, that's a data change.
- Weapon descriptions shown in the UI; the data stays.

## Files

- [`map.md`](map.md): the decision map (what the code says, every decision and
  why, the landmines).
- [`audit.md`](audit.md): the audit's findings with file and line.
- [`slices/`](slices/): one file per slice.
