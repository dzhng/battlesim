# battlegame

A browser real-time tactics game, built around reconnaissance, physical fire and positioning. Every round is a flown projectile, and every side sees only what its own units can see. Players fight on fresh seeded generated maps with the faction roster. The full design lives in [`specs/`](specs/).

Players know the game by a public name that may change at any time, owned by [`gameName.ts`](apps/battle-lab/src/gameName.ts); code, packages and the repo keep their working names and never take it.

Content has three owners, and none reaches another's place. Game battles are generated and field the roster's units. Test units (`test_*`) and `test` maps are the tests', labs' and benchmarks' own; menu units and `menu` maps are the menu backdrop's own. A saved map's `meta.json` says which it is ([saved maps](fixtures/README.md#saved-maps)). The developer menu's entries are tools, labelled as such, and stay in production builds; [browser checks](web/README.md#checks-and-evidence) say which lab pages are tests (scenes) and which are tools.

## How it fits together

The simulation is the one authority; everything else observes it. Start with the
owner of the thing you want to change:

| Owner | Responsibility and entry point |
|---|---|
| [Contracts](crates/contract/README.md) | Shared commands, observations, map admission, physical templates and exact content identities. |
| [Simulation](crates/sim/README.md) | Deterministic battle rules, replay digests, encounter placement and native reports. |
| [Map generation](crates/mapgen/README.md) | Seeded layout, physical compilation, admission and generation reports; shared by native tools and WebAssembly. |
| [WebAssembly boundary](crates/game-wasm/README.md) | Native/browser parity, geometry queries, packed publication memory and binding lifetime. |
| [Presentation](crates/presentation/README.md) | One side's publications to each drawn frame's records: interpolation, poses, effects, cook-offs, tiers and cast lights, shared by every client. |
| [Browser application](web/README.md) | Workers, map adapters, player input/readouts, build configuration and browser verification. |
| [Battle lab](apps/battle-lab/README.md) | Player pages and focused developer fixtures, composed from the same battle view. |
| [Renderer primitives](packages/renderer-core/README.md) | Shared camera/depth contracts, device admission and GPU allocation accounting. |
| [Battle renderer](packages/battle-renderer/README.md) | Observed feeds, terrain, models, light, fog, effects, overlays and GPU lifetime. |
| [Pipelines](crates/pipelines/README.md) | The battle renderer's shaders and pipeline settings, exported for a native client and validated as `wgpu` compiles them. |
| [Scene assets](packages/scene-assets/README.md) | Appearance contracts, validation, baking and loading; includes [Blender authoring](packages/scene-assets/README.md#authored-sources-blender). |
| [Battle audio](packages/battle-audio/README.md) | Recorded and synthetic sound preparation, source calibration and playback from observed causes. |
| [Fixtures](fixtures/README.md) | Authored rules, unit/prop catalogs, generator presets, saved maps and sound assignments. |
| [Assets](assets/README.md) | Source media, prepared runtime files, review sheets and third-party provenance. |
| [Native client](native/client/README.md) | The native client on `wgpu`: the browser's exported shaders and passes over the presentation crate's frames. |

Development tools edit those existing owners: the [mechanics editor](apps/mechanics-editor/README.md),
[map workbench](apps/map-workbench/README.md) and [sound workbench](apps/sound-workbench/README.md).
They share [fixture publication](apps/fixture-publication/README.md), which keeps
reviewed saves consistent and recovers interrupted writes.

[The root task runner](package.json) composes Rust and web commands; [the Rust
workspace](Cargo.toml) and [web manifest](web/package.json) own members,
dependencies and exact command definitions. Packages and apps are source directories;
the web application owns their JavaScript dependencies.

## Rules from first principles

The simulation builds behaviour from low-level physical properties, never from named special cases.

- **Obstacles are bodies:** a shape, a weight class, which mover classes it blocks, a cover tier, and integrity.
- **Movers have a footprint and a push class.**

The rules then follow from those properties alone:

- **Can go here:** the footprint fits, judged by navigation's clearance field (in a tight spot, by the bodies and ground themselves, anywhere the hull fits) and then per-tick collision.
- **Can clear it:** the mover's push class exceeds the body's weight class.
- **Is cover:** a body stands between the soldier and the threat.
- **Breaks:** its integrity runs out.
- **Stops a round, or not:** a round flies through a body that doesn't stop rounds, and wears it if the body has integrity.
- **Holds fire:** a gun holds fire only for what its rounds can't break or can't see past, and fires into anything else on its line until it breaks.

So there is no "wall", "road block" or "tank trap" in the code. Dragon's teeth are just small heavy bodies that block vehicles, and a squad takes cover behind each one because each is a body. Vehicles and props follow the same rules. A new obstacle is an entry in the prop catalog (`fixtures/props/`), not new code.

Combat follows the same idea. A behaviour belongs to the thing that physically has it, never to a unit's role:

- **A weapon owns what it can engage.** For each altitude layer it reaches, it lists a range and the target classes it engages (`infantry`, `vehicle`, `air_rotor`, `air_fixed_wing`, `missile`). Intercepting a missile is just targeting one, fired from a layer the weapon lists `missile` in; a round a shot can catch comes first, because it is about to hit something.
- **A weapon's mechanisms are opt-in flags that compose.** Guidance needs the operator's line of sight to the target. A seeker acquires inside its cone. `requires_clear_launch_path` holds fire when the launcher's own corridor is blocked. `interceptable` lets its rounds be shot down. `fires_on_its_own` fires whatever the unit's engagement policy.
- **A radar contact is a contact like any other.** Its source changes only its evidence (its radius, bearing and range), never who may fire at it; an `anti_radiation` row may also fire at what radiates, a radar contact or a seen unit whose radar emits.
- **A unit owns its body and senses.** That is its hull, its push and weight classes, its sight per layer, its radar (emitting makes it findable, and its reach is how far it sees aircraft), its radar receiver (it finds enemy radars that emit, by bearing and range), its flares (dispensed from a hull), its altitude layer and its flight model.

So there is no "air defence unit" or "anti-tank unit" in the rules. An anti-aircraft gun is a weapon whose air layers list aircraft, and the vehicle carrying it is an air-defence unit only in the roster's grouping. Roles and families group the roster and pick symbols and voices; they never decide behaviour.

These were all built and then deleted, because each stood in for a property something already had:

- **An `air_defense` flag on units.** It gated interceptors, radar contacts and an air-only sight rule. Interceptors now arm from their weapon's `missile` class, a radar receiver finds any unit whose radar emits, and guided fire's line-of-sight rule covers every target.
- **Short, medium and long air-defence tiers.** They drove nothing but labels and validation. How far an air-defence unit reaches is its weapons' range.
- **An `anti_armor` weapon flag.** The weapon's target classes say it engages vehicles.
- **A boolean `can_intercept`.** It became the `missile` target class, listed per layer.
- **"High air" standing in for "flies on wings".** The flight model (`rotor` or `fixed_wing`) is its own data; the layer only sets height and what can be seen.
- **Air targets judged by their height.** A target's layer comes from what was seen or heard, and a round keeps the layer it was fired from.
- **Lane width sized from the widest hull of any unit,** aircraft included. Only ground hulls drive streets.
- **A `sead` unit flag and a second radar block on aircraft.** Finding emitting radars is a sensor a unit has, shown on its card; firing at a radar contact belongs to the weapon's `anti_radiation`. A unit may have either without the other.
- **Auto-fire inferred from an airborne seeker.** An aircraft's seeker missile that reached the air, or homed on radars, skipped the engagement policy, behind a ground-only permission path. A row that fires on its own says so (`fires_on_its_own`), and one permission path covers every unit.
- **A proximity fuse inferred from a motor and a blast.** Any powered round with a blast burst near the aircraft it was fired at. A round with a fuse authors its reach (`proximity_fuse_m`).
- **Hull and speed limits keyed by altitude layer.** The largest aircraft hull and the fastest cruise were read by `low_air` and `high_air`, standing in for helicopter and jet. They are keyed by flight model (`hull_limits.rotor`, `air_speed_limits_kmh.fixed_wing`), and the speed limits are rules data.
- **A hauler or a wreck known by its role or its art.** A vehicle's `_logistics` class and the endurance harness read the `logistics` role, and the simulation knew a wreck by `drawn_by: "wreck"`. A hauler is a unit that carries `supply`; a wreck is a prop row authored `wreck: true`.
- **A `ground_attack` tag (`bomb`, `strafe`) on weapon rows.** A bomb is an unpowered row (`delivery: unpowered`); a strafing gun is the aircraft's always-available gun (`default`).
- **Interceptors armed on the ground only.** Arming against incoming rounds skipped every airborne unit. An aircraft's row that lists `missile` arms as a ground mount's does.
- **Flares as an aircraft's store.** `capabilities.air.flares` and the charges in an aircraft's flight state became `capabilities.flares`, which any unit may carry.
- **A separate pass arming interceptors.** It locked only an idle mount on any round in reach, and aimed at the round as if it stood still. A round is a target like any other, taken through the same shot assessment, lead included, and one a shot can catch comes before the mount's other targets.

If a rule asks "is this an X unit?" or "is this target an aircraft?", the question is usually one of these again.

A catalog row describes a body, while its placement supplies geometry ownership. Garrison seats and building-scaled integrity require a placed building aggregate; ordinary props, generated forest bodies, bridge decks and vehicle wrecks have no such owner. This constraint follows the body's destruction chain. Aggregate bodies remain immovable until the simulation supports moving all their parts together.

It's a game, not a physics simulation. The target is **Hollywood realism**: the battle should look and behave the way a war film makes it look, not the way a ballistics table says. Keep what a viewer expects, even exaggerated, like cover blown apart, shells felling trees and sparks off armour. Drop what looks silly on screen, even when it's physically defensible, like a squad's stray rifle fire mowing down a forest. Sustained, aimed fire may fell one tree; incidental fire shouldn't clear woods. Use first principles where they stay simple, and hard-code a clear game rule where a principled version would be complex. Today's hard-coded rules:

- cover only helps infantry, and crouching is an animation;
- garrisons are a named fighting-position mechanic;
- a soldier's own rounds pass untouched through the tall cover he leans round to fire (he steps out past its edge, fires a burst and tucks back in); everyone else's rounds still hit it;
- suppression pins squads only; a vehicle's crew isn't suppressed;
- active protection (a Trophy) is a simple capability, not an interceptor weapon: while it has charges, any interceptable missile that comes within its standoff is destroyed, with no sight, cone or range of its own;
- who can hold an objective follows the unit card's category (recon, infantry and vehicles), a game rule rather than a physical one;
- the skirmish AI and the scripted encounter defender choose their behaviour by role tags (logistics, artillery, recon, AT, MBT). They're opponents' playbooks, not rules of the battle.

## Running it

You need [Bun](https://bun.sh), [rustup](https://rustup.rs) (it installs the pinned Rust and its `wasm32-unknown-unknown` target from [`rust-toolchain.toml`](rust-toolchain.toml), the same release the deployment builds with), [wasm-pack](https://rustwasm.github.io/wasm-pack/), and a WebGPU browser.

```bash
bun run setup   # install web dependencies and Playwright Chromium
bun run dev     # build the WebAssembly, start the lab app
```

`bun run setup` is also the test prerequisite. It installs the managed Chromium
used by Vitest and by checks that cannot attach to a system browser, then checks
for `ffmpeg` and `ffprobe`, which the audio asset tests execute.
To repair an existing checkout, run `bun run --cwd web install-browsers`.
To check only the native audio tools, run `bun run --cwd web check-prerequisites`.
On WSL, checks prefer headless Windows Chrome when `.wslconfig` uses
`networkingMode=mirrored`; the managed Chromium remains available as the
fallback when Windows Chrome cannot be reached. macOS uses the managed
Chromium directly.

The main menu owns play and tutorial entry. The battle route's URL is a shareable
identity for the admitted map and player choices; its parser and the benchmark
README own the accepted fields. Ordinary Play admits a generated battlefield
before publishing its address, while explicit seeds and saved replays remain exact.

The local [map workbench](apps/map-workbench/README.md) tunes generation on a live
top-down plan. The [mechanics editor](apps/mechanics-editor/README.md) edits resolved
rules and unit families. The [sound workbench](apps/sound-workbench/README.md)
auditions the whole library and edits recipes and assignments. All are available
from the developer menu during `bun run dev`; each README explains its workflow.

[Battle lab documentation](apps/battle-lab/README.md) explains developer routes,
player presentation and their links to shipped design rationales. [Browser
configuration](web/README.md#build-and-serving) owns local serving and deployment
requirements; [native reports](crates/sim/README.md#reports-and-cost) own simulation
and balance measurements.

## Checks

[The task runner](package.json) owns the check, verification and visual commands;
the simulation and browser READMEs explain which evidence each command provides.
Use the narrowest check that can move while iterating and run the full closeout
checks when a spec is finished, as [AGENTS.md](AGENTS.md) requires.

Large media is Git LFS material and generated output belongs in ignored scratch
directories. [Assets](assets/README.md) owns fetching and provenance; [browser
configuration](web/README.md#build-and-serving) owns serving prerequisites and
worktree-specific build isolation.

## Developing in WSL

WSL has no Vulkan driver for the host GPU, so its Chromium draws WebGPU on SwiftShader, a software adapter whose limits a battle exceeds. Use Windows Chrome instead: with WSL's networking set to `mirrored` in `.wslconfig`, it opens the dev server in WSL at its `localhost` address, draws on the real GPU and keeps hot reload. Browser checks make the same choice on their own ([browser choice](web/README.md#checks-and-evidence)).

## Plans and decisions

[Specs](specs/README.md) hold active plans and finished rationale. A plan's own
README is its live handoff; its choices ledger records decisions made where the
plan was silent. [Design references](design/) retain the earliest planning map.
Repository working principles and agent entry points live in [AGENTS.md](AGENTS.md)
and [CLAUDE.md](CLAUDE.md); procedure-specific workflows live under
[`.agents/skills/`](.agents/skills/).
