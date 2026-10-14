# Artillery units

Artillery is part of the ordinary battle roster. Its authored weapons use the
simulation's indirect high arc, terrain and prop collision, distance-scaled
angular spread, long reach and a weapon-specific minimum range. A close target
leaves the weapon holding fire; moving the unit is still the player's choice.
The same `Attack`, `AttackMove`, ground-target and engagement-policy contracts
serve artillery and direct-fire weapons, so right-click, attack-move,
automatic acquisition, hold fire and `ReturnFireOnly` keep one meaning.

The skirmish policy procures artillery through its role data. It can fire at an
identified target or place an ordinary ground order from fresh contacts,
observed movement, firing evidence and public objectives. Those guesses are
bounded and fallible because the policy receives an `ObservationFrame`, never
the hidden battle state.

## Why this shape exists

Indirect fire was already a property of the projectile solver. Keeping
artillery on that path means a ridge, a prop, minimum range, penetration,
spread and replay identity are decided by the same owners as every other
weapon. A special artillery command or manual-fire flag would have created a
second order vocabulary for a behavior the existing commands already express.

The catalog carries five shared indirect profiles while each physical roster
row owns its body, mount, mobility, appearance, wreck and faction data. This
keeps tuning in one place without making the units aliases. Runtime appearance
loading is session-scoped and widens when a battle spawns or identifies a new
kind; the renderer filters GPU model arrays to the current session's unit art.
That matters on adapters such as SwiftShader and some Windows configurations
whose `maxTextureArrayLayers` is 256. The renderer refuses an actual overflow
before allocation instead of poisoning the device.

The command bar uses the existing Attack Ground control and adds only a
contextual inset emphasis when the selected weapon's catalog trajectory is
indirect. The ground cursor's existing spread ring remains the one generic
distance-scaled cue for every weapon. The focused lab uses proxy emitters so it
can prove flight and ridge clearance without installing the complete roster
texture set.

## Principles and invariants

- The simulation is authoritative. Presentation, the command bar and the AI
  consume observations and resolved catalog data; none reconstructs hidden
  positions or weapon state.
- `Trajectory::Indirect` is the projectile metadata; `crates/sim/src/flight/mod.rs`
  validates flight timing and `crates/sim/src/flight/solve.rs` chooses the arc
  and resolves collision. `weapons` owns minimum-range holding, target
  assessment and spread.
- Artillery rows are ordinary physical catalog units. Every indirect weapon is
  admitted by the same range, lifetime and flight validation as other weapons,
  and every unit has a real mount, sound, appearance and wreck chain.
- AI ground guesses use only public evidence and stay inside the weapon
  envelope. Contact and sighting guesses expire with their evidence; objective
  guesses remain valid only while the public objective state qualifies. All can
  miss after relocation.
- The Attack Ground command serializes the existing `Order::Attack` with
  `TargetRef::Ground`; no artillery-only protocol exists.
- Catalog, rules and replay identities change together. Old planned-card or
  pre-artillery readers are not supported.
- GPU appearance generations contain only the session's currently requested
  unit art plus the map art they draw. New kinds widen that session set without
  refetching the whole catalog; rebuilding a generation may re-upload the
  current session's art.

## Code pointers

- `crates/contract/src/ballistics.rs` defines trajectory metadata;
  `crates/sim/src/flight/mod.rs` admits feasible timing and
  `crates/sim/src/flight/solve.rs` owns high/low arcs and collision.
- `crates/sim/src/weapons.rs` owns minimum-range holding, normal target orders
  and scatter; `crates/sim/src/skirmish_ai/mod.rs` owns observation-bounded
  procurement and ground guesses.
- `fixtures/game.json`, `fixtures/units/roster/` and
  `fixtures/units/roles.json` are the authored weapon, physical-unit and role
  sources. `fixtures/catalog.json` and `assets/catalog.json` are derived
  outputs.
- `apps/battle-lab/src/gameAppearances.ts` and
  `apps/battle-lab/src/useBattleSession.ts` own session-scoped art requests;
  `packages/battle-renderer/src/models/modelTextures.ts` owns the adapter-limit
  guard and texture-array upload.
- `web/src/battle/input/commandReach.ts`,
  `web/src/battle/present/readouts.tsx` and `web/src/hud.css` own the contextual
  Attack Ground emphasis. `apps/battle-lab/src/routes/ballistics.tsx` and
  `web/scenes/artillery.mjs` own the focused lab checkpoint.
- `crates/sim/src/skirmish_ai/tests.rs`, `crates/sim/tests/flight_ballistics.rs`,
  `crates/sim/tests/skirmish.rs`, `web/scenes/panels.mjs` and
  `web/tests/unitControl.test.tsx` pin the behavioral contracts.

## Hard cutover and verification

The catalog and replay change is intentionally hard-cutover. The skirmish
integration test records artillery procurement and ordinary attacks, reloads
the replay and compares the tick digests; removing the artillery weapon row is
rejected by the existing rules identity check.

Focused native and browser checks pass, including the synthetic 2.5 km profile,
every shipped artillery row's high-arc range, minimum-range holding, catalog
admission, AI prediction cases, replay parity, command emphasis, Attack Ground
serialization and the texture-array overflow guard. The fixed-seed artillery
lab capture is retained at
[`assets/artillery-crest.png`](assets/artillery-crest.png). It was produced by
the artillery-only checkpoint in `web/scenes/ballistics.mjs` through the
shipped `/lab/artillery` route (seed `20260925`, SHA-256
`a1f058dc5503e91bd07131d0254ec2d429f62af11f9aeefa9ae455bac37c9da2`) and
records the tactical flight-bench result; no external command-bar baseline was
supplied.

The focused native tests, browser tests and artillery scene pass. The browser
scene uses proxy emitters and completes on this host with SwiftShader;
production appearance loading remains covered by the session-scoped
texture-array tests. A repository-wide run is intentionally not claimed here;
the closeout proof is the affected-surface checks above.

## Dead ends

- A manual-fire flag and artillery-specific command were rejected after the
  user chose ordinary right-click and attack-move behavior.
- A second ballistic solver, special deployment timer and a second range
  visualization were rejected because existing physical and UI owners already
  express those decisions.
- Loading every catalog appearance at page warm-up was removed after the
  Windows/SwiftShader layer limit exposed that it made an otherwise valid
  battle fail before play began.
- Installing the full roster in the flight lab was rejected; proxy emitters
  keep the physics checkpoint independent from the renderer's texture budget.
