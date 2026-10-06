# Choices ledger

These are the decisions the build made where the spec was silent or contradicted itself. They are judged against the village checkpoint (slices 01–16) as it finally shipped. The first group needs your call. The second is worth a second look. The rest are sound. Within each group, the least confident entries come first. Your own rules live in requirements.md, and the resolutions the spec itself made live in decisions.md.

## Needs your call

### The crossfire target is split out, not forced
- **When:** slice 15.
- **The choice:** encounter.md asks the village to show at least one tank lost to a prepared crossfire of two AT teams. Across the ten comparison seeds, that loss never happens. The scripted tanks turn back when the red tank first hits them from long range, which is about 400 m before either AT team can join in. The target is recorded as unmet. The crossfire mechanic is still shown in slice 10's ambush lab, where a crossfire beats a prompt escape. No spawn point was moved just to manufacture a loss.
- **Why / the gap:** encounter.md asks for the loss, and slice 15 allows a failing mechanic to be split out rather than forced.
- **The reach:** whether the village battle alone shows a crossfire at all.
- **Provisional call:** accept the split. **To reverse:** move the red tank (or the AT teams) in `fixtures/game.json` so the tanks reach the crossfire, then rerun the village report.
- **Confidence:** low.


### The village's hold progress is public
- **When:** slice 15.
- **The choice:** Both sides see the encounter status, which is the seconds the zone has been held and the verdict. So a player standing in the village and still reading "0 s held" learns that some enemy remains inside the zone, but never who, how many or where. The supported comparison script uses exactly that to decide whether to keep shelling and sweeping.
- **Why / the gap:** encounter.md defines the hold rule but not what a player sees of it. The observation rule forbids hidden state except through legal cues.
- **The reach:** the village panel, the supported script, and its capture results.
- **Provisional call:** public. Capture-point games normally show "contested". **To reverse:** show only the verdict on the panel, and have the supported script sweep on a timer instead.
- **Confidence:** medium.

### A unit's own fire never suppresses it
- **When:** slice 07 (kept through slice 09).
- **The choice:** A near miss is a round passing close to a body, and it adds suppression. Suppression is the "pinned down" state that slows infantry. Near misses are never counted against the unit that fired the round. So a squad's own bullets passing its members don't pin it down. Near misses from any other unit, friend or foe, still count.
- **Why / the gap:** the spec never said whether your own outgoing fire suppresses you.
- **The reach:** how quickly squads get pinned, especially in long firefights.
- **Provisional call:** own fire never suppresses the firer. **To reverse:** drop the firing-unit check from the near-miss loop in `sim/src/flight/mod.rs`. That one check also stops a unit's rounds from hitting its own soldiers (see "A squad's rounds pass through its own soldiers"), so the two rules must be split first.
- **Confidence:** medium.

## Worth a second look

### The stress battle's late state is authored, not played
- **When:** slice 16.
- **The choice:** The late state is the aftermath of a long battle: 20,000 fallen soldiers and 2,000 wrecks. It is built at the start, not reached by playing. The fallen are 2,500 whole rifle squads that start dead, their soldiers lying in formation, which uses the same records a live battle fills. The wrecks are placed as map props, so every side knows them from the first tick. A wreck made in a real battle is learned only when seen. The late state draws from its own random stream, so it never changes the waves.
- **Why / the gap:** validation.md asks for a synthetic late state but not how to build it.
- **The reach:** every late-state number. Because the wrecks are map props, fog, knowledge and publication treat them differently from real wrecks. Slice 16b lists "author them as battle-made wrecks" as an option.
- **Confidence:** medium.

### Garrisons use the fixture's building concealment
- **When:** slice 11.
- **The choice:** A soldier inside a building is detected at 0.2 times the normal range (`sensors.building_range_multiplier`). That value sat unused in the fixture until now. When forest also hides the soldier, the stronger concealment wins, and the two are never multiplied. Infantry then spot a garrison at about 120 m, and a tank at about 70 m.
- **Why / the gap:** no slice said what the value was for. The brief says a building hides you much better than a forest does.
- **The reach:** Enemies mostly find a garrison when it fires, which leaves them a firing area to shoot at. This changes how the village plays, and the number came from the provisional fixture rather than from you.
- **Confidence:** medium.

### A squad's rounds pass through its own soldiers
- **When:** slice 09 (changed slice 07's rule).
- **The choice:** A unit's rounds can never hit that unit's own bodies. Slice 07 exempted only the soldier who fired. As a result, a squad's back rank shot its own front rank on every volley. Soldiers of every other unit, friend or foe, still take the round (P09).
- **Why / the gap:** P09 says collisions apply "regardless of allegiance", but it doesn't say whether a squad's synchronized volley may pass through the squad itself. The other option was to make back-rank soldiers hold fire, but P11 says infantry never holds fire for friends.
- **The reach:** every squad volley, and a tank can't strike its own hull. It reversed a behaviour that slice 07 had pinned, although that behaviour was itself an unreviewed choice.
- **Confidence:** medium.

### Enemy tracers are clipped to seen ground
- **When:** slice 08.
- **The choice:** An enemy round's flight is drawn only where it passes over ground your side currently sees. That ground comes from the same 8 m visibility grid the fog uses, refreshed every 6 ticks. Each drawn stretch stops at its last sample over seen ground. The alternative was a line-of-sight check from every friendly eye to every round, which costs a check per round per tick.
- **Why / the gap:** slice 06's carry-in said "ground visibility field plus line of sight".
- **The reach:** A round high above a hidden valley stays hidden, even when someone could see that patch of sky. Tracers can lag fog changes by up to 0.2 s. The rule is conservative, since it never shows more than the fog, but it isn't the stated rule.
- **Confidence:** medium.

### Supply waits for the next item in order, even when a cheaper one fits
- **When:** slice 13.
- **The choice:** Service follows the contract's order: ammunition first, then vehicle health, then replacement soldiers. If the truck can't pay for the next item, the unit waits with "no stock", even when a cheaper item later in the order would fit. For example, an AT team whose next missile costs 20 still waits when the truck's last 15 points would buy a soldier. A repair point costs one whole point of stock, even when less than a point of health is missing.
- **Why / the gap:** the contract gives the order but not what happens when stock runs short.
- **The reach:** the last few points of every truck's stock. Skipping ahead would use stock more fully, but it would reorder service.
- **Confidence:** medium.

### "Not firing" means not fighting, but the panel says "fired this moment"
- **When:** slice 13.
- **The choice:** Supply serves only units that stand still and are not firing (L04). A unit counts as firing, and so isn't served, while any of its weapons is working a shot: aiming, reloading, turning its turret, firing, or guiding a missile. The weapon decides this each tick and stores it on its lock, the record of what it is aiming at. The supply check reads that stored flag rather than the reason text shown to the player. "Standing still" means no movement order at all, so a tank turning on the spot to drive off is already moving. Supply trucks are never served, not even by another truck.
- **Why / the gap:** L04 doesn't say whether the gaps between shots count as firing.
- **The reach:** Squads must break off a fight to be resupplied, which is the rotation L04 wants. The panel describes this state as "waiting for supply: fired this moment". That undersells it: a unit that is only aiming shows the same words.
- **Confidence:** medium.

### Readouts slide out from under the panel
- **When:** slice 16 (closing cleanup).
- **The choice:** A unit's readout cluster (its weapon rings) and the name tags are placed together once per frame. The panel boxes are marked as no-go areas. Anything that would land under a panel slides to the panel's right. A cluster that would cover another rises above it. A destination name that would cover anything drops below its ring, then further down if needed.
- **Why / the gap:** the spec didn't say what to do when readouts collide with the panel or each other. Before this, clusters were cut off under the panel ("ank #5") and names printed over each other.
- **The reach:** Nothing is hidden or overprinted. But a cluster that slid sideways no longer sits over its own unit, so when many units bunch up near the panel, it can look as if it belongs to a neighbour. Only selected units carry a name to disambiguate.
- **Confidence:** medium.

## Sound

### Pan keys: Total War layout (decided by the user)
- **When:** slice 04; decided by the user on 2026-09-25 during the battle-look planning.
- **The choice:** The camera pans with WASD (and the arrows), rotates with Q/E, zooms with the wheel and orbits with middle-drag, Total War style, so the letter commands moved off those keys. Battle-look shipped it: the command keys are `CommandBindings` (`web/src/battle/input/commandBindings.ts`), the camera's are renderer-core's `CameraController`.
- **Why / the gap:** contracts.md bound S and A both to commands and to WASD panning; the user chose Total War's layout.
- **The reach:** every keyboard command and the camera.
- **Confidence:** high.

### What each kind of prop blocks (your decision)
- **When:** slices 02 and 09; changed by your decision after the checkpoint.
- **The choice:** Every prop kind says, per kind of mover, whether it stops it: one table (`PropKind::blocks`), with infantry and vehicles as the two mover classes today. A wreck stops vehicles only; soldiers climb over and through it. Buildings, walls, crates and ruins stop everyone. Tree trunks and bridge decks stop no one: trunks still stop rounds and sight, so a tank drives through a forest more slowly (M02) instead of routing round every trunk, and a bridge deck is driven on, not around. A wreck still hides what is behind it and still stops rounds by its shape. Route planning keeps a separate "fits here" map per mover class, so a squad plans straight across a wreck while a tank plans round it.
- **Why / the gap:** You asked that wrecks block vehicles but not infantry, and that blocking be a per-prop property so later props can block infantry but not vehicles (a fence), or light vehicles but not heavy ones: each is a new column or row in the one table.
- **The reach:** Narrow village lanes stay open to infantry after a tank dies in one. The geometry lab's traversal view colours a prop red when it stops everyone and amber when it stops only some movers.
- **Confidence:** high.

### Screenshots are regenerated, not committed
- **When:** slice 01.
- **The choice:** Each lab scene writes its evidence images into `throwaway/evidence/<fixture-id>/`, a folder git ignores. To see them, rerun the scene, for example `bun run --cwd web scene -- foundation`. The committed record is each slice's verdict: the numbers, and every critique finding with its outcome.
- **Why / the gap:** the spec first asked for images committed under `specs/.../assets/evidence/`. The workflow keeps raw captures out of git, so every re-capture doesn't pile PNGs into history.
- **The reach:** every slice. Old captures are lost on the next run unless copied aside.
- **Confidence:** medium.

### Plateau ("mesa") relief replaced the ramp
- **When:** slice 02.
- **The choice:** Map relief comes in two shapes. A ridge follows the village formula. A mesa is a flat-topped rectangle whose sides fall at a set angle. An earlier one-sided ramp dropped straight to zero at its edges, which made accidental cliffs.
- **Why / the gap:** the spec asked for "hill, slope threshold" without a list of shapes.
- **The reach:** maps are authored from ridges and mesas, and a new shape is one more variant.
- **Confidence:** medium.

### Water is see-through; bridges are decks over it
- **When:** slice 02.
- **The choice:** Water is a rectangle whose ground is lowered to a bed, with a translucent sheet drawn at the surface. The sheet isn't solid, so rays and rounds pass through it to the bed. A bridge is a deck prop that is solid for rays and rounds, and its top counts as walkable ground. The ground under the bridge is still water.
- **Why / the gap:** the spec asked for a walkable top surface over impassable water, but not whether water stops sight or rounds.
- **The reach:** rounds fired at a river strike its bed. Water that stops rounds would need a new rule.
- **Confidence:** medium.

### A malformed command still uses up its sequence number
- **When:** slice 03.
- **The choice:** Each command carries a per-side sequence number that must be exactly the next one (1, 2, 3, …). If the number is wrong, the command is refused and the counter doesn't move. If the number is right but the content is bad (another side's unit, a missing unit, a point off the map), the command is refused, its number is spent, and it goes into the replay log. A replay then submits it again and gets the same refusal.
- **Why / the gap:** the spec asked for ordered acknowledgements and a replay of "accepted commands" without saying whether a bad command counts as accepted. Logging only valid commands would let replay acknowledgements drift from what the player saw.
- **The reach:** replays and any future networking.
- **Confidence:** medium.

### The page keeps its own copy of the static map
- **When:** slice 03.
- **The choice:** The battle runs in a web worker. The page builds its own read-only copy of the fixed starting map, from the same Rust geometry code compiled to WebAssembly, and uses it to draw terrain and to find the ground under a right-click. Only the starting map is copied, and every side knows that from the start. Anything that changes later, such as wrecks and ruins, reaches the page only through that side's observations.
- **Why / the gap:** the spec said picks use "the authoritative surface" and static geometry is public, but not where the pick runs. Asking the worker would add a round trip to every click.
- **The reach:** wrecks and ruins are drawn from observed prop changes, never from hidden truth.
- **Confidence:** medium.

### The simulation catches up at most four ticks, then runs slow
- **When:** slice 03.
- **The choice:** If the page falls behind, each wake-up runs at most 4 ticks. It then drops the rest of the backlog and reports "running slow". It never skips ticks, so the battle just runs slower than real time. If the page stops handing back its buffers, ticking stops ("waiting-consumer"). A hidden tab pauses, and on resume the clock restarts from now instead of racing through the missed time.
- **Why / the gap:** the spec required bounded catch-up and explicit slowdown. The number 4 is a choice.
- **The reach:** when a step costs more than a tick, as in the late-state stress battle, the battle visibly slows instead of skipping.
- **Confidence:** medium.

### Routes are planned on 2 m cells, so gaps must be about 4 m for infantry
- **When:** slice 04.
- **The choice:** The route planner divides the map into 2 m squares. A square counts as blocked if any solid prop overlaps it, so a planned route never goes through a wall. As a consequence, infantry are sure of a way through only when a gap is about 4 m wide, and a tank (3.6 m wide) needs about 6 m. Actual movement uses exact prop shapes; only planning is coarse.
- **Why / the gap:** the spec fixed the rule that infantry use gaps a vehicle can't, but not the planning resolution.
- **The reach:** village streets are tens of metres wide, so it doesn't matter there. A 2 m doorway stays closed to everyone. Narrower alleys on later maps would need 1 m cells, at 4× the memory and search time.
- **Confidence:** medium.

### Head-on vehicle deadlocks are broken by id
- **When:** slice 04.
- **The choice:** Two tanks meeting nose to nose both stop, each reporting that it is waiting for the other. After 2 s, the tank with the higher id replans as if the other were a wall and drives round it. If there's no way round, it keeps waiting, still naming the blocker, and tries again every 2 s. Squads never deadlock: they sidestep vehicles and gently push apart from each other.
- **Why / the gap:** the spec asked that vehicles avoid each other or wait, and that crowds make progress or explain why not. It gave no rule for deadlocks.
- **The reach:** every vehicle meeting, including convoys in narrow streets.
- **Confidence:** medium.

### A squad is eight soldiers in two loose ranks
- **When:** slice 04.
- **The choice:** A squad is one unit you select and order. Inside, it holds real soldiers: 8 for rifles, 4 for recon, 3 for AT. They stand in two staggered ranks 2.5 m apart around the squad's position and facing. Each soldier is drawn, seen, hit and killed on its own. The spacing isn't a formation command, and players can't change it.
- **Why / the gap:** the spec required per-soldier sight and casualties but gave no formation shape.
- **The reach:** hit rates depend on how spread out squads are. The spacing can be tuned.
- **Confidence:** medium.

### Group moves keep each unit's place in the group
- **When:** slice 04.
- **The choice:** Right-click with several units selected, and each unit's destination is the click point plus its current offset from the group's centre. A column arrives as a column. Offsets wider than 40 m are scaled down. A destination inside an obstacle moves to the nearest standing room within 16 m, and failing that uses the click point. Each unit plans and drives at its own speed. Queued (Shift) moves take offsets from positions at the moment the order is given.
- **Why / the gap:** the spec said "preserve relative destination positions where space permits" without numbers.
- **The reach:** every group order, the defender and the comparison scripts.
- **Confidence:** medium.

### Fog redraws five times a second; spotting runs every tick
- **When:** slice 05.
- **The choice:** Two things answer "what can I see". Spotting (identification) decides which enemies your side knows about. It casts exact sight lines from every observer to every nearby enemy soldier or hull on every tick, 30 times a second. The fog, the darkening on the map, comes from a sweep of sight lines over 8 m squares and is redone every 6 ticks per side. Its outline can trail true sight by up to 0.2 s.
- **Why / the gap:** the spec allowed a slower rate "with a bounded visibility delay". The fog sweep is far too costly to run 30 times a second for both sides.
- **The reach:** the fog outline, and tracer clipping, which uses the same grid. Spreading the sweep over ticks is one of slice 16b's options.
- **Confidence:** medium.

### An enemy's handle survives a brief loss of sight, then is replaced
- **When:** slice 05.
- **The choice:** Your side never learns an enemy's real id. It sees a handle such as "contact 7", assigned when that enemy was first spotted. If contact 7 goes out of sight and comes back within 1.5 s (the agreed grace), it keeps the handle, so your weapons keep aiming at it. If it stays hidden longer, the handle is retired, and when it reappears it gets a new one, because your side can't know it's the same tank.
- **Why / the gap:** the spec required side-scoped handles and a 1.5 s grace, but not when a handle is reissued. One handle forever would quietly tell you "that's the tank you saw five minutes ago".
- **The reach:** weapon locks and last-seen areas key on handles.
- **Confidence:** medium.

### Lab scripts move the enemy on a timetable
- **When:** slice 05.
- **The choice:** A lab fixture can list timed orders for either side, such as "at tick 30, red tank 3 moves to (480, 300)". These are part of the scenario, like a wall that drops at a set tick. They run the same way live and in replay, and they don't use command sequence numbers. The village defender is different: it decides from red's own observation, sends ordinary commands, and those are recorded in the replay.
- **Why / the gap:** the spec mentioned deterministic command scripts in fixtures without saying how they enter the battle.
- **The reach:** every lab with a moving enemy.
- **Confidence:** medium.

### A lost sighting leaves a full-size area at the last position
- **When:** slice 06.
- **The choice:** When your side loses sight of a spotted enemy, a red disc appears where it was last seen. The disc stays put and fades over the contact's life. (Since 2026-09-28 a contact's radius is 3 × its cause's footprint radius and it lasts 30 s: battle-look decisions.) If the enemy is spotted again, the disc goes.
- **Why / the gap:** V09 said "use the contact visual language" without a radius. A smaller disc would suggest a precision that decays at an unknown rate.
- **The reach:** return fire at areas, and the defender's and scripts' reactions.
- **Confidence:** medium.

### Each sound goes to the nearest listener that heard it
- **When:** slice 06.
- **The choice:** Every half second, each unseen enemy makes at most one sound of each kind: engine or footsteps, plus gunfire if it fired. The sound goes to the nearest friendly unit within hearing range for that kind: 200 m for infantry, 650 m for vehicles, 1000 m for shots. The caption reads like "Heard engine, moving, near, east of recon #0". Direction snaps to 8 compass points, and distance is near (within half the range) or far. Enemies you can see make no sounds, and idle units still make noise ("voices", "engine, idling").
- **Why / the gap:** V11 fixed the rules but not who owns a sound that several units hear.
- **The reach:** audio and captions everywhere.
- **Confidence:** medium.

### A shot through a ridge is refused, never re-aimed over it
- **When:** slice 07.
- **The choice:** Before a weapon fires, its curved path is flown against the real terrain and props. If something would stop the round more than 0.5 m short of the target, the answer is "blocked" and nothing fires. A tank never quietly switches to a lobbed arc to clear a hill. Only a weapon row marked `"trajectory": "indirect"` tries the high arc first. No village weapon is indirect; the ballistics lab uses a lab-only mortar row to show one. Accuracy spread is applied after this check, so a scattered round that clips the crest is a real miss.
- **Why / the gap:** P03 forbids ignoring the ridge but gave no arrival tolerance or order of arcs, and the fixture had no way to mark indirect fire.
- **The reach:** the "blocked trajectory" reason, and future artillery, which must set the flag.
- **Confidence:** medium.

### How a round's flight ends
- **When:** slice 07.
- **The choice:** A round expires after its weapon row's lifetime (the ATGM's 12 s), or after a 30 s physics cap for rows without one. A row asking for more than the cap is a setup error, and the launch solver only looks for hits within the lifetime. A round that is off the map and moving away ends there with a "left the map" event, since nothing can bring it back. Every round reports exactly one ending.
- **Why / the gap:** "bounded lifetime" gave no source for the number, and only hit and lifetime endings were specified.
- **The reach:** every weapon, and everything that reads flight events.
- **Confidence:** medium.

### Scatter is solved on the same arc; unreachable scatter is a refused shot
- **When:** slice 07.
- **The choice:** Accuracy spread moves the aim point sideways and up or down, in the plane facing the shooter. The round is then solved to hit the moved point on the same low or high arc. At the edge of range the moved point may be out of reach. The shot is then refused and the random draw is still used up. It never falls back to a perfect unscattered shot.
- **Why / the gap:** the spec converts spread before solving but doesn't cover this edge case.
- **The reach:** only fire at extreme range.
- **Confidence:** medium.

### Flight events are ordered by time within the tick
- **When:** slice 07.
- **The choice:** Events in one tick are sorted by when they happen within the tick. Ties go by round id, then a round's near misses before its ending, then by unit.
- **Why / the gap:** "ordered events" had no defined order.
- **The reach:** damage and suppression are applied in this order.
- **Confidence:** medium.

### Moving fire spreads √2 wider
- **When:** slice 08.
- **The choice:** A weapon that may fire on the move widens its spread by √2 while moving (`physics.moving_scatter_multiplier`). Against a small target, the chance to hit falls with the square of the spread, so this halves hits, which is the brief's "50% accuracy reduction".
- **Why / the gap:** W04 gave the target but not what to scale. There's no separate hit chance to halve, because every round flies physically.
- **The reach:** every shot on the move, and village balance.
- **Confidence:** medium.

### A blocked target can be swapped for a shootable one while reloading
- **When:** slice 08.
- **The choice:** A weapon reconsiders its target while reloading. Automatic choice only picks targets it could shoot now: in range, with a clear path, and no friendly vehicle in the way. If the tank you were shooting slips behind a wall mid-reload, the gun may switch to something it can reach. If nothing better exists, it keeps the old target and shows "blocked trajectory". The 1.5 s grace still protects a target that is only briefly out of sight.
- **Why / the gap:** the contract said "highest-cost identified damageable target" without saying whether a blocked one counts.
- **The reach:** target switching around cover.
- **Confidence:** medium.

### A squad weapon fires one round per living soldier
- **When:** slice 08.
- **The choice:** A squad's rifles are one weapon with one aim and one reload, but each shot sends one round from every living soldier. The rounds aim at seen enemy soldiers in turn, or at points inside a firing area.
- **Why / the gap:** the spec gave squads a rifle weapon without saying how members contribute. A single round would make squad strength irrelevant to firepower.
- **The reach:** squad firepower and damage.
- **Confidence:** medium.

### Weapons hold fire within 1 m of a friendly vehicle
- **When:** slice 08.
- **The choice:** A weapon holds fire when its predicted path passes within 1 m of a friendly vehicle's hull, or when the aim point's blast would reach one (`physics.friendly_prefire_margin_m`). Friendly infantry never stops a shot (P11).
- **Why / the gap:** P11 said "obstruct the predicted path" with no tolerance.
- **The reach:** tanks firing past each other in a column.
- **Confidence:** medium.

### Blast damage uses the weapon's one damage figure
- **When:** slice 09.
- **The choice:** An explosive round's direct hit does its `damage`. Its blast does that same damage scaled by `(1 − r/R)`, where r is the distance and R the blast radius, to each soldier whose fragment roll hits. The body struck directly is skipped by the blast.
- **Why / the gap:** the contracts say "configured blast damage", but the fixture has only one damage value per weapon.
- **The reach:** HE, grenade and ATGM lethality. A separate blast figure could be added later as a tuning value without changing the rule.
- **Confidence:** medium.

### Which armour face a hit meets
- **When:** slice 09.
- **The choice:** For a round, the face is read from where it struck. Take the point just outside the hull, in the hull's own frame, scaled by the box's half-sizes. It's the roof if the point is further above than beside; otherwise front, rear or side, whichever direction is largest. A blast uses the same rule at its burst point, needs a clear line to the hull's centre, and falls off with distance to the hull's surface.
- **Why / the gap:** P10 names four faces but not how to pick one at an edge or corner.
- **The reach:** the value of flanking tanks, and AT ambush angles.
- **Confidence:** medium.

### One round suppresses a squad by its strongest effect only
- **When:** slice 09.
- **The choice:** In a tick, one round's near miss and its nearby impact could both suppress the same squad. Only the stronger counts. Impacts are measured to the squad's nearest standing soldier. Vehicles are never suppressed. Friendly rounds suppress friendly squads, because the rule measures distance rather than who fired, except for the unit that fired the round.
- **Why / the gap:** the contracts say "one near-miss suppression event per squad per tick" but not how near misses and impacts combine.
- **The reach:** how quickly squads get pinned.
- **Confidence:** medium.

### What happens when a unit dies
- **When:** slice 09.
- **The choice:** A destroyed vehicle or a squad with nobody standing drops out of its side's unit list. Its wreck and fallen soldiers stay on the map, and any order naming it is refused as "destroyed". If the enemy side had it spotted when it died, the enemy's track ends at once with no last-seen area, and an attack order on it counts as complete. If it died unseen, nothing changes: the track lapses after the usual 1.5 s and a last-seen area follows. Enemy fallen soldiers appear once your side has seen the ground they lie on, and are remembered afterwards. Your own fallen are always shown. Your own vehicle's wreck is always known too, because a side knows where its vehicle died; otherwise a tank dying with no other friendly eyes on its spot vanished from its own side's picture. A side that had an enemy vehicle spotted when it died sees its wreck appear.
- **Why / the gap:** the spec says "a dead visible target completes the attack" and "corpses remain", but not how a dead unit appears to its owner, or who learns of a death and when.
- **The reach:** selection, attack orders, and what a player can infer.
- **Confidence:** medium.

### Guided missiles fly straight and chase what the launcher sees
- **When:** slice 10.
- **The choice:** A weapon row with a turn rate is guided. The missile flies at a constant speed with no gravity, turning at most that rate toward a steering point. Launch solving, the friendly-vehicle check and flight all use that same path. While the launcher supports it, the steering point is the target's position as the launcher last saw it, with no lead. At 180 m/s against a 6 m/s tank, chasing is enough, and it never uses a prediction the launcher couldn't make. (Since 2026-09-28 the speed is not constant: a row's `accel_mps2` and `top_speed_mps` give it a rocket motor, thrust along its heading, and solving, the predicted path and flight share that model. Chasing still hits at 700 m/s; see `specs/done/battle-look/decisions.md`.)
- **Why / the gap:** the contract says steering "follows observed target motion within turn limits" but gives no flight model or steering law.
- **The reach:** every ATGM shot, and later anti-air missiles.
- **Confidence:** medium.

### The ambush lab's escape uses cover beside the tank
- **When:** slice 10.
- **The choice:** A missile covers 500 m in about 3 s, and a tank moves 6 m/s, so only cover a few metres away can break sight in time. A hill tens of metres away never can. The lab therefore puts a building right beside the tank. A prompt escape behind it is untouched, a late escape is hit, and a prepared crossfire, where a second team still sees the tank, hits it.
- **Why / the gap:** the slice named a "hill escape".
- **The reach:** the lab only. The village's own geometry decides real escapes.
- **Confidence:** medium.

### Deploy, Pack and moving: what a truck remembers
- **When:** slice 12.
- **The choice:** A deploying unit (the supply truck) stores one posture it holds when it has nowhere to go: deployed or packed. Deployed is the default, because a stopped truck sets up. Its target is worked out each tick: packed while it has somewhere to go, otherwise the stored posture. Deploy clears the truck's orders like Stop (its weapons are untouched), and it sets up where it is. Pack only changes the stored posture, so a moving truck keeps moving. Stop, Deploy and any new move or attack order reset the posture to deployed. The Shift (queue) flag is ignored for both. Units that never deploy ignore both orders, so Deploy on a mixed selection never stops a tank. A truck enters the battle packed and, with no orders, is set up after the full 15 s.
- **Why / the gap:** the contract gives the rules (moving packs, a stopped unit deploys, Pack keeps it packed) but not what is stored or how the orders meet the queue. Storing the target itself would need a second "was I moving?" memory, which the slice forbids.
- **The reach:** an explicit Pack is used up by the next move, so the truck sets up again where it arrives. To arrive packed, press Pack while it drives. An authored "starts deployed" field can be added when an encounter needs one.
- **Confidence:** medium.

### Deployment is shown as a ground ring and a simple pose
- **When:** slice 12.
- **The choice:** Around a deploying truck, the ground carries a thin track ring and a bright arc. The arc runs clockwise from the nose, and its length is how far set-up has got. It is green while deploying and amber while packing. A white arrowhead at the moving end shows the direction, so direction never depends on colour alone. When fully set up, a dark-green disc fills the ring. The truck's pose depends only on progress: four legs slide out and down, and a mast with a lamp rises. Fully packed, every part is inside the hull. The weapon readouts add a "SETUP" square with ▲ or ▼ while setting up or packing. The battle panel says "deploying 40%", and the deployment lab's panel shows seconds and "↻ deploying" / "↺ packing".
- **Why / the gap:** the folded and unfolded pose was delegated, and the indicator's form was unspecified.
- **The reach:** real models can replace the parts later without touching the rule. The two panels word progress differently.
- **Confidence:** medium.

### Soldiers take facing slots one tick after a weapon locks
- **When:** slice 11.
- **The choice:** Each tick, before weapons act, each garrisoned weapon's current target, as its own side sees it, decides who stands where. A soldier fires only from a slot whose wall the line to the target leaves at more than 6° (`garrison.slot_facing_min_deg`). The widest spread in the fixture is about 2.6°, so a round can never graze back into its own wall. A squad weapon moves every living soldier, and a single weapon moves its operator. A soldier already facing the target stays put. Otherwise it takes the nearest free facing slot, with the lower slot number winning ties. If none is free, it waits, and the weapon reports "no facing slot". (Since 2026-09-28 a soldier may instead trade with a squadmate who needs the window less, holding fire for the swap: battle-look decisions.) A fresh target gets its soldiers on the next tick; the aim time (0.8 s or more) hides that.
- **Why / the gap:** "slot relocation is a one-tick abstraction" said neither when it runs, who moves, nor how grazing angles are judged.
- **The reach:** In a full building, a target on one side is fired on only by the soldiers already facing it. A target almost parallel to a wall is served by the next wall round the corner, and a corner target by two walls.
- **Confidence:** medium.

### Building capacity is checked at the order and again at the door
- **When:** slice 11.
- **The choice:** A garrison order is refused as "capacity full" if the ordered squads, the side's occupants and its squads already heading in wouldn't all fit. Only the side's own units are counted, so the refusal reveals nothing hidden. When the entry timer ends, the squad is checked again against who is actually inside. If it no longer fits, or enemies hold the building, it waits beside the building ("no room: waiting") and tries every tick. It never splits.
- **Superseded (2026-09-28):** a building takes one squad; a second is refused outright and nothing waits (`specs/done/battle-look/decisions.md`).
- **Why / the gap:** the contract covered a squad that doesn't fit, but not two orders in one tick or a building the enemy holds.
- **The reach:** a player learns that an enemy holds a building only by walking up to it, which is physical contact, like bumping into an unseen wreck.
- **Confidence:** medium.

### Entering and leaving a building
- **When:** slice 11.
- **The choice:** A squad walks to a point 2 m outside the nearest wall. Within 4 m of the walls (`garrison.entry_distance_m`), it stands still for 2 s (`enter_exit_s`) and is then inside. Leaving takes the same time. While entering or leaving, the squad has no movement goal and its weapons report "changing position". A move or attack-move given to a squad inside makes it leave first. Stop during entry cancels it, and Stop during exit keeps the squad inside. An attack order given to a squad inside fires from the building and never walks out to chase. A leaving squad is placed, as a formation with standing room for every soldier, at the outside point nearest its next destination (or where it entered), searched every 2 m round the building. If there is none, it stays inside and tries again.
- **Why / the gap:** the contract gave the timers and asked for "deterministic free positions outside", but not how other orders interact or which positions.
- **The reach:** Garrison orders queue with Shift like any other. Leaving has no key: the command bar has a Leave building button. Entry and exit mirror each other.
- **Confidence:** medium.

### Building cover goes to a target seen at its slots
- **When:** slice 11.
- **The choice:** A round aimed at a spotted squad that is inside a building spreads wider by the building's cover multiplier. Blast fragments reach its soldiers with the building's lower chance. Where forest cover also applies, the stronger wins; they're never multiplied. Fire at a contact area or at the ground gets terrain cover only. A blast doesn't count the occupants' own building as a wall in the way, because the building is already their cover.
- **Why / the gap:** the contract says building cover applies "once", but not how a shooter knows its target is inside.
- **The reach:** garrisoned soldiers can only be seen at their slots, so "seen inside" is what the shooter observes. A soldier who is hit still takes full damage (P12).
- **Confidence:** medium.

### Only direct hits wear a building down
- **When:** slice 11.
- **The choice:** A round that strikes a building takes its structural damage off the building's health. A blast nearby doesn't. Rifles and the HMG do no structural damage. Building health lives with the garrison state, not the world, which only owns shapes.
- **Why / the gap:** L10 says only structural weapons damage buildings, but not whether blasts count.
- **The reach:** a tank's HE brings a 400 hp village building down in four hits on its walls.
- **Confidence:** medium.

### A collapse leaves a lower ruin, and survivors scramble out
- **When:** slice 11.
- **The choice:** At zero health, the building is replaced by a 2 m ruin of the same footprint. Each occupant survives with the fixture's collapse survival chance, rolled on the damage random stream. A survivor searches rings 1 m apart around its slot for the first point it can stand on and walk to in a straight line without crossing a solid or water, keeping at least 1 m from other survivors. A survivor with no such point dies where it stood. The squad regroups on the survivor nearest their middle, takes at least the collapse suppression level, and its soldiers walk back into formation. Squads still entering from outside are unharmed.
- **Why / the gap:** the contract gives the rules but not the search, the regrouping or what the scattered squad does next.
- **The reach:** the squad drifts back into shape over a few seconds. A side that sees the ruin stops drawing the building it replaced.
- **Confidence:** medium.

### Buildings are drawn apart from the ground mesh on the battle routes
- **When:** slice 11 (extended in slice 16).
- **The choice:** The battle view (village, its replay, endurance) and the garrison lab draw buildings separately from the static world mesh, so a building your side has seen fall can disappear. A known ruin is drawn as a 5 × 5 grid of rubble heaps, none taller than the ruin's collider. Garrisons show occupant pads, an arc for the entry or exit timer, and a red ring for a squad waiting for room. The other labs keep buildings inside the world mesh.
- **Why / the gap:** how garrisons and ruins look was delegated.
- **The reach:** every route where a building can fall.
- **Confidence:** medium.

### Right-click on a building garrisons it
- **When:** slice 11.
- **The choice:** A right-click whose camera ray first meets a building sends a garrison order for the selection, and Shift queues it. Leaving is the command bar's Leave building button.
- **Why / the gap:** the controls contract lists garrison among queueable orders but gives no gesture.
- **The reach:** every route, through the one shared pointer pick.
- **Confidence:** medium.

### Every eligible unit is served at once
- **When:** slice 13.
- **The choice:** A set-up truck serves every eligible unit in reach at the same time, each at the configured rates: 1 round/s, 2 hp/s, one soldier per 5 s. When an item completes, it's paid for in ascending unit id, so if stock runs short, the lower-numbered unit gets the last of it. An item is never part-paid; the unit waits with "no stock". A unit in reach of two trucks is served by the first set-up truck, in unit order, that can pay for its next item. Progress toward an item pauses, and isn't lost, while the unit is ineligible.
- **Why / the gap:** "round-robin one service quantum per eligible recipient" could mean one unit after another, or all at once.
- **The reach:** how fast a battered group recovers, and who comes first when stock is low.
- **Confidence:** medium.

### Replacement soldiers take a fallen soldier's place and a new id
- **When:** slice 13.
- **The choice:** A replacement is a new soldier, with a new id, standing in a fallen soldier's spot in the formation. The fallen soldier's record stays where it lies. A squad inside a building gets ammunition only, no replacements, because its seats are fixed per soldier. It shows as waiting ("in a building: no replacements"). A squad with nobody standing is never served.
- **Why / the gap:** the contract required new ids and kept corpses, but not where replacements stand or what happens in buildings.
- **The reach:** squad strength after resupply, and garrisons.
- **Confidence:** medium.

### What a weapon's ring shows
- **When:** slice 14.
- **The choice:** Above each of your units, a dark box holds one ring per weapon. A dashed amber arc shows reloading, and a solid cyan inner arc shows aiming. The middle shows rounds left of the loaded kind (AP or HE on the tank cannon, ∞ when unlimited). A caption names the weapon (CANNON, HMG, RIFLES, GREN, ATGM). An upper badge appears while guiding a missile, and a lower badge carries a glyph for why the weapon can't fire. Plain progress (aiming, reloading, firing) gets no badge, and finished timers disappear. Beyond 700 m of camera distance, rings stay only over selected units. The selection panel always gives everything in words: each weapon's reason, ammunition and timers, and the unit's strength (soldiers or hit points), how pinned a squad is, its building phase and timer, and its supply state.
- **Why / the gap:** U02 and U03 fixed rings, numbers and the guidance icon, but not how reasons, weapon identity, unit condition or zoom appear.
- **The reach:** the battle UI and every lab that lists units.
- **Confidence:** medium.

### Keys and right-click commands
- **When:** slice 14.
- **The choice:** Right-click on a spotted enemy attacks it. A and G arm attack-move and attack-ground for the next right-click on the ground, then go back to plain move; Escape disarms them. E switches the selection to "return fire only", or back to "fire at will" if all of it already holds fire. S stops. Keys are ignored while typing. The command bar shows every one of these, plus Deploy, Pack and Leave building. Garrison stays on right-clicking a building.
- **Why / the gap:** the contract named the keys but not how an armed mode ends or what E does to a mixed selection.
- **The reach:** every player command.
- **Confidence:** medium.

### What counts as holding the village
- **When:** slice 15.
- **The choice:** Each tick, the village counts as held when a living blue unit that isn't a supply truck is within 100 m of the centre, and no living red unit of any kind is, including a red squad blue can't see. Any contest resets the count to zero. 30 held seconds in a row is a capture. Blue with no living combat unit is a defeat. At 15 minutes the verdict turns "inconclusive", but judging continues, so play can still end in a capture.
- **Why / the gap:** encounter.md says "holds the zone uncontested for 30 seconds" without saying what contests it, whether a contest resets or pauses the clock, or whether recon counts.
- **The reach:** every village result. A hidden defender really does hold the ground; how much the player sees of that is "The village's hold progress is public".
- **Confidence:** medium.

### The defender's small decisions
- **When:** slice 15.
- **The choice:** Red's defender sends its garrison orders on the first tick, using the fixture's unit-to-building pairs. Each AT team makes one explicit attack, on the costliest tank its own sights see within 900 m (ties to the lower handle), and after that fires at will without being re-targeted. A tank below 35% health or a squad below half strength moves once to its fallback point by the shortest route. Supply trucks never retreat.
- **Why / the gap:** encounter.md gives the triggers but not ordering, ties or route policy.
- **The reach:** red's behaviour in every village battle.
- **Confidence:** medium.

### What the comparison scripts may react to
- **When:** slice 15.
- **The choice:** A script reads only blue's observation, plus what every player knows: the static map and the objective. The ambush scripts' "incoming fire" cue is a hit one of its tanks felt, or a fresh firing area within 400 m of a tank. The supported script keeps its tanks out of sight until the scout spots enemy armour and attacks it. It then shells the buildings in turn, once the armour is gone or has been fought for 60 s (90 s if none is found). It pushes in and rotates badly hurt units through supply and back. If the zone is still contested once it's inside, it shells whatever still stands, then sweeps. No script order is ever refused.
- **Why / the gap:** encounter.md names each script's intent, not its moves or what a legal cue is.
- **The reach:** the tactical comparison report.
- **Confidence:** medium.

### A saved village battle
- **When:** slice 15.
- **The choice:** "Save replay" downloads a JSON file holding the variant and the replay, and also keeps the last save in this browser, so `/replay/village` opens it straight away. Other files load through a file picker. A file for the other variant, or another fixture, is refused with the simulation's own mismatch message on the panel.
- **Why / the gap:** the spec asks for replay export and import but gives no file form.
- **The reach:** the village UI.
- **Confidence:** medium.

### The stress battle's shape
- **When:** slice 16.
- **The choice:** The stress battle is a seeded synthetic fight on a 3 × 2 km field, not the full 4 km map. Each side fields 100 units: 50 rifle squads, 20 tanks, 12 AT teams, 10 recon squads and 8 supply trucks. Every 2 minutes, groups of ten attack-move to seeded points in the middle. A fifth of each side waits at the rear and joins from the second wave. Trucks set up at the rear. At minute 30, one wave is instead everyone firing at the ground in the middle. There are no reinforcements, because that mechanic belongs to slice 21 and a lab-only copy would be a second version of it.
- **Why / the gap:** validation.md names the population, turnover and bursts, not a map or a script.
- **The reach:** every slice 16 number. The roster can't reach validation.md's rounds-per-second target at the fixture's fire rates.
- **Confidence:** medium.

### Budgets are reported, not asserted
- **When:** slice 16.
- **The choice:** The endurance scene fails only on broken contracts: a battle that fails, GPU allocations that don't return to baseline after resets, or page errors. Frame, tick and memory numbers against validation.md's targets go into the evidence and the verdict, not into pass/fail checks.
- **Why / the gap:** validation.md calls its targets "proposed" and revisable with evidence.
- **The reach:** the scene suite stays green on slower machines. A missed budget shows up in the verdict, not as a red test.
- **Confidence:** medium.

### Heard sounds are played and captioned on every battle route
- **When:** slice 16 (closing cleanup).
- **The choice:** The battle view (village, its replay, endurance) and the contacts lab share one sound hook. It plays each heard sound, once you switch sound on, panned by where the camera is looking now, and captions it. Captions fold repeats: the same sound (kind, direction, listener) keeps one row with a count and jumps to the top with its latest wording. At most 3 rows show, and a row expires 5 s after its sound was last heard.
- **Why / the gap:** contracts.md renders sound through cues with visible captions, but only the contacts lab had them. The spec gave no caption layout, and an unfolded list grew into six near-identical gunfire rows that pushed the panel down.
- **The reach:** every battle route. A burst of many different sounds shows only the newest three.
- **Confidence:** medium.

### The supply-waiting ring and its words
- **When:** slice 16 (closing cleanup).
- **The choice:** Under each unit in a truck's reach, a 10.5–12.5 m ring on a dark band shows its service. It is a full bright-cyan ring while being served, and four long cyan dashes while waiting. Waiting covers moving, firing, no stock, in a building, and truck not set up yet. The truck's own reach is a solid white ring once set up, and faint fine dashes before that. The gold dashes are the objective, so each dashed meaning looks different. The selection panel says "waiting for supply: <why>", and the supply lab's list and legend use the same words.
- **Why / the gap:** the spec gave no supply visuals. The battle view used to leave units in a building or near an unready truck unmarked, while the supply lab marked them. There is now one supply layer with the lab's rule.
- **The reach:** in the village, units near a truck that hasn't set up yet now show the waiting ring.
- **Confidence:** medium.

### Selected units and their destinations share a name tag
- **When:** slice 16 (closing cleanup).
- **The choice:** Each selected unit shows its name, the same one the panel uses, over its readout cluster. The same name sits just above its destination ring.
- **Why / the gap:** carried from slice 04 into slice 14. Two routes that start close together couldn't be told apart, and the spec gave no marker.
- **The reach:** only selected units get tags, which keeps a crowded map quiet.
- **Confidence:** medium.

### Lab fixtures have one registry and one scene each
- **When:** slice 01.
- **The choice:** One JSON file (`apps/battle-lab/src/fixtures.json`) lists each lab fixture: its id, route and description, plus `"build": "production"` for a timing fixture. The lab's router and the browser test runner both read it. A test pins the router's routes to it, and the runner refuses to start if any fixture lacks a scene file or any scene lacks a fixture. The runner starts its own dev server, so `bun run --cwd web scene -- <id>` works with nothing else running. Scenes share one helper file for driving a lab.
- **Why / the gap:** the spec asked for "one lab/scene registry" without saying what form it takes.
- **The reach:** every lab adds one JSON row and one scene file.
- **Confidence:** high.

### Browser checks run headless on the real GPU
- **When:** slice 01.
- **The choice:** The runner launches Chromium in Chrome's full headless mode with WebGPU switched on. On this Mac, that gets the real Apple Metal adapter, so screenshots and timings come from the real GPU rather than a software renderer.
- **Why / the gap:** the spec allowed a visible-window probe only if headless differed, and didn't say which headless mode or adapter to use.
- **The reach:** timings measure the real GPU. A machine without a hardware adapter fails the "hardware adapter" check instead of quietly falling back.
- **Confidence:** high.

### The scene owns and destroys every GPU buffer itself
- **When:** slice 01.
- **The choice:** Tearing down a TypeGPU root (0.12.5) doesn't free the buffers it created; 11 leaked per rebuild. The scene records every allocation it makes and destroys each one when disposed. The viewport counts live GPU resources, and scenes check the count returns to its starting value after resizes and rebuilds.
- **Why / the gap:** the spec required cleanup back to baseline but not how ownership works.
- **The reach:** every GPU resource the renderer makes.
- **Confidence:** high.

### One shader draws the world, the units and the overlays
- **When:** slice 01.
- **The choice:** The renderer has one shader. Each vertex carries a position, normal and colour, and each instance carries a placement (x, y, z, heading) and a tint. Units and props are drawn as instances, and the static world once with an identity placement. The shader has two pipelines: opaque, and translucent (blended, with no depth writes) for sheets such as water. The fog is a per-side bit field that the same shader reads to darken fogged ground. 4× MSAA smooths edges. Lighting is one sun plus ambient, with faces flipped toward the eye so hand-built shapes light correctly whatever their winding.
- **Why / the gap:** the drawing primitives and scene structure were delegated.
- **The reach:** every drawn thing goes through this one shader.
- **Confidence:** high.

### The camera block holds only what the scene reads
- **When:** slice 01.
- **The choice:** The camera data sent to the GPU is 40 floats: view-projection, its inverse, eye position, near plane and viewport size. The sibling game's 48 included fields only its old shaders used. A test checks the byte size against the shader's struct.
- **Why / the gap:** research.md said to port camera primitives without saying whether to keep unused fields.
- **The reach:** adding a field means changing one packer, the struct and the size test together.
- **Confidence:** high.

### Web dependencies are pinned to exact versions
- **When:** slice 01.
- **The choice:** `web/package.json` lists exact versions, such as `"typegpu": "0.12.5"` rather than `"^0.12.5"`, matching what the sibling's lockfile resolved. three.js, Tailwind and Radix were left out.
- **Why / the gap:** the spec said to keep the pinned versions without saying how.
- **The reach:** every upgrade is an explicit edit.
- **Confidence:** high.

### Walkability is judged per triangle
- **When:** slice 02.
- **The choice:** Whether ground units can stand somewhere depends on the slope of the ground triangle under them, and whether it's water. Each terrain triangle is tagged once, by asking at its centre point, which lies inside it. An earlier attempt tagged grid corners, which touch up to six triangles, and the colours blurred.
- **Why / the gap:** the spec fixed the 35° rule and the triangle layout, but not how walkability is sampled.
- **The reach:** the overlay and the route planner use the same per-triangle rule.
- **Confidence:** high.

### The replay fingerprint covers all carried battle state
- **When:** slice 03 (completed in slices 11 and 16).
- **The choice:** A digest is a fingerprint of the whole battle state. It is taken every tick and folds in every piece of state the simulation carries from one tick to the next: units and their orders (tagged by kind), soldier offsets, route progress, blockers, each weapon's lock, including whether it is working the shot, missiles in flight and their support, a side's pending fire, contacts with where their evidence came from, garrisons, building health and ruins, what each side knows, the random generators, and the referee. The lab's "Check replay" plays the worker's log back on the page and compares digests every tick. That one check proves same-build replay and that the worker and the page run identically. The page copy also really gives away transferred buffers, as a worker does.
- **Why / the gap:** the spec asked for "direct versus worker parity" without saying how, and a digest that skips any state can't catch drift in it.
- **The reach:** any new state must be folded into the digest. The two deliberate exceptions are the next two entries.
- **Confidence:** high.

### Command numbering stays out of the fingerprint
- **When:** slice 16 (closing cleanup).
- **The choice:** The digest leaves out command bookkeeping: each side's next expected sequence number and the lists of accepted and waiting commands.
- **Why / the gap:** live play accepts a command a tick before a replay lets it in, so including the counter would make live and replay digests differ on identical battles. The bookkeeping's effect on the battle, the orders it applied, is already in the digest.
- **The reach:** replay parity. A bug that only upset numbering, without changing any applied order, wouldn't show in the digest.
- **Confidence:** high.

### The defender's memory stays out of the fingerprint
- **When:** slice 15.
- **The choice:** Red's defender remembers which AT teams it has already sent to attack and which units have already fallen back. That memory isn't in the digest. Its whole effect is the ordinary commands it sent, which the replay carries. A replay runs with the defender switched off, as encounter.md asks, so the memory never exists there.
- **Why / the gap:** the digest rule says all battle state goes in, but the defender is a player stand-in that happens to run inside the battle loop.
- **The reach:** replay parity and every stored digest.
- **Confidence:** high.

### How a side learns of new obstacles
- **When:** slice 04 (extended in slices 05 and 11).
- **The choice:** Each side plans routes with the map's authored props plus the new obstacles it knows about. It learns a new obstacle, such as a wall dropped across a road or a wreck, when any fog cell under its footprint comes into view, or when one of its units comes within 2 m of it. A unit planning far away drives straight at an unseen new obstacle, learns it on arrival, and detours. A side that never sees or reaches it never learns it. The one exception is a wreck its own vehicle left, or one it watched an enemy vehicle leave, which it knows at once (see the choice on destroyed units).
- **Why / the gap:** the spec says remains enter a side's knowledge "when observed or physically encountered" without saying how near counts as encountered. Checking only a prop's centre missed large props, because a 24 m ruin hides the ground at its own middle.
- **The reach:** wrecks and dropped walls. Ruins are the one exception (next entry).
- **Confidence:** high.

### Every side plans round a ruin, seen or not
- **When:** slice 11.
- **The choice:** A side learns of a ruin as a known prop only when it sees part of it. But route planning always includes ruins. A ruin stands exactly where the building it replaced stood, so including it means an unseen collapse can never open a route through that footprint.
- **Why / the gap:** the contract says unseen changes must not alter a side's routes. Removing the old building from planning would have done exactly that.
- **The reach:** every route near a collapsed building.
- **Confidence:** high.

### A partly seen squad is reported where its visible soldiers stand
- **When:** slice 05.
- **The choice:** If three soldiers of an eight-soldier squad step out from behind a building, you see those three. The squad's reported position is their average, not its true centre, and its velocity comes from those observed positions.
- **Why / the gap:** the spec said to publish only observed soldiers, but not which position represents a partly seen unit.
- **The reach:** weapons aim at what is observed. Aiming at the hidden centre would leak information.
- **Confidence:** high.

### Your own overlays draw over the fog
- **When:** slice 06.
- **The choice:** Route lines, destination rings, contact discs and remembered obstacles draw at full brightness even over fogged ground. Terrain, units and props under fog are darkened. The overlays are your side's own knowledge, and a firing area is most useful exactly where you can't see.
- **Why / the gap:** the spec didn't say how overlays and fog combine.
- **The reach:** every overlay.
- **Confidence:** high.

### Turning vehicles are checked with a slightly generous box
- **When:** slice 07.
- **The choice:** Within each piece of a flight step, a turning vehicle's box is tested at its middle heading, grown by the farthest any corner moves in that piece. Pieces that register a hit are halved until the growth is under 1 mm. A hit can land up to 1 mm early, but a real hit is never missed.
- **Why / the gap:** the spec asked for conservative bounds and narrowed time of impact without a method.
- **The reach:** every hit on a vehicle.
- **Confidence:** high.

### One seeded random generator for all combat randomness
- **When:** slice 07.
- **The choice:** The random generator is SplitMix64, whose whole state is one 64-bit number, so it's easy to fold into the digest. Bell-curve samples use Box–Muller, with the ±3σ cut done by redrawing. Combat, damage and observation each get their own stream. Results repeat exactly within one build, which is the replay promise.
- **Why / the gap:** no generator existed.
- **The reach:** every random roll.
- **Confidence:** high.

### An attack-move that stops to shoot says "halted"
- **When:** slice 08.
- **The choice:** A unit on attack-move stops advancing while any of its weapons is aiming, reloading, turning or firing at something it can engage. Its movement state then reads `halted` rather than moving. Once it can't engage anything, for example when the target's last-seen area has faded, it moves on. A target it can't hurt or can't shoot at never halts it.
- **Why / the gap:** W16 said "stops for reachable targets" but not how stopping shows or which weapon states count.
- **The reach:** how attack-move feels, and the readouts.
- **Confidence:** high.

### Weapons are authored as named groups of ammunition kinds
- **When:** slice 08.
- **The choice:** In `game.json`, each weapon a unit carries is a record, a "mount". For example, the tank's `cannon` holds `tank_ap` and `tank_he`, which share one aim and one reload, and there is a separate HMG. A record also says whether it's a squad weapon and whether it sits on a turret, which turns at the fixture's turret rate. Weapon rows can be flagged anti-armour (never engages infantry) or armour-piercing (never fired at an area). The row flagged `default` is the unit's unlimited everyday gun.
- **Why / the gap:** the fixture listed weapons as loose strings such as "cannon: tank_ap | tank_he".
- **The reach:** everything that reads weapons: missiles, supply, readouts.
- **Confidence:** high.

### Deployment progress counts whole ticks
- **When:** slice 12.
- **The choice:** A deploying unit stores how many ticks of set-up it has done, from 0 to the full 450 (15 s at 30 Hz). Each tick moves it one step toward its target. The published progress is that count divided by 450. A truck 40% set up packs in exactly 180 ticks, and one half packed sets up again in exactly 225.
- **Why / the gap:** L01 asked for equal, reversible durations without saying how progress is stored. A growing fraction would pick up rounding error, and the two directions would differ by a tick.
- **The reach:** supply readiness and every duration check.
- **Confidence:** high.

### A truck waiting to pack neither drives nor turns
- **When:** slice 12.
- **The choice:** A unit with a route but some set-up left doesn't move or turn. It reports the movement state `packing`, and its "am I stuck?" watch is paused. It still plans its route, which is drawn. Progress is updated before movement each tick, so the unit moves on the tick packing finishes.
- **Why / the gap:** "translate only at zero" didn't say whether turning in place counts, or what the unit reports meanwhile.
- **The reach:** the panel can say why a truck with orders isn't moving. Hearing treats it as idle.
- **Confidence:** high.

### Deployment and supply numbers live in one service section; every finite round has a price
- **When:** slice 12 (completed in slice 13 and the closing cleanup).
- **The choice:** The fixture's `service` section holds the set-up time, the supply reach, each truck's stock, the service rates, and a stock price for each weapon row's rounds. Every weapon row with limited ammunition must have a price, or the battle refuses to set up, so no round is ever given away. Only the supply unit kind deploys; the kind maps to a set-up time the same way kinds map to speeds.
- **Why / the gap:** the numbers existed, but which unit kinds deploy wasn't stated as data. An earlier version treated unpriced rows as free, which L05 (no free supply) forbids.
- **The reach:** a later deploying kind, such as radar, adds one line. A new limited-ammunition weapon must be given a price.
- **Confidence:** high.

### Missile support is renewed from the last sensing
- **When:** slice 10.
- **The choice:** Each tick, before rounds fly, a launcher keeps control of its missile only if it stood still, is alive, and its own sensors spotted the target at the last sensing. Sensing runs after flight within a tick, so that means the previous tick's sight. Otherwise the missile is released for good: its steering point drops to the ground under its last position, and it flies on to it. Stop releases it through the same check.
- **Why / the gap:** the spec lists what releases support but not when in the tick it's judged.
- **The reach:** escapes are decided within a thirtieth of a second of sight being lost.
- **Confidence:** high.

### Two new launcher reasons: guiding, and no own sight
- **When:** slice 10.
- **The choice:** "Guiding" means a loaded missile is waiting while one is still in flight. "No own sight" means a launcher whose target only its team (say, a scout) can see. Neither names what blocks the view.
- **Why / the gap:** the listed reasons include "guiding" but nothing for the rule that a launcher needs its own sight.
- **The reach:** the readouts and the panel.
- **Confidence:** high.

### Occupants stand on slots just outside the walls, spread evenly
- **When:** slice 11.
- **The choice:** A garrisoned soldier stands on a slot 0.45 m outside a wall (`garrison.slot_standoff_m`). Its hit capsule, eyes and muzzle are all there. A building has one slot per soldier of capacity (16), split evenly over the four walls, with any extra going to the longer ones, and spaced evenly along each. An entering squad takes slots one wall at a time: the first slot on each wall, then the second, and so on. Two rifle squads therefore put two soldiers on every wall. The squad's own position becomes the building's centre, which is where its fire reports and sounds come from.
- **Why / the gap:** the contract said "exterior side of the facade", "just outside" and "distributed evenly", without distances or a layout.
- **The reach:** A round that misses a soldier meets the wall right behind. A round leaving through a facing wall can't re-enter its own building, so no collider is ever switched off. Every wall is watched from the moment a squad enters.
- **Confidence:** high.

### The ground an observer stands on is seen
- **When:** slice 11.
- **The choice:** The fog sweep marks the cell under each eye as seen. Its sight lines start one cell out, so a lone squad used to stand on a dark square.
- **Why / the gap:** slice 05 left an observer's own cell to its neighbours' sweeps without saying so.
- **The reach:** fog under isolated units. Spotting is unchanged, because it's judged per target.
- **Confidence:** high.

### Scenarios can start units worn and trucks with a set stock
- **When:** slice 13.
- **The choice:** A scenario can start a unit with lower vehicle health, soldiers already fallen (lying in formation), or rounds already spent, per weapon. A truck can start with any stock. Only this kind of authored set-up may be left out of a scenario; every numeric rule comes from the fixture with no defaults.
- **Why / the gap:** the supply lab needed "a damaged tank, depleted AT squad, casualty rifle squad, empty truck", which the scenario format couldn't express.
- **The reach:** labs, the village, and the stress battle's late state.
- **Confidence:** high.

### Shelling a building aims at its wall
- **When:** slice 15.
- **The choice:** A ground attack whose point lies inside a building aims 5 cm in front of where the line from the weapon's real muzzle meets that building's wall. Without this, HE aimed at a building's centre was refused as blocked by the building itself. Letting rounds ignore the target building would have let them pass through walls.
- **Why / the gap:** the attack-ground contract didn't say what a point inside a building means.
- **The reach:** every ground attack on a building, by a player or a script.
- **Confidence:** high.

### Timing verdicts run on a production build
- **When:** slice 16.
- **The choice:** A fixture marked `"build": "production"` (only `endurance`) is served from a production build; every other scene uses the dev server. React's development build records every changed prop in the browser's performance timeline, including million-float overlay meshes. That crashes the late state and would distort frame timings. Scenes that fast-forward thousands of ticks on the dev server clear those records as they go.
- **Why / the gap:** the harness had one kind of server, and the spec asks for honest timings.
- **The reach:** the scene runner and the endurance scene. A long hand-played session on the dev server can still run into React's limit.
- **Confidence:** high.

### Only speed-ups that change no outcome
- **When:** slice 16.
- **The choice:** Every optimisation shipped leaves every battle's digest unchanged: a height-only ground query, fog sight lines that stop once nothing further can be seen (and skip the foliage maths where there is no foliage), and spotting that lists the living enemy once per side instead of once per observer. Faster options that would change routes, timings, what a side knows or what is drawn are proposed in slice 16b and wait for you: planning once per group or on a per-tick budget, a tighter search heuristic, spreading the fog sweep over ticks, and sending remains as changes only.
- **Why / the gap:** the slice allows optimisations "preserving outcomes" and forbids silent gameplay changes.
- **The reach:** performance only; no behaviour changes.
- **Confidence:** high.

### One battle view and one session shell
- **When:** slice 16 (finished in the closing cleanup).
- **The choice:** One component (`BattleView`) is the played or replayed battle: world, units, overlays, readouts, sounds, selection and command bar. The village, its replay and the endurance route add only their own panel content. Underneath, one hook (`useBattleSession`) owns the static world, the worker session, the command path, the drawn units and the click and drag-select adapters for both the battle view and the labs. Labs build their overlays from the same per-concern layers the battle view combines. So every lab right-clicks a spotted enemy to attack it, and drag-select works on the battle routes. The overlay takes the supply reach and objective zone from the scenario it draws.
- **Why / the gap:** the spec didn't cover this. The village route and the labs had grown their own copies of the session, picking and overlays.
- **The reach:** every route. A fix to picking, overlays or the command path lands everywhere at once.
- **Confidence:** high.

## Superseded by battle-look (2026-09-26)

The fixed two-rank formation at 2.5 m, replacements stepping into a fallen soldier's formation slot, and "don't rigidly collide every soldier" are superseded by battle-look's movement lane ([`specs/done/battle-look/`](../done/battle-look/README.md)). There, soldiers are individual bodies with no set formation, resolve cover at the order, and walk their own lanes. See `specs/done/battle-look/movement-unknowns-map.html`.

The supply-waiting ring and its words are superseded too (2026-09-28): a truck's reach shows only while it is selected, no ring lies under the units in reach, and a unit in a set-up truck's reach says RESUPPLYING, SUPPLY FULL or CANNOT SUPPLY in its info panel. See "Supply is shown in context" and "Every unit has an info panel" in [`specs/done/battle-look/decisions.md`](../done/battle-look/decisions.md).
