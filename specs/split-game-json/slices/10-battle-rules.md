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
