# Helicopters: decision ledger

The rules the build held to. They came from the 2026-10-10 walk with the user
(`explore-unknowns`) and from the plan synthesis. Changing one needs the user.

Who closed each one: **U** the user, **T** the code and data, **A** the agent on the
user's behalf under the delegation below.

## From the walk (verbatim)

| # | Decision | By | Why |
|---|---|---|---|
| D1 | Weapon rows list the altitude layers they can target: `ground`, `low_air`, `high_air`. Rifles, MGs, snipers and autocannons get `ground` and `low_air`. Tank guns, grenade launchers, RPGs, ATGMs and rocket pods get `ground` only. A stray round that physically flies through a helicopter still hits it. | U | One data column; future anti-air slots in. |
| D2 | **Feature:** harmless plinking (`fires_regardless`) applies to helicopters too. Never remove it. | U | Sparks and tracers look cinematic. Saved to memory. |
| D3 | When shot down, a helicopter falls in a spinning arc that keeps its momentum. It becomes a `light_wreck`, which heavier vehicles can push and fire can wear down, like vehicle wrecks. It bounces off buildings, fells the trees it lands on, and damages the props and units under it. | U + A (arc, unit damage) | War-film image; reuses the existing wreck and felled-tree code. |
| D4 | Cruise is 20 m above the ground. It climbs to roof + 10 m over taller things, with a hard ceiling of 40 m above the ground. A roof over 30 m is an obstacle to route around. | U | Nap-of-the-earth feel. Only metro towers block. |
| D5 | Every armour face is at least 10. Light/utility/heavy are 15 on every face; attack is 25/15/15/10; armoured is 30/25/20/15. HP: light 150, utility 200, heavy 250, attack 250, armoured 300. | U | Autocannon kills in 2–3 hits, HMG in 8–13, rifles do nothing. First pass; balance comes in a later spec. |
| D6 | No collisions in the air. Overlapping helicopters get a soft separation push. | U | Looks better than clipping; avoids complexity. |
| D7 | It turns on the spot when hovering. Cruise is about 2–3 times wheeled road speed. | A | WARNO feel. |
| D8 | **Every hull-mounted weapon (any unit) fires only when the body faces the target** (the turrets' bearing tolerance). A helicopter turns its whole airframe to aim while it keeps flying its route. A ground vehicle turns toward a target only when it has no move order. Infantry are unchanged. | U (general) + A (scope) | `weapons::point_mount` aimed non-turret mounts instantly. That's a bug for every unit. As far as checked, the current roster has only turret mounts. |
| D9 | Helicopters fire missiles **and** rockets on the move. A helicopter ATGM row extends `atgm` with `stationary: false`, and its guidance survives the helicopter's own movement. Ground ATGMs are unchanged. Rocket pods are a new unguided row that can only target the ground. | U + A (separate row, guidance) | The user called it realistic. Changing the shared `atgm` row would change infantry teams. |
| D10 | A helicopter sinks to a low hover while idle inside a deployed supply truck's zone (status Serving, NoStock or Full). It climbs back on any order or engagement. Resupply logic is unchanged. **A change of altitude alone does not count as moving.** | U + A (stays low when Full) | Prevents a sink → `Moving` → climb loop. |
| D11 | Airborne lost-contact marker: a red, perfectly circular sign floating in the air. It goes through a **design spike** (`design-with-images`, then `game-ui`), and the user picks. Contacts gain a height. | U + A (height) | `ContactView.center` is 2D today. |
| D12 | The skirmish AI buys and flies helicopters. **All 19 go live.** Transport types are door-gun gunships until transport exists. Helicopter cards gain roles so the AI's role picker can find them. *(See D35: every card has `roles: []`, so this means rotation entries.)* | U + A (roles) | `skirmish_ai.rs:47-92` picks by role, and helicopters have `roles: []`. |
| D13 | **Placeholder spec: transport** (trucks and helicopters), including landing as a low hover above every ground unit. The resupply sink (D10) ships now and is reused there. | U + A (sink ships now) | Helicopters run dry without resupply. |
| D14 | **Done when** you buy an Apache in a skirmish, it flies in, pops over a village and kills a tank, an IFV shoots it down, and the wreck flattens trees. | U | The user agreed. |
| D15 | Helicopters arrive flying in from the player's map edge, with no road-edge footprint check. | A | WARNO; nothing to collide with in the air. |
| D16 | No automatic return to base. Out of ammo, it waits for orders like any unit. | A | User: "same as every other unit". |
| D17 | Seen over fog (V02); sees vehicles in woods, but not infantry (V05). Hovering over a forest does **not** give the helicopter forest concealment. | T + A | Existing requirements in `specs/battle-foundation/requirements.md`. |
| D18 | A drop line plus a ground ring show where the helicopter is; designed through `game-ui`. | A | Its position can't be read without them. |
| D19 | Damaged helicopters trail smoke. There's no HP-driven smoke anywhere today, so it's new; keep it minimal. Rotor wash comes later. | A | Cinematic consequence; polish later. |
| D20 | Helicopters can't capture or contest objectives (this is already true: `objectives.rs:61`). | T + A | Same as WARNO; the code already says so. |
| D21 | Air routing uses its **own simple 2D grid** whose only blockers are footprints with a roof more than 30 m above the ground. Water, slope, roads and pushing are ignored. It does not add a third `MoverClass`. | A | About 10 code sites read `class != Infantry` as "vehicle" and would silently give a flier water and push rules (`navigation.rs:947…`, `search.rs:269`). |
| D22 | Engagement range against air targets is measured in 3D. Area fire can't target an airborne contact. | A | `engage` uses 3D distance while `select` uses XY (`weapons.rs:677`, `881`, `985`). There's no way to aim at a point in the air. |
| D23 | Rotor sound: a real recording from YouTube, following the precedent in `fixtures/sounds.json` of US-government field footage (public domain). The candidate is "UH-60 Blackhawk B-Roll", credited to Master Sgt. Jason Stadel, 2017 (<https://www.youtube.com/watch?v=vvB-8c9KX-c>). It is 28 s with a steady rotor bed, and a trial download is in `throwaway/explore-helicopters/audio/`. Prepare and assign it through the sound workbench. | U | The user asked for YouTube. |

**Delegation.** The user opted in after five answers. Inferred preferences:
1. War-film feel beats physics.
2. One physical rule, but hard limits where they matter.
3. Consequences are visible and permanent.
4. Balance should be "not clearly broken", with tuning later.
5. Take recommendations that are grounded in the code.

Scope: questions that follow clearly from these preferences. Ask when preferences conflict.

The user also said:
- **Harmless plinking is a feature, not a bug** (D2).
- **Don't spend time on balance.** "Not clearly unbalanced" is the bar; balancing
  every unit is a later spec.
- **Confirm code facts yourself.** The user doesn't read the code.

## From the plan synthesis (agent on the user's behalf)

The three drafts left these open. Each answer follows the preferences above.

| # | Decision | Why |
|---|---|---|
| D24 | The altitude rule reads **every body top**, trees included: altitude = min(ceiling, max(cruise, top + clearance)). Over a 12 m canopy it flies at 22 m. A roof over the obstacle height (30 m) is routed around instead. | One rule with no special case for trees. This replaces the earlier wording "trees never lift a helicopter": trees never force a reroute, but they do lift it 2 m. |
| D25 | The battle-wide air heights live in **one `air` block in `fixtures/game.json`**: cruise 20 m, clearance 10 m, ceiling 40 m, obstacle 30 m, separation radius. `Mobility::Air` carries only per-type numbers: cruise speed, turn rate, climb rate. | The air grid is shared per side, so per-type heights would need a grid per type. One owner. |
| D26 | Cruise speeds come from the unit-roster balance profiles (`specs/done/unit-roster/starter-balance.md`): light 180, utility 220, heavy 230, attack 240, armoured 220 km/h. That's 2.25–3 times the 80 km/h wheeled road speed, matching D7. | Values already chosen; no balance pass. |
| D27 | The low-hover height (D10 and the transport landing) is **derived**: the tallest ground hull top in the catalog plus 2 m. It isn't authored. | "Above every ground unit" stays true when the roster changes. |
| D28 | Chin guns and door guns are **turrets**. Rockets and missiles are **hull mounts**, so the airframe turns to bring them to bear (D8). | User: "the built-in HMG, that's got a turret already, but if it's rockets then it should be able to turn". |
| D29 | `targets` is authored on every root weapon row and inherited through `extends`. `ifv_he` is the BMP-3's 100 mm main-gun HE (`fixtures/units/roster/eastern.json`), not an autocannon, so it inherits `ground` from `tank_he`. | D1's classes, checked against the code. |
| D30 | No belly armour face. A hit from below resolves to front, side or rear, as now (`units.rs:770`). | D5's minimum of 10 already stops every rifle, so a fifth face changes nothing a viewer sees. |
| D31 | **The falling airframe is not a body**, so rounds pass through it. The wreck rests where the arc ends; over a live hull, that hull takes the crash blast and the wreck moves to the nearest spot clear of hulls. The kill and every crash casualty are credited to the shooter who brought it down. | Simple, deterministic, cinematic. |
| D32 | Each side builds its air route grid from **what that side knows**, not physical truth. | The simulation is the one authority, and each side sees only its own (README principle). |
| D33 | Contacts carry the emitter's **height and layer**, firing-report contacts included. The sign floats when the contact is more than the low-hover height above the ground. | A heard helicopter is overhead. |
| D34 | Smoke (D19) starts below **50% HP**. Identified enemy airframes publish a coarse **smoking** bit, never exact HP. | A viewer sees an enemy smoking; nobody sees its health bar. |
| D35 | Helicopters get a real role in `fixtures/units/roles.json`, and the skirmish AI's role picker (`skirmish_ai.rs`) buys and orders them through it, like light vehicles (attack-move). *(Corrected in slice 01: physical units must carry a role; only `planned` cards have `roles: []`.)* | Uses the policy that exists. |
| D36 | Entry (D15) is at the entry site's XY, at cruise altitude, with no footprint check. | Concrete version of D15. |
| D37 | `moved` compares **XY for every unit**. Ground units can't change z without changing XY, so their digests stay unchanged, and a test proves it. | A general rule, not a helicopter case (an altitude change alone would otherwise count as moving). |
| D38 | **Guidance is its own property of a guided row**, separate from the `stationary` firing rule. A guided row (one with `turn_deg_s`) must state `guidance`: `"stationary"` means the launcher guides only while it holds still (today's ATGMs, so their behaviour is unchanged), and `"on_the_move"` means the launcher keeps guiding while it moves (`heli_atgm`). The enum leaves room for a future `"fire_and_forget"` self-guided missile for any unit; it isn't added until a weapon uses it (D43's rule). | **U** (2026-10-10, during implementation): "We shouldn't hardcode missile behavior like this… we need flexibility." This replaces the agent's derived rule `!moved \|\| !def.stationary`. |
| D39 | Any hover bob is drawn only. The sim's altitude moves only toward its target. | Keeps a hover bob out of the digest. |
| D40 | Test stand-ins:<br>• `test_gun_jeep`, with an autocannon on a turret, is the "IFV" in the first checkpoint.<br>• The closing scene (D14) is a deterministic encounter on the test map, followed by a recorded skirmish at a pinned seed with the real Apache.<br>• The red force in the closing scene is bought through the normal purchase commands. *(Overturned at closeout: see choices N5, N6.)* | Tests can't field roster units, and the AI won't reliably buy a tank and an IFV. |
| D41 | Helicopter wrecks use their existing `_wreck.glb` art (`assets/source/roster/disabled/`). | The art already exists. |
| D42 | A wheeled vehicle with a hull mount and no move order holds fire when it can't line up. Latent: no such unit exists. | Completes D8. |
| D43 | The layer enum ships as `AltitudeLayer { Ground, LowAir }`, and weapon rows author `targets` from those two. `HighAir` is added with the first jet or anti-air weapon that reaches it, so the column keeps the shape the user asked for. | Model only what production writes (refactor-clean). Nothing flies high yet. |

## Confirmed in the code (2026-10-10)

1. **Tree heights.** Trunks are 10 m tall and the canopy is 12 m (`fixtures/game.json:511`). A 20 m cruise clears every tree by 8 m, so trees never lift a helicopter.
2. **Model nodes**, read from the 19 `.glb`s after an LFS pull of just those files:
   - **Rotors.** Every helicopter has two rotor nodes: `rotor_main`/`rotor_tail`, `rotor_lower`/`rotor_upper` (Ka-52) or `rotor_front`/`rotor_rear` (Chinook), each with `blade_*` meshes.
   - **Chin guns** are static meshes (`gun_turret`/`gun_barrel` or `gun_mount`/`gun_barrel`) with no runtime rig nodes. The exporters must add `turret`/`gun`/`muzzle` rig nodes.
   - **Door guns** are modelled only on the UH-1Y and UH-60. **Rocket pods** are modelled only on the AH-6M.
   - **No modelled weapon** for its planned HMG: Mi-8, Z-20, Merlin, Wildcat, both Chinooks, NH90 and MH-6M. The Tiger UHT has no rocket pods. Each needs mount nodes in its exporter.
3. **Asset budget.** All committed runtime art is 384 MB. The 512 MiB cap applies per page and counts only the units fielded, which is at most two factions. The 19 source models are 65 MB before baking. There is headroom, and `asset check` still guards it.
4. **Hull-mounted weapons.** All 58 vehicle mounts in `fixtures/units/**` are turrets or ride one. D8 changes no current ground vehicle.
5. **Crash damage.** The existing splash damage fits (U). `damage::blast` (`damage.rs:631`) takes a weapon definition, so the crash is a `helicopter_crash` weapon row: data, not code.
   - Its blast radius covers the airframe.
   - Its structural damage is at least the 100 HP of a tree trunk (`fixtures/props/generic/nature.json`, `topples: true`), so every tree under the wreck falls.
   - Its damage to units is moderate, below a tank HE shell.

Correction to item 1 by D24: trees do lift a helicopter by 2 m; they never force a reroute.

## Rejected alternatives

| Alternative | Why it lost |
|---|---|
| Two flight modes, normal 60 m and low 15 m (old battle-foundation slice 18) | The user wants one always-low layer that pops over roofs. |
| A physical gun elevation limit, instead of the altitude-layer column | A distant helicopter is only about 2° up, so tanks could still shoot it. The user wants a hard rule. |
| Fixed 20 m altitude, with buildings as walls | Needs a full air navigation mesh and boxes helicopters into streets. Replaced by pop-up plus a 30 m obstacle cut. |
| Fixed 70 m altitude above everything | Sees the whole map, and the whole map sees it. Not WARNO. |
| Keeping the 55–110 HP from the old balance pass | One autocannon hit kills almost every helicopter. |
| Lowering autocannon damage | It would change autocannon fire against every ground vehicle. |
| Mid-air collisions | Overcomplicated, and reads as a bug. |
| A third `MoverClass::Air` in ground navigation | About 10 sites read "not infantry" as "vehicle". |
| Changing the shared `atgm` row to `stationary: false` | It would change infantry missile teams. |
| A synthetic rotor sound | The user asked for a real recording from YouTube. |
