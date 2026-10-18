# Transport: the map of unknowns

The `explore-unknowns` walk on 2026-10-16, held with the user. [README.md](README.md) records what shipped; where the drafting pass corrected a card below, the card says so. Every decision below was shown to the user before it closed. "User" means the user answered it, "territory" means the code answered it, and "agent" means the agent answered on the user's behalf under the delegation recorded in section 2. Every number is a starting guess to tune in play, unless it is marked **hard**.

**Done means** (user):
- squads ride, unload and bail out in a played skirmish, and the skirmish AI moves its infantry by transport;
- a transport test scene in the suite, made of a few high-level functional tests rather than many unit tests;
- native gets transport too, unless that proves too complex (the sweep found it cheap: section 4, L6);
- the main-menu reel is unchanged (corrected: the market-town reel was re-recorded for the building bail-out; every event before tick 2093 is unchanged).

## 1. Known knowns: the ground as found

- **Nothing carries passengers.** There is no load or unload order (`crates/contract/src/command.rs:109-195`) and no capability field (`crates/contract/src/catalog.rs:367-391`). Carrier roster rows say "passenger carriage not implemented", and `specs/done/unit-roster/capability-admission.md:38` forbids enabling carriage without a real implementation and a behavioural test.
- **Squad sizes** are the length of the squad's slot list (`catalog.rs:1003-1005`): 8 (rifle, assault, Eastern rifle, Marine), 6 (marksman), 4 (every recon patrol), 3 (ATGM, MANPADS, RPG teams), 2 (heavy snipers). There are no separate MG teams.
- **Building entry** is one stationary timer for entering and leaving, `garrison.enter_exit_s` = 2 s, with the squad within `entry_distance_m` = 4 m (`fixtures/game.json:1080-1084`). The squad stands outside, fully exposed, during the timer; then every soldier appears in his seat in one tick (`crates/sim/src/garrison.rs:474-617`). Any other order cancels it.
- **Building collapse** gives each seated soldier a 50% survival roll on `damage_rng`. A survivor keeps his health and appears unhurt on the nearest free spot within 20 m, then comes out pinned. No spot means death (`garrison.rs:792-879`). There is no animation.
- **Damage is per soldier**, 100 hp each (`crates/sim/src/units.rs:35`, `fixtures/units/roster/shared.json:388`).
- **Orders** queue per unit; Shift sets `queued` on the envelope (`command.rs:198-210`, `units.rs:483-494`).
- **The command bar** has mode buttons (the next click picks a target) and instant buttons. Hotkeys live in one table (`web/src/battle/input/commandBindings.ts:23-32`); `KeyU` is free. Buttons are disabled, never hidden. No button acts in place when pressed twice. Entering a building is a right-click; leaving is a button with no hotkey.
- **Helicopters never land.** They load and unload from the resupply low hover (`crates/sim/src/movement/air.rs:165-178`), as the helicopter walk decided.
- **The skirmish AI** buys on a role rotation with no pairing and no like-for-like rebuy (`crates/sim/src/skirmish_ai/mod.rs:480-541`). It knows only what its side sees.
- **Buying** is one card at a time (`Order::ConfirmPurchase`); there is no deck or bundle.
- **Info-card pips** are always five and show a fraction (`web/src/battle/present/panelRows.ts:136-141`). State rows carry numbers in their words ("SUPPLY 412", `panelRows.ts:47-51`). Compact cards hide every row (`hud.css:1281-1284`).
- **Native** has no HUD, cards, commands or hotkeys; it replays recorded runs (`specs/native-gaps/README.md`).

## 2. Known unknowns: the decision ledger

**Preferences the user confirmed** (checkpoint, after about seven answers): standardise and keep it simple, reusing existing mechanics; judge by how it looks in a war film; the player keeps explicit control, and nothing silently kills soldiers; faction quirks are welcome where the roster produces them; a lean UI. **Delegation scope:** the two questions still open at the checkpoint (buying, AI pairing). Every later question went back to the user.

### Seats and carriers

| ID | Decision | Why | By |
|---|---|---|---|
| D1 | Capacity is a count of seats, in three sizes only: **3, 8, 16**. | Standardise; few numbers. | user |
| D2 | **Recon patrols become 3 soldiers**: drop one scout, keep the marksman, keep Force Recon's grenadier. `test_recon` stays at 4. This is a named outcome change. | Recon then rides the 3-seat tier. | user |
| D3 | **16 seats:** CH-47 (US, Europe), Mi-8, AW101, NH90. **8:** M2 Bradley, Stryker ICV, Stryker Dragoon, LAV-25, ACV-P, BMP-2M, BMP-3, BTR-82A, T-15, ZBL-08, Boxer APC, Boxer RCT30, CV90 Mk IV, CV9040C, Puma, VBCI, UH-60, UH-1Y, Z-20, Mi-35 Hind. **3:** HMMWV, Tigr-M, VBL, M3 Bradley, Stryker RV, BRM-3K, MH-6 Little Bird. **None:** everything else, including all three supply trucks. | War-film rule: 8 means "carries a squad in a film", 3 means "a jeep or scout car". | user |
| D4 | **Supply trucks carry no one.** | They carry supplies. The HEMTT's model is an open bed of crates; none of the three models has seats. | user |
| D5 | Any mix of whole squads may share a carrier's seats. | That is what counting seats means. | user |

### Loading and unloading

| ID | Decision | Why | By |
|---|---|---|---|
| D6 | Boarding and unloading reuse the building rules: both units stopped, the squad exposed during the timer, any other order cancels it. The timer is **1 s** for carriers (2 s stays for buildings). The boarding distance and the unload spots are closer than a building's. A helicopter boards and unloads from its low hover. | Reuse; vehicles are quicker than buildings. | user |
| D7 | **Board** by right-clicking your own carrier with squads selected (they walk to it, following it if it moves), or by right-clicking your own squad with a carrier selected (the carrier drives to it). Shift queues either. If the squads need more seats than there are, the closest whole squads board and the rest are refused visibly with "capacity full". | Mirrors the building right-click; both directions are convenient. | user |
| D8 | **Unload** is a new command bar button and the **U** key, shown only for carriers. U then a map click means "go there and unload"; Shift queues it. **U twice** acts like a fresh move order: it replaces the queue, stops the carrier at once and unloads in place. The building's "Leave building" stays as it is, with no hotkey. | Explicit control. | user |
| D9 | **An explicit unload never kills.** With no room, it is refused with a new "no room to unload" reason. For U twice the refusal comes at once in the order reply; for a go-there or queued unload it comes on arrival, the step is dropped, the carrier stops with everyone aboard, and later queued orders stay. An unload is all or nothing. | Nothing silently kills soldiers. The no-room-means-dead rule belongs to destruction only. | user (all-or-nothing: settled unless said otherwise) |
| D10 | Passengers don't fire, can't be selected and can't be ordered. They exist only through their carrier until they unload. | Your call. | user |

### Destruction

| ID | Decision | Why | By |
|---|---|---|---|
| D11 | **Bail-out injures everyone and kills some**, for carriers and buildings alike. Each soldier rolls to die; survivors lose health. Severity, mildest first: vehicle, building, helicopter. Starting values: vehicle 10% death and 10–30 hp lost; building 25% and 20–40 hp; helicopter 25% and 30–60 hp, after a **50% roll that kills the whole squad**. A soldier with no room to land dies. Survivors come out pinned. | Unhurt-or-dead made no sense. | user |
| D12 | The search limit for a landing spot shrinks to about **8 m for buildings** (from 20 m) and about **5 m for vehicles**. Soldiers appear on the nearest free spot, with no animation. | Appearing far away read wrong. | user |
| D13 | A helicopter shot down keeps its passengers aboard until it hits the ground. They bail out around the wreck, after the crash explosion. A crash that leaves the map kills the squad. | The airframe falls for several ticks (`crash.rs:86-133`). | user |
| D14 | Passengers who die pay their kill reward to the killer's side. | The bail-out reveals them anyway. | user |
| D15 | **Refunding** a loaded carrier refunds and retires its passengers along with it. | Otherwise they stay invisible forever. | user |

### Buying and the AI

| ID | Decision | Why | By |
|---|---|---|---|
| D16 | Transports and squads are bought as separate cards; nothing starts mounted; no bundle. | Preference for reuse: no new buying mechanic. | agent |
| D17 | The AI keeps one transport per infantry or recon squad, 3 seats for 3-man teams and 8 for bigger squads. It loads the squad, drives toward the objective and unloads short of it. A transport that dies is rebought; a squad that dies is replaced and the transport takes it. | The user's opening message. | agent |
| D18 | The AI unloads short of the objective, outside the reach of enemy weapons its side has seen. It never uses hidden information. | The user's opening message; the AI is fog-bound (`specs/done/skirmish-ai/README.md:12-16`). | agent |

### UI and sound

| ID | Decision | Why | By |
|---|---|---|---|
| D19 | Every carrier's card always shows a **Seats row with a count only** (`SEATS 6/8`), in both compact and detailed view, even when empty. No pips. | Seats are what makes a transport. One pip per seat was the intent, but 16 is too long. | user |
| D20 | Each passenger squad's compact card sits under its carrier's card, whether the carrier's card is compact or detailed. Passenger cards are always compact, even with Space held. | Your call. | user |
| D21 | An unload order is drawn as the ordinary move line, with a new unload icon at its destination. | One component, different data. | user |
| D22 | Hovering your own carrier with squads selected shows a new generated "board" cursor icon. | Matches the building cursor. | user |
| D23 | The enemy sees only the carrier: never who or how many are aboard, until soldiers get out or bail out. | Each side sees only what its units see. | user |
| D24 | New spoken lines for boarding and unloading, generated with the Duet CLI's ElevenLabs access (`packages/battle-audio/README.md`). | Your call. | user |

## 3. Unknown knowns: what was drawn out

- **Seats identify a transport.** That is why the Seats row is permanent, even in compact view and even when empty (D19).
- **War-film reading decides roles.** That is why supply trucks carry no one, and why the Hind and the Little Bird carry troops (D3, D4).
- **The player must never lose soldiers to an order.** That is why explicit unloads are refused rather than killing, and why the bail-out death rule is limited to destruction (D9).
- **Consumers:** the player, the skirmish AI, the native replay client, the replay and parity records. The menu reel is out of scope.
- **Proof:** a transport test scene in the suite, with few high-level tests.

## 4. Unknown unknowns: landmines

Every card is decided or a sharp edge. Nothing is OPEN.

- **L1. Passengers must stay alive but be off the field** (sharp edge). `Unit::alive()` (`units.rs:600`) has about 140 call sites that read false as dead: death shouts, the unit cap, casualties, the referee. So "aboard" needs its own "on the field" check, applied at every site: colliders (`battle.rs:2492-2533`); blasts and near-miss suppression (`damage.rs:513-530, 735-779`, which would also consume damage rolls); vehicle shoves and treads (`movement/soldier.rs:287, 1025-1060`, `battle.rs:2374`); sensing and fog (`sensing.rs:235, 246`, `battle.rs:3364`); hearing (`hearing.rs:33-50`, where an enemy would otherwise hear infantry inside the hull); radar (`knowledge.rs:297`); shouts (`shouts.rs:264, 479`); objective capture (`objectives.rs:62-71`, which ignores height, so passengers would capture from a helicopter); resupply (`supply.rs:143-190`, which would heal passengers and spawn replacements outside); the encounter referee (`encounter/referee.rs:33-41`). Passengers still count toward the unit cap. Garrison is no precedent: a garrisoned squad is fully on the field.
- **L2. Passenger positions follow the carrier** (sharp edge): copied every tick, or from `crash.position` while a shot-down helicopter falls. That keeps bodies, sound and bail-out placement right. Bodies are laid with `fall` at ground height (`surface_at`), never at hull height.
- **L3. Orders and the AI** (sharp edge). `validate_units` (`battle.rs:1712`) must refuse passengers. The observation keeps passengers in `own`, marked aboard: if they were dropped, the AI would read boarding as a loss (`skirmish_ai/mod.rs:247, 341, 393, 582`).
- **L4. Determinism** (sharp edge). Bail-out rolls use per-event generators (`mix(seed ^ BAIL_OUT_STREAM ^ host ^ tick)`, then each squad mixes in its own unit id so two squads leaving one host are hurt independently; the pattern of `arrangement::rng` and `battle.rs:761`; corrected 2026-10-16: the crash burst itself draws from `damage_rng`), not `damage_rng`. New unit state folds into `Unit::digest` only when present (the precedent is crashes and the fallen log, `battle.rs:4790-4810`). Missing it fails almost no test: replay tests compare a run against its replay (`tests/battle_authority.rs:145-167`), and the only pinned golden is the menu reel's event logs (`tests/menu_reel.rs:220-238`; corrected 2026-10-16). D2 and D11 change outcomes on purpose. Contract changes refuse older replays through the engine fingerprint.
- **L5. The publication layout** (sharp edge). `OWN_FIELDS` is a fixed table (`publication.rs:339-393`) that the reader matches exactly (`publication/reader.rs:305-325`). New own fields mean re-recording the captures, native's included (`fixtures/README.md:250`), as their own step. Seats total comes from the catalog. (Corrected 2026-10-16: there is no passengers section. The relation is published once, on the passenger's own row, and seats used is derived from it.) Nothing is ever added to `IDENTIFIED_FIELDS`, and the enemy card (`panelRows.ts:509`) never shows seats.
- **L6. Native is cheap** (sharp edge). The only native cost is the capture re-record (L5) and skipping passengers in the shared presentation crate (`presentation.rs:1493-1498`), which native inherits.
- **L7. Two new card behaviours** (sharp edge). Compact view needs an exception for the Seats row (`hud.css:1281`). Passenger cards under their carrier are a new nested-callout layout: every callout today is anchored at its own unit (`readouts.tsx:191-202`, `readoutLayout.ts:39-41, 254-265`). It needs a panel specimen and re-blessed `panels` pictures; `web/tests/uiCoverage.test.ts` fails on any unpinned component. The new icons come from `packages/scene-assets/src/icons.ts`.
- **L8. The hand-mirrored command pipeline** (sharp edge): `command.rs` (Order, OrderError); `web/src/battle/sim/protocol.ts`; the order wording in `useUnitControl.ts:140-175`; `commandReach.ts`; `commandBindings.ts` (and its uniqueness test); the command bar (`readouts.tsx:663-738`); `apps/battle-lab/src/Tutorial.tsx`. Right-clicking your own unit is a new intent (`pointerIntent.ts:101`) with a new cursor (`gameCursor.tsx:7-15`). Refusals need text (`rejectedOrder.tsx:17-27`) and a spoken "can't" line (`packages/battle-audio/src/voices/responses.ts:79-90`).
- **L9. The capability is mirrored in four places** (sharp edge): `Capabilities` (`deny_unknown_fields`, `catalog.rs:365-391`) and its `check`; the TS mirror (`packages/scene-assets/src/units.ts:222-245`); the mechanics editor (`apps/mechanics-editor/src/fields.ts:298-330`); the blessed `fixtures/catalog.json`. Tests use test units only, so add a test carrier in `fixtures/units/test/vehicles.json`. Drop "passenger carriage not implemented" from the carrier descriptions.
- **L10. Records pinning the old rules** (sharp edge). The collapse tests (`tests/garrison.rs:839-1030`, `tests/buildings.rs:1028, 1191`, and `battle.rs:5822-5828`, which pins the draw count) and battle-foundation slice 11 (`specs/battle-foundation/slices/11-garrisons-and-ruins.md:56`) describe all-or-nothing survival. Parity records embed the rules and the catalog (`BLESS_PARITY=1`; `fixtures/parity/exact-numbers.json`, `skirmish-purchases.json`). The survival fields are deleted in a hard cutover, with no `serde(default)` and no migration (corrected 2026-10-16). They live only in `fixtures/game.json` and the parity records `exact-numbers.json`, `skirmish-purchases.json` and `skirmish-objectives.json`, which are re-blessed.
- **L11. Helicopter crashes over a bridge fall to the riverbed** (bug beyond the feature). `crash.rs:128` uses `height_at`, which ignores bridge decks (`world/mod.rs:530-535`); `surface_at` knows them. Fix it in its own commit, first. If the fix turns out complicated, skip it (user).
- **L12. A clear spot under a hover** (sharp edge). Search outward with `arrangement::nearest_free` from the helicopter's x/y, with z from `surface_at` (bridges), excluding live hulls and other soldiers as well as props (`standing_room` checks props only). Over water or a roof, the search reaches the bank or the building's side, or fails (D9 refuses, D11 kills).
- **L13. Refund** (decided, D15): `retire_withdrawals` (`battle.rs:915-988`) must retire and refund passengers with the carrier, and a refund aimed at a passenger is refused.
- **L14. A seen squad that boards** lapses to a last-seen contact beside the carrier (`knowledge.rs:436`). That is correct fog behaviour, not a leak.

## Confirm before coding

- The new "on the field" check lands at every L1 site in one pass. Grep `alive()` and list each site's verdict in the slice.
- The bail-out spot search excludes units as well as props (L12).
