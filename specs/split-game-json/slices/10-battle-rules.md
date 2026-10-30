# 10 Presentation reads the battle's rules

**Contract:** both clients present a battle from that battle's own resolved
rules. An encounter's added or overridden weapon, and its `appearance`, reach
effects and sound.

**Seam:**
- Presentation parses weapons through contract's `resolve_weapons`, which
  is idempotent on resolved rules. Raw-file, catalog-session and encounter
  rules all reach `Fielding` and sound in one shape.
- Native `fielded()` and `audio::setup` take the scenario's `Rules` from
  `sim::maps`, overrides included, not the raw file.
- The browser builds `SessionCatalog.rules` from wasm (parse plus
  `sim::admit` plus serialize) and rebuilds its sound frame for each battle.
- `presentationRules` (`compose.ts:89-93`) and its test are deleted, so
  shipped rows no longer overlay the scenario's.
- The shipped UI reads `tick_hz` and `air` from the battle's rules:
  - `captions.tsx:17`, `battleStatus.tsx:11`, `MatchResultOverlay.tsx:34`,
    `battleOverlay.ts:261,355`;
  - `ghostAloft` uses the Rust cruise height instead of its own copy.
- **The fielding check** refuses:
  - a weapon whose look doesn't exist;
  - a look missing a part its weapon's physics needs: a blast if
    `blast_radius_m > 0`, a motor if `accel_mps2`, a shot and flash if the
    weapon is fired. `helicopter_crash`'s look is only a blast.

**Run / see:** two battles in a row, the second an encounter that overrides a
weapon's `appearance`, `tick_hz` and air. Each presents its own rules, with
no stale state from the first.

**Verify:**
- Write first: a native and a wasm system test of exactly that run.
- A look that's unknown, or missing a part physics needs, is refused at
  fielding with its weapon's id.
- For shipped battles, 01's probe and the native plates are unchanged.

**Moves:** nothing for shipped battles. Overridden battles now present their
overrides, which is the fix.

**Depends on:** [08](08-looks.md).

**Delegated:**
- the lifetime plumbing;
- whether `catalog.json` keeps a `weapons` copy.

**Would change this slice:** a reader that can't get the battle's rules
without a second parse path. Fix the seam; don't keep the raw-file read
alongside it.


## Done — 2026-10-29

Both fielding bindings read weapons from the battle's rules. Native fielding
and audio take the resolved `Rules`; replay loads its encounter through
`sim::maps`, including its unit mounts. Browser sessions admit and serialize
rules through wasm, rebuild their catalog and sound per battle, and provide
those rules to the HUD. Ghost heights call the Rust cruise-height owner.

Proof: all 77 probe/shader files match `after-08-merged/SHA256SUMS`; all six
native `plate-*.png` files match `before/native/` by `cmp`. No output or identity
was re-blessed. Evidence is under
`/Users/server/dev/battlesim/throwaway/split-game-json/after-10/`.
Consecutive native and wasm battles fire their real encounter-added gun through
the publication, fielding and sound path. Fielding refusal tests cover unknown
looks and missing physics-required parts, including a crash with only its blast.
Mounted ammunition is checked for firing parts too: mounting that blast-only
crash row refuses it by name, including when it is a later ammunition choice.
The presentation crate, native replay/count/audio tests, focused web tests,
rebuilt wasm, typecheck, lint, clippy and the staged size ratchet passed.

Decisions: see the [slice 10 ledger](../decisions.md#slice-10).

Review: shape, diff and docs reviewed; a before/after native plate was inspected
and opened together in Preview. An unprimed screenshot critique remains for the
integrator: this implementation-subagent brief prohibits starting child agents.
Whole-repository closeout remains slice 16.

**Review follow-ups (closed):**

- `useCaptions` takes `session.units` and `session.rules.tick_hz` from `BattleView` (8fa8d29dd).
- Capture warm-up and graphics-test reel timing read the resolved scenario tick rate (8fa8d29dd).
- Projectile diagnostics and controls read `session.rules.weapons`, including look resolution (8fa8d29dd).
- `GameRules` is now `ResolvedRules` throughout its consumers (c70c0acd3).
- `fielded`'s doc describes the supplied rules/catalog and runtime art, with no removed map argument (this closeout).
- The slice's decisions paragraph links to the decisions ledger (this closeout).
