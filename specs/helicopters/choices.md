# Helicopters: choices ledger (final)

This ledger lists the choices the implementing agent made where the plan said nothing. It was rewritten at closeout against the shipped code, not against the pass where each choice first landed. Choices that later passes overturned are gone. Duplicates are merged. Each "deferred to slice N" entry now states how it ended up. A choice that only restated a given in [decisions](decisions.md) (D1–D43) was dropped. If you want to change a decision, change it there.

The entries are grouped by verdict: **needs-user** (decide; each one has a provisional call already taken), **unsound** (redo, starting from the corrected decision), and **sound** (the architecture you now own). Within each group, the least confident entry comes first. Confidence means how sure the audit is that you would have made the same call.

**Review these first:** N1 (every helicopter sounds like a Little Bird), N2 (the airborne contact sign is capped in size on screen), and U1 (the last corner of a route can overshoot the goal and come back).

---

## Needs-user

### N1. Every helicopter is weight class `light`, so a Chinook sounds like a Little Bird
- **When:** slice 16 (the air sound rows came in 17b).
- **The choice:** every unit has a `weight_class`, and the class decides two things: what its wreck is, and which engine sound loop it plays. D3 says every downed helicopter leaves a `light_wreck`. The catalog loader also refuses a wreck that doesn't match its unit's weight class. So the agent gave all 19 helicopters `weight_class: "light"` in `fixtures/units/air/profiles.json`. The cost: slice 17b authored three rotor rows (`air_light`, `air_medium`, `air_heavy`), and a CH-47 Chinook and an MH-6 Little Bird both play `air_light`. The medium and heavy rows have no user. The alternative was medium for utility and armoured types and heavy for heavy types. That would sound right, but under today's rules it would also give a Chinook a heavy wreck, which contradicts D3.
- **The gap:** D3 fixed the wreck and the sound design wanted weight, and nothing said which one wins.
- **The reach:** any later rule that keys on weight class, such as pushing, ramming or which wreck is left, treats every helicopter as light.
- **Verdict:** needs-user. Provisional call: all light; the unused rows stay ready. To reverse it, reopen D3 (heavier helicopters leave heavier wrecks), or let an air unit name its sound class apart from its weight. Either way, one data edit per profile then gives each class its own sound.
- **Confidence:** low.

### N2. The airborne contact sign is held between 20 and 44 px of radius instead of covering the whole uncertainty area
- **When:** slice 13.
- **The choice:** suppose blue loses sight of a red helicopter, or only hears it. Blue then has a *contact*: a guess at where the helicopter is, with an uncertainty radius. On the ground that guess is drawn as a disc covering the whole area. Slice 02's spike carried that rule into the air. For the test helicopter the area is 21 m, which is about 480 px across at close zoom. An unprimed critique said a floating disc that size hid the ground it marked and read as a decal. The shipped sign follows the area but is clamped to a radius of 20–44 px (`presentation.contacts.air.min_radius_px` / `max_radius_px`), with the hatching rescaled to match. Up close, the sign is therefore *smaller* than the area the enemy might be in.
- **The gap:** slice 02 fixed the look but not how large the sign gets on screen.
- **The reach:** a player can't read the true uncertainty of an air contact from the sign when zoomed in. Picking still uses the full radius (N2 changes only drawing).
- **Verdict:** needs-user. This deviates from slice 02's note. Provisional call: bounded size. To reverse, raise `max_radius_px` in `fixtures/game.json`; no code changes.
- **Confidence:** low.

### N3. The closing scene's climax plays under fog
- **When:** slice 18.
- **The choice:** the closing lab (`/lab/air-closing`, encounter `fixtures/maps/air/encounters/closing.json`) is drawn from blue's point of view. Blue fields only the Apache. Once the BMP shoots it down, blue has no eyes left, so the fall and the wreck are drawn under fog hatching, and the fogged tree crowns read as grey smoke. That is honest: a side sees only what its units see. The alternative was a blue ground observer parked within sight of the wood, which would keep the climax in full colour at the cost of one more unit in the scene.
- **The gap:** D14 describes the beat but not who watches it.
- **The reach:** this is the D14 showcase, so it's the shot people will look at.
- **Verdict:** needs-user (taste). Provisional call: no observer, so the climax stays under fog. To reverse, add one blue unit to `closing.json`. Check first that the observer doesn't change the beat: it could become a target or a spotter.
- **Confidence:** low.

### N4. Door guns rest pointing forward along the cabin, so they barely read when idle
- **When:** slice 16.
- **The choice:** each transport carries one door gun (mount `door`, the `hmg` rig) on a pintle in the left door (`door_gun` in `packages/scene-assets/blender/aircraft_parts.py`). At rest its yaw is zero, so the barrel lies forward along the fuselage side. At workbench zoom, the UH-1Y's and UH-60M's guns read as thin rods on the skin, and the MH-6M's bench gun can't be seen at all. The alternatives were a stowed rest yaw pointing outward, or a heavier mount. Either would make the gun read as a gun while the helicopter is idle.
- **The gap:** nothing specified a rest pose for a turreted gun on an aircraft.
- **The reach:** it only matters until the gun fires, since firing swings it onto the target.
- **Verdict:** needs-user (taste). Provisional call: forward rest, as built. To reverse, give each door gun's rig a rest yaw in `door_gun`; it's art only.
- **Confidence:** low-medium.

### N5. The closing scene fields the real T-72B3 and BMP-2M, placed by hand rather than bought
- **When:** slice 18.
- **The choice:** D40 planned test stand-ins (`test_tank`, and `test_gun_jeep` as the "IFV") bought through the normal purchase commands. The shipped encounter places a roster `eastern_t_72_t_72b3_2016` and an `eastern_bmp_ifv_family_bmp_2m_berezhok` directly. Real art reads as D14 ("an IFV shoots it down"). The BMP-2M's gun is the same `autocannon` row as the jeep's, so the beat plays to the tick either way. Purchases exist only inside a skirmish, so a saved encounter can't buy anything.
- **The gap:** D40 assumed tests couldn't field roster units; by slice 18 they could.
- **The reach:** retuning either roster card (hp, armour, gun) retimes the scene. The native `air_closing` tests catch that by name.
- **Verdict:** needs-user (it changes the D40 given). Provisional call: roster units, placed. To reverse, swap the two `kind`s back to the test units in `closing.json`.
- **Confidence:** medium.

### N6. No recorded skirmish at a pinned seed: the lab alone is D14's proof
- **When:** slice 18.
- **The choice:** D40 also planned a recorded skirmish at a pinned seed, with the real Apache doing the D14 beat in a real AI battle. Two 8-minute probe battles produced a Tigr kill and no shoot-down, because the AI's tanks and IFVs come late in its rotation. A natural D14 needs four things in one battle: a tank and an IFV on the Apache's path, the IFV winning, and the wreck landing in trees. The agent stopped searching and let the deterministic closing lab stand as the proof.
- **The gap:** D40 assumed such a seed was cheap to find.
- **The reach:** nothing shows the D14 beat emerging from a real skirmish rather than a directed encounter.
- **Verdict:** needs-user (it changes a D40 given). Provisional call: lab only. To reverse, run a seed search in the background and record the first hit.
- **Confidence:** medium.

### N7. Over a roof, the aircraft's ground marker circles the house, and the drop line stops on the roof
- **When:** slice 12 (kept at its checkpoint, seen again in slice 18).
- **The choice:** an aircraft's ground marker (S7 below) is ground paint, and its drop line is depth-tested. So when a helicopter hovers over a house, the roof hides the marker except where the ring passes round the house's sides, and the line ends at the roof. Critiques read the ring as "the house is selected", and in slice 18 the Apache looked perched on a block. The alternative was to paint the marker onto whatever surface lies under the aircraft, roof included.
- **The gap:** the contract says the ring is ground paint, and ground paint lands only on painted ground layers.
- **The reach:** this affects every aircraft over every building.
- **Verdict:** needs-user (taste). It was shown at the slice 12 checkpoint with no reply. Provisional call: kept as built. To reverse, decide whether the marker should follow the roof (a renderer change to paint on roof layers) or stay on the ground.
- **Confidence:** medium.

### N8. A door gun can traverse all the way round, through its own cabin
- **When:** slice 16.
- **The choice:** the simulation has no traverse arc for any mount. A left-door gun told to fire at something on the right swings through the fuselage and fires. In a war film it would either refuse, or the pilot would turn the airframe. The alternative was a traverse arc on mounts, so a gun only fires inside its arc, or the airframe turns to bring the target into it, as hull-fixed weapons already make it do (S20).
- **The gap:** D28 made door guns turrets without limiting them.
- **The reach:** it affects every turret that sits on one side of a hull, which today means door guns.
- **Verdict:** needs-user (a new mechanic). Provisional call: full traverse. To reverse, add an arc to the mount row and route "outside arc" to the same `Reach.face` turn the hull-fixed rule uses.
- **Confidence:** medium.

---

## Unsound

### U1. An aircraft closing on its goal round a last corner can bow a few centimetres wide
- **When:** slices 03 → 07, re-checked at closeout (`crates/sim/src/movement/air.rs`, `crates/sim/src/navigation/air.rs`).
- **The choice:** an air route comes from A* over 4 m cells and is string-pulled to the corners the flight needs. When the goal lies just past a corner, the aircraft turns onto the final leg still carrying sideways momentum. Its acceleration-limited steering bleeds that off on the way in, so its path bows slightly wide of the goal before it settles.
- **What the closeout found:** the "metre past and back" the build recorded is gone after later flight fixes. The largest swing away from the goal, across twelve tower positions, measures 0.06 m. The test `flies_on_past_a_tower_without_turning_back` now asserts that the aircraft never turns back by more than 0.1 m, and that within 30 m of its goal it never swings more than 0.1 m away from it. Two attempts to remove the bow entirely failed: a step-crossing snap, and settling within 0.25 m. The bow happens about 1 m out, where the leftover sideways momentum is still bleeding off.
- **The reach:** invisible at 20 m up. A precise landing (transport, D13) will need an exact stop.
- **Verdict:** unsound, partly redone. **Corrected decision:** before transport needs a precise hover over a landing point, give the final leg a steering law that cancels sideways velocity first (or plans its braking from the turn into the leg), and tighten the test to no swing at all.
- **Confidence:** medium.

---

## Sound

### S1. An airborne contact that sits at the low hover lies on the ground but still can't be area-fired
- **When:** slices 05, 13, closeout.
- **The choice:** two rules meet here. Area fire means firing at a contact's ground point. The sim refuses it for any contact in the air layer, through one owner, `SideKnowledge::ground_contact` in `crates/sim/src/knowledge.rs`: an Attack order on such a contact fails with `OrderError::AirContact` ("AIR CONTACT"), and automatic area fire skips it. Since closeout, the cursor offers no attack on it either (`PointerPick.contactAirborne` in `web/src/battle/input/pointerIntent.ts`, set from `layer` in `apps/battle-lab/src/sideInstances.ts`). Separately, the sign *floats* only when the contact is `aloft` (S9). Take a helicopter heard while it sits at the low hover over its supply truck. Its sign lies on the ground, but you can't attack it. The alternative was to key the refusal on `aloft` too, so anything drawn on the ground can be shelled.
- **The gap:** D22 says "airborne" and D33 says "floats above the low hover"; the two lines differ by a few metres.
- **The reach:** a player may see a ground-drawn contact the cursor refuses.
- **Verdict:** sound. It is D22 read literally, and both sim and web agree. If the case shows up in play, the switch is a one-word change in both owners.
- **Confidence:** low-medium.

### S2. A gun's pitch limit and recoil live on the model's gun node, and the drawn gun can lag briefly
- **When:** slice 15, extended in slice 16 (`packages/scene-assets/src/articulation.ts`, `aircraft_parts.py`).
- **The choice:** a gun's elevation limits are custom properties on its pitching node in the exported model (`pitch_min_deg`, `pitch_max_deg`). The Apache's chin gun is -60..+11°. `articulate` clamps to them wherever the model is posed. A model that states none keeps the old global defaults (`DEFAULT_PITCH_LIMITS`), so no existing art had to be re-exported. In the same way, a node's `recoil_max_m` caps the shared 0.45 m recoil stroke: the Apache's chain gun used to slide 0.42 m back into its turret on every round. Side effect: the pose driver eases toward the *published* elevation without clamping, so when the sim asks for an angle past the limit, the drawn gun can hang a moment before it moves again. No scene showed it. The alternative was to put limits in the appearance catalog entry, which would be a second owner of "how the part moves".
- **The gap:** only global pitch limits existed.
- **The reach:** every future model states its own limits. The limits are drawing-only: the sim's rounds don't read them.
- **Verdict:** sound. Watch for the lag.
- **Confidence:** medium-low.

### S3. Sign stem and sign colours differ from the spike's picture
- **When:** slice 13 (`presentation.contacts.air` in `fixtures/game.json`).
- **The choice:** the floating sign got a thin pale stem down to the ground under it. The stem is 2 px, `stem_alpha` 0.8, a depth-tested segment, with no ground ring and no stem when seen from straight above. Without it, two critique rounds read the sign as lying on the ground. The alphas were also raised from spike D's 0.5/0.75: fill is now 0.55 and hatch 1. Up close, D's values read as a pink film, and 0.7 erased the hatching. The rim stays the shared grey-blue outline colour, not D's unlit white. The last critique still called the disc partly see-through and the rim weak.
- **The gap:** the spike fixed a still picture, not how the sign reads in a moving 3D view.
- **The reach:** it's all data. Any "too busy" or "too faint" note is a number in `game.json`.
- **Verdict:** sound.
- **Confidence:** medium-low.

### S4. First-pass helicopter weapon numbers
- **When:** slice 09 (`fixtures/game.json` weapon rows).
- **The choice:** `heli_atgm` extends the ground `atgm` with `stationary: false`, `guidance: "on_the_move"` (D38's property), 1200 m range and 8 rounds. `rocket_pod` extends `tank_he` as an unguided ground-only salvo weapon: 900 m, 450 m/s, 12 mrad scatter, salvos of 8 at 0.12 s, 38 rounds, damage 60, penetration 30, 7 m blast, structural 80. Resupply prices are 20 and 2 per round. The alternative was a guided rocket. These are placeholder values: balance was out of scope.
- **The gap:** D9 named the rows but not their numbers.
- **The reach:** every helicopter that carries them inherits these numbers until the balance spec.
- **Verdict:** sound (the user said "not clearly broken" is the bar).
- **Confidence:** medium-low.

### S5. Mounts: one per weapon kind, so a helicopter fires every rocket from one pod
- **When:** slices 15–16 (unit cards; art in `aircraft_parts.py`).
- **The choice:** a mount is one aimed weapon position, and a weapon row's `ammo` is its whole load. The Apache has three mounts: `chin` (autocannon, turret), `rockets` (`rocket_pod`, hull-fixed, muzzle at the left inboard pod) and `missiles` (`heli_atgm`, hull-fixed, right outboard rack). So all 38 rockets leave the left pod. The roster follows the same rule. Missiles come off the right outer station. Rockets on the AH-6M come from its right launcher, and on the Tiger UHT from its left inboard launcher. A new blunt `rockets` store kind replaced the Tiger UHT's sensor-pod stores. Labels are "Chain Gun", "AT Missile", "Rockets" and "HMG". The alternatives were a mount per side, which would split the ammo and need two muzzles, or one combined pylon mount, which couldn't fire rockets and missiles at the same time.
- **The gap:** the cards list one weapon row per store type.
- **The reach:** symmetric launch from both sides would need a muzzle-alternation feature, not a second mount.
- **Verdict:** sound.
- **Confidence:** medium.

### S6. The air grid: 4 m cells, walls widened by the largest air hull, built from what the side knows
- **When:** slice 03, limits raised in slices 15–16 (`crates/sim/src/navigation/air.rs`, `hull_limits.air` in `fixtures/game.json`).
- **The choice:** each side keeps one 2D air grid of 4 m cells. Its only walls are footprints whose roof is over 30 m (D21). Every wall is widened by `hull_limits.air.half_length_m`, so one grid serves every airframe. That limit grew to 9.94 m (the Merlin's art), and half width to 3.65 m (the Ka-52 over its wing-tip pods). So a Little Bird now keeps almost 10 m off a tower, like a Merlin. A route is a straight line when that line is clear. Otherwise it's an 8-neighbour A* with no corner cutting, then string-pulled. The grid reads the side's *beliefs* about standing props (D32), so a tower destroyed out of sight still walls routes. That holds by construction, and no test covers it, because no destructible test prop is over 30 m. The closeout deleted a cell-expansion counter that had been kept for a bound test nobody wrote. The alternatives were a grid per airframe size (a wall margin fitted to each hull) or a visibility graph.
- **The gap:** D21 said "simple grid" and nothing more.
- **The reach:** a bigger future airframe widens every aircraft's margin. The search runs only when a tower is in the way.
- **Verdict:** sound.
- **Confidence:** medium.

### S7. The aircraft's ground marker is the vehicle's own unit marker, always shown, joined by a 4 px drop line
- **When:** slice 12 (`buildAircraftMarks` in `packages/battle-renderer/src/orderOverlay.ts`; `orders.drop_line_*` in `fixtures/game.json`).
- **The choice:** to show where an aircraft is over the ground (D18), every own or identified-enemy aircraft gets the existing vehicle unit marker painted on the ground beneath it: a circle with a facing arrowhead. It's shown all the time, not only when selected, because at 20 m up the ground position is unreadable at any moment. Colours follow the ground vehicles' rules: orders' colour at `current_alpha` when idle, full order colour while an order moves it, selection colour when selected, `hud.enemy` for enemies. A drop line runs from the airframe's foot to the marker's centre. Its width is `drop_line_px` 4 under the shared stroke rule, at its own opacity of 1 rather than the marker's 0.55, which went faint over pale roads. No line is drawn under 0.5 m. The alternative was a new ring or glyph just for aircraft.
- **The gap:** D18 asked for "a drop line plus a ground ring", designed through game-ui.
- **The reach:** one marker concept for every vehicle. A critique called 4 px heavy at map zoom; the fix for that is `drop_line_alpha`, not the stroke rule.
- **Verdict:** sound.
- **Confidence:** medium.

### S8. Ghosts of aircraft fly at cruise height; with Space held, the end-of-orders ghost gets a drop line
- **When:** slice 12 (`ghostAloft` in `apps/battle-lab/src/unitGhosts.ts`).
- **The choice:** a ghost is the faint preview of where a unit will be: the end of its orders, a right-drag move, a purchase placement. An aircraft's ghost now stands `air.cruise_agl_m` above the ground, not landed. With Space held (the same condition that shows ghosts), a drop line joins the end-of-orders ghost to its ground marker. The first critique's top finding was that the ghost floated unanchored. The right-drag preview and purchase-placement ghosts are lifted but carry no drop line.
- **The gap:** the slice asked for lifted ghosts; anchoring them was a critique outcome.
- **The reach:** two ghost kinds still float unanchored.
- **Verdict:** sound.
- **Confidence:** medium.

### S9. Contacts carry a height, and the sim decides whether a contact floats (`aloft`)
- **When:** slices 05 and 13 (`crates/sim/src/knowledge.rs`, `ApproximateContact` in `crates/contract/src/observation.rs`, `web/src/battle/input/contactPick.ts`).
- **The choice:** a contact's `z` is the height of the evidence that made it. For a lost sighting, that's where the track was last seen. For a firing report, it's the shooter's height, kept even when the report's centre is scattered across the ground. Its `layer` is the unit's band (`LowAir` for any aircraft). On top of that, the sim publishes `aloft`: in the air band and more than `air::low_hover` above the ground under its centre (D33). The web reads `aloft` for three things: whether the glyph floats, where its panel hangs, and how the pointer picks it (a ray through a sphere of the contact's radius when aloft, the ground disc otherwise). The alternative was for each web consumer to test `layer` and height itself. That would need a TypeScript copy of the low-hover height, which Rust derives from the catalog.
- **The gap:** D33 named the threshold but not its owner.
- **The reach:** a published contact field. Ground fire never reads `z` (it aims at the terrain under the centre), so ground behaviour is unchanged.
- **Verdict:** sound.
- **Confidence:** medium.

### S10. How the closing beat is directed
- **When:** slice 18 (`fixtures/maps/air/encounters/closing.json`).
- **The choice:** the D14 beat is a fly-in, a pop-up over the village, a tank kill, the IFV's shoot-down, and a wreck in the trees. It's produced by two moves and two engagement settings, with no starting damage. The tank holds fire unless fired on. At tick 300 the Apache is set to return fire only and sent on to (455, 465), over the wood, so the hidden BMP fires first and the Apache is still flying when hit. It carries about 60 m into the trees. The brief suggested starting the Apache at hp 90 so one round downs it. But that's below the 50% smoking share, so it trailed smoke from the first frame. A goal past the wood landed the wreck 5 m outside it, and a goal beyond the edge landed it off the map.
- **The gap:** D14 describes the beat; making it happen deterministically was left open.
- **The reach:** the beat depends on the BMP-2M's autocannon numbers and on the Apache's hp. The `air_closing` tests name each beat.
- **Verdict:** sound.
- **Confidence:** medium.

### S11. The lab scenes are directed with scripted engagement settings
- **When:** slices 07, 10, 13, 15 (`fixtures/maps/air/encounters/*.json`).
- **The choice:** each lab stages its claim with orders and engagement settings rather than special code.
  - `flight` (`/lab/air`): the gun jeep is set to fire at will at tick 540 and back to return fire at 590. Its one hit halves the helicopter, which keeps hovering. The tank stands 440 m off: inside its own gun's range, outside the helicopter's HMG range, and its gun never fires.
  - `crash`: the test helicopter starts at `hp: 1`, so the jeep's first hit downs it in frame.
  - `apache`: three test tanks instead of a jeep. One rocket killed the jeep before the chin gun's first round.
  - `lost`: the red helicopter is scripted behind the corner shop, with the jeep 0.8 m from its wall. A 12.65 m roof hides a 20 m airframe only from that close, and the 22 m block's roof would stand above the sign.

  The alternatives were starting damage, extra helicopter health, or more map.
- **The gap:** the slices named what each scene must show, not how to stage it.
- **The reach:** retuning the test units retimes these scenes.
- **Verdict:** sound.
- **Confidence:** medium.

### S12. The crash: a digested list of falling airframes with a fixed little physics
- **When:** slice 04, closeout (`crates/sim/src/crash.rs`, `Battle::land` in `crates/sim/src/battle.rs`).
- **The choice:** when a helicopter dies, its unit is replaced by a `Crash` in `Battle.crashes`. The crash keeps the airframe's momentum and falls under the flight config's gravity. It spins at π rad/s, with the direction set by unit id. It glances off immovable bodies at 0.3 restitution, slides off a roof at 4 m/s, and is clamped inside the map edges (closeout). On impact it bursts through `damage::detonate`, which shares its casualty code with ordinary hits. The wreck then rests at the nearest clear spot on the map, searched ring by ring out to 30 m. The crash is folded into the battle digest only while falling, so battles without one keep their digests. Since closeout the digest also includes who shot it down and which sides saw it go down. The falling airframe is not a body, so rounds pass through it (D31). Credit to the shooter holds by construction, since the crash carries the death's source into the blast. No test asserts it, and no test covers the roof slide.
- **The gap:** D3 and D31 described the image; every number and the falling model are the agent's.
- **The reach:** this is the one owner of a falling airframe. Publication (S13) and drawing read it.
- **Verdict:** sound.
- **Confidence:** medium.

### S13. Witnesses of a crash learn the trees it felled
- **When:** slice 18 (`Battle::land`).
- **The choice:** this is a knowledge rule. A side learns the state of the world only through what its units see. In the closing scene the Apache was blue's only unit, so after it died blue saw its wreck (already a rule) but drew the trees it had flattened as still standing. Now the props a crash newly fells are marked seen-fallen for every side that saw the airframe go down, as the wreck is. The alternative was to leave fallen trees to sight alone, or to add a blue observer (N3).
- **The gap:** D3 said the wreck fells trees; nothing said who knows.
- **The reach:** this is a general rule about knowledge. It can move the digest only for crash landings among toppling props.
- **Verdict:** sound. It is the same knowledge as the wreck's, and keeps D14's "the wreck flattens trees" visible.
- **Confidence:** medium.

### S14. The resupply sink: when it holds, how fast, and how low
- **When:** slice 14 (`sinks`, `low_hover`, `height_target` in `crates/sim/src/movement/air.rs`).
- **The choice:** an idle helicopter in a deployed supply truck's zone sinks to the low hover (D10). "Idle" means no orders at all, no engaged weapon, and *last* tick's supply status Serving, NoStock or Full. Supply runs after movement, so a fresh order or engagement lifts it the same tick, before the status catches up. The sink rate equals the type's climb rate. At the low hover it still clears anything under it by 2 m: a roof within its hull's half length, or forest canopy, lifts the hover target. That case is untested. The 2 m is the code constant `LOW_HOVER_CLEARANCE_M`, because D27 fixes it. `low_hover(rules)` rescans the catalog (tens of types) each time it's needed, without a cache. The alternatives were "no movement goal" as the idle test (an Attack on a target in reach has no goal, so it would stay low), a slower separate sink rate, and a cached height.
- **The gap:** D10 and D27 set the rule but not its edge cases.
- **The reach:** transport (D13) will land at the same height through the same function.
- **Verdict:** sound.
- **Confidence:** medium.

### S15. The flight model
- **When:** slice 03, fixed in slice 07 (`step_aircraft`, `turn`, `separation` in `crates/sim/src/movement/air.rs`).
- **The choice:** an aircraft has a ground velocity that eases toward the speed it wants, at `cruise / drive.acceleration_s` (the ground vehicles' acceleration time). It takes each corner no faster than `cruise × cos(turn angle)` and no faster than it could stop from in the rest of the route, braking on 80% of its acceleration. A waypoint on the way counts as passed within 1 m, or once the aircraft is beyond it along the leg that led there. The slice 07 lab found it skimming a tower's corner 1–2 m wide and flying 130 m on before turning back. Overlapping aircraft drift apart at up to 4 m/s. Above 1 m/s it faces where it flies; below that, its ordered facing, at the type's turn rate. The alternatives were a full aerodynamic model (out of scope) or a wider waypoint radius (which only moves the miss).
- **The gap:** D4, D6 and D7 gave the feel, not the model.
- **The reach:** every aircraft flies through it. The aircraft-only constants are at the top of the file.
- **Verdict:** sound. U1 is its one remaining defect.
- **Confidence:** medium.

### S16. The skirmish AI buys helicopters only as reinforcements, and they never count toward holding a flag
- **When:** slices 15, 17, closeout (`crates/sim/src/skirmish_ai.rs`, `objectives::can_hold` in `crates/sim/src/objectives.rs`, `fixtures/units/roles.json`).
- **The choice:** every roster helicopter carries a new `helicopter` role (symbol: rotary wing). The AI's reinforcement rotation gained it as a fifth role, after infantry, recon, AT and light vehicle. A deck without a helicopter buys the next role in turn, so other decks buy as before. The opening is never a helicopter: it stays a ground screen, as in WARNO. A bought helicopter is ordered to an objective like any combat purchase. Since closeout, when the AI weighs how many of its units already head to each objective, only units that can hold one count. That's one owner, `objectives::can_hold`, shared with capture itself (D20). The test helicopter keeps `light_vehicle`. The alternative was buying one in the opening, which would change every US and Europe skirmish.
- **The gap:** D12/D35 said the AI buys and flies them, not when.
- **The reach:** helicopters appear only mid-battle in AI skirmishes. There's no return-to-base code (D16).
- **Verdict:** sound.
- **Confidence:** medium.

### S17. The rotor sound plays in the vehicle row's engine slot, through a replaced effect
- **When:** slice 17b (`air_*` rows and `effects.rotor` in `fixtures/game.json`, `fixtures/sounds.json`).
- **The choice:** a vehicle sound row has an `engine` loop, which always plays at least at idle level, and a `running` loop that follows rolled ground travel and is silent under 2% load. The rotor went in `engine`, and `running` is null. The slice text said `running`, but an aircraft never adds ground travel, so the rotor would have been silent even in flight. Idle and load are set equal, as a governed rotor is. The recording reaches the row through a synthesized `rotor` baseline that `recorded-rotor-uh60` replaces, the way `tracks` and `motor` are replaced. A row naming the recorded recipe directly would never be prepared, since a battle prepares only baselines and their replacements.
- **The gap:** the slice named the wrong slot, and the audio pipeline's rule about baselines wasn't in the plan.
- **The reach:** the sound workbench offers the rotor under Defaults & effects. Any future aircraft sound follows the same path.
- **Verdict:** sound.
- **Confidence:** medium.

### S18. Rotor sound rows: the loop crop and three weight classes
- **When:** slice 17b.
- **The choice:** the loop is cut from 19.8–22.8 s of the UH-60 recording. Only the 18–23 s stretch carries the blade beat, 17.2 Hz (4 blades × 258 rpm); earlier stretches sound like rushing air. The cut is exactly 50 blade passes, so the seam keeps the beat. There are rows for `air_light`, `air_medium` and `air_heavy`: gain 0.45/0.55/0.65, rate 1.1/1.0/0.9, no turret whine, no reverse beeper. They reuse the `running-gear` processing (high-pass 50 Hz, low-pass 5 kHz). The alternatives were the 0.2–3.9 s stretch, a single `air_light` row, or a new processing profile.
- **The gap:** D23 named the recording, not the cut or the rows.
- **The reach:** because of N1, only `air_light` is heard today.
- **Verdict:** sound.
- **Confidence:** medium.

### S19. Air profiles: shared data for the five helicopter classes
- **When:** slices 15–16 (`fixtures/units/air/profiles.json`, a new game root in `sim::fixtures`).
- **The choice:** `roster_profile_{light,utility,heavy,attack,armoured}_helicopter` each state D5's hp and armour (ricochet 0.1), D26's cruise speed, the starter-balance sight (700/700/700/900/850 m) with a 1/0.6/0.3 shape, no push, and `light_wreck`. Turn and climb are light 100°/s and 9 m/s, utility 70/7, heavy 50/6, attack 80/8, armoured 70/7. Loudness is light 800 m, heavy 1000 m, the rest 900 m. Eye heights follow each canopy, from 1.4 m (Little Birds) to 2.2 m (Chinook, Ka-52). The Apache started inline and now extends the attack profile, resolving exactly as before. The alternatives were inline rows on all 19 cards, or putting them in the ground profiles file.
- **The gap:** D5 and D26 covered hp, armour and speed; turn, climb, loudness and eyes were open.
- **The reach:** these are first-pass values that only order the classes (a Little Bird turns faster and is quieter than a Chinook). The balance spec owns them.
- **Verdict:** sound.
- **Confidence:** medium.

### S20. "Hull-fixed" guns turn the whole body, and keep it on target through every burst
- **When:** slice 08, closeout (`hull_fixed`, `Reach.face` in `crates/sim/src/weapons.rs`; consumers in `movement/air.rs` and `movement/mod.rs`).
- **The choice:** a mount is hull-fixed when it's on a vehicle, isn't a turret, and rides none. Its bearing is the body's yaw. While it has a target it can fire on, it asks the body to face it through `Reach.face`. Since closeout that request holds through every aim, burst and reload, not only while turning onto the target, so a strafing gunship no longer swings back between bursts. An aircraft turns to that bearing ahead of its travel heading. A tracked vehicle pivots only when it has no orders, and a wheeled one holds fire (D8, D42). While it's off target, the mount reports the existing `TurretTraversing` reason. `face` enters the digest only when set, so units without hull guns keep their digests. The alternative was a new action reason, which the player would read the same way.
- **The gap:** D8 set the rule but not the mechanism or its timing.
- **The reach:** every future fixed-gun vehicle uses this path.
- **Verdict:** sound.
- **Confidence:** medium.

### S21. `weapons::reaches` is the one reader of `targets`; the hold-fire distance stays flat
- **When:** slice 07 (`crates/sim/src/weapons.rs`).
- **The choice:** `targets` lists the altitude layers a weapon can hit (D1). Exactly one function reads it, and it gates which kinds a mount uses, which explicit orders are valid, the return-fire threat ranking, and the area-fire hold, which now counts only enemies the weapon can reach. Actual fire was already 3D (D22). But the hold's "is an enemy in reach" distance stays horizontal. The alternative, 3D there too, differs by under 1 m at 20 m height and would have shifted ground replays on slopes.
- **The gap:** D22 said engagement is 3D, without naming every check.
- **The reach:** a future high-air layer goes through the same function.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S22. Rotor spin is metres swept by the blade tip, at a drawing speed of 45 m/s
- **When:** slice 06 (`presentation.pose.rotor.tip_mps` in `fixtures/game.json`; `packages/battle-renderer/src/models/poseDriver.ts`).
- **The choice:** all rotors spin from one articulation input, `rotor`: the distance every blade tip has swept, which grows at 45 m/s while the unit is airborne. Each `rotor_*` node turns by that distance over its own radius, as a wheel turns by distance over its radius. So every tip runs at one speed and small tail rotors turn faster, with no per-rotor authoring. A real tip runs about 220 m/s. 45 keeps the tail rotor under a quarter turn per 60 Hz frame (no backwards strobing) and the main rotor near 60 rpm, so you can see it turn. There is no blade blur. The alternatives were an angle per rotor, or a gear ratio on the tail rotor.
- **The gap:** nothing said how rotors move.
- **The reach:** one renderer-only number to retune.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S23. Hull boxes are measured from the art, under the rotor head
- **When:** slices 15–16 (each exporter's `AFT_M` and `PUBLISHED_M`).
- **The choice:** a hull box is the sim's shape for a unit: what blocks rounds and what gives cover. Each helicopter's box is fitted to its exported airframe, not its published dimensions, and rotors are left out (S24). So the box stops under the rotor head: the AH-1Z's is 3.45 m tall, not the published 3.76 m. Airframes drawn off-centre were shifted forward to sit centred on their box: the Little Birds by 1.15 m, the rest by 0.28 m or less, the Apache by 0.3 m. Mount pivots moved with them. Published figures stay in the exporters as the size the art is drawn to. The alternatives were raising the fit tolerance, or boxes at published size, which would give cover where there's no airframe.
- **The gap:** the hull fit check refuses boxes that don't match their art.
- **The reach:** this is the rule for every future airframe: the art decides the box.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S24. Hull fit ignores rotors; posed bounds treat a rotor as its disc
- **When:** slice 06 (`isRotor` in `packages/scene-assets/src/articulation.ts`, `validate.ts`).
- **The choice:** every node named `rotor_*` or `blade_*`, and everything under it, is left out of the hull fit. A rotor isn't solid, and the box shouldn't grow to its sweep. For culling and framing, each rotor's bounding box widens to the square around its disc, in closed form, the way a wheel's does. The alternative was to sample rotor angles in `sweepArticulations`, which would add a sampled sweep and padding.
- **The gap:** none; the fit rules predated rotors.
- **The reach:** a rotor never blocks a round, and a rotor sticking out of its box (as on the Mi-35M's tail) isn't a defect.
- **Verdict:** sound.
- **Confidence:** high.

### S25. Wrecks throw every main blade clear; one owner for rotorcraft guns and wrecks
- **When:** slices 06, 15, 16 (`chin_gun`, `door_gun`, `rotorcraft_crash` in `packages/scene-assets/blender/aircraft_parts.py`).
- **The choice:** a helicopter wreck snaps every lifting-rotor blade off and lays it thrown clear as the wreck's `debris`, which sinks away and is never cover. The guns' rig nodes are renamed so no turret piece is thrown. Each rotor's blades land in their own stretch, so the Chinook's and Ka-52's two rotors don't overlap. Wrecks get a 1.5 m footprint tolerance (the Ka-52 gets 1.6 m, because a shared rubble plate passes its 7.3 m width by 3 mm). The UH-60M's and Z-20's tails slew 0.35 rad instead of 0.45 to stay inside. One module owns this for all 19 airframes, and the Apache's art re-exported byte-identical onto it. The alternative was drooping blades left on the wreck, which reach 5.7–6.8 m sideways past any box.
- **The gap:** D41 said to reuse the existing wreck art; the source wrecks didn't fit the box rule.
- **The reach:** every new rotorcraft calls these three functions.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S26. Where chin guns aim, and which guns are not chin guns
- **When:** slice 16.
- **The choice:** chin guns (`chin`, the `gun` rig, autocannon) on the AH-1Z, Tiger HAD, Mi-35M, Mi-28NM and Z-10 each carry their own elevation limits (S2): AH-1Z −50..+18°, Tiger HAD −28..+28°, Mi-35M and Mi-28NM −40..+13°, Z-10 −50..+12°. The last three are approximate, since their source libraries give no figure. The Ka-52's cannon and the AH-6M's guns are hull-fixed, not turrets: the Ka-52's 2A42 traverses only a few degrees, and the Little Bird's guns are fixed to its planks, so a turret would swing them through the fuselage. The airframe turns to aim them (S20). The alternative was one limit for every chin gun, and turrets for all guns as D28's wording might suggest.
- **The gap:** D28 assumed every gun was a chin gun.
- **The reach:** the limits are drawing-only. Being hull-fixed changes how the Ka-52 and AH-6M fight: they must turn to shoot.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S27. Door guns: one per transport, in the left door
- **When:** slice 16.
- **The choice:** the cards list one HMG per transport, so each gets one door gun (`door`, the `hmg` rig) on a pintle in the left (+Y) door, elevation −60..+20°. US and Europe types get M2s, and the Mi-8 and Z-20 get Kords. The UH-1Y's and UH-60M's modelled right-door guns stay as stowed art. The Mi-8's outrigger pods stay art: its card fields only the gun. The MH-6M's gun sits at the front of its left bench. The alternative was a gun per side, which would split ammo across two mounts.
- **The gap:** the cards gave a weapon count, not a position.
- **The reach:** every transport reads the same. See N4 (rest pose) and N8 (traverse).
- **Verdict:** sound.
- **Confidence:** medium-high.

### S28. A hull-fixed weapon is drawn by the hull, with no rig
- **When:** slice 15 (`MountRole` `"hull"`, `hullFixed` in `packages/scene-assets/src/units.ts`; `validate.ts`; `vehicle_export.rig`).
- **The choice:** a rig is the set of articulated nodes that let a drawn turret or gun turn and pitch, and only two exist (`gun`, `hmg`). A hull-fixed weapon never articulates, so it's drawn by the hull itself. It needs no rig and no entry in the appearance's `mounts`, and its muzzle flash appears at the published muzzle, turning only with the body. The fit check still refuses an undeclared *turning* mount. The alternative was a third rig kind for rockets and missiles, which would add nodes that never move. Not checked: that the art actually puts a pod at each hull-fixed muzzle, since no fit check exists for that.
- **The gap:** the Apache had three mounts and the rig system had two kinds.
- **The reach:** all rocket and missile carriers, and the Ka-52 and AH-6M guns, use this path.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S29. A falling airframe is published to each side under the name that side knew it by
- **When:** slice 10 (`FallingAirframe` in `crates/contract/src/observation.rs`; `crashCount` header word, `crashes` group).
- **The choice:** the observation every side receives gained a `crashes` group: `id, own, kind, x, y, z, yaw, pitch, roll`. For its own side, `id` is the unit id. For the enemy, it's the identified handle that side had for it as it died, stored in `Crash.knowing` where each side's sighting of the death is recorded. So the renderer continues the same drawn vehicle into the fall: same rotor phase, no pop. `kind` was added beyond the slice's field list so a side can draw the airframe without remembering it. The alternatives were the raw unit id for everyone, which leaks identity, or no `kind`.
- **The gap:** the slice listed fields; identity under fog was open.
- **The reach:** a publication contract that the web decoder and every replay read.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S30. The fall's tilt is derived in the sim and drawn through the one posing path
- **When:** slice 10 (`Crash::attitude` in `crates/sim/src/crash.rs`; `web/src/battle/present/interpolate.ts`; `poseDriver.ts`).
- **The choice:** the nose drops and the body leans into its spin in proportion to how fast it's sinking, up to 0.35 rad pitch and 0.25 rad roll at 15 m/s of sink. It's level at the moment of death. `Crash::attitude` computes this from digested state, so the attitude adds nothing to the digest. On the web, the falling airframe flows through the same interpolator and pose driver as a live vehicle (rotor still turning), with the tilt applied as a rigid motion about its foot. A tilted model is never drawn as a flat impostor card. The alternatives were zero tilt (an upright spinning airframe reads as a lift), a renderer-side guess (a second owner), or a separate crash-model path.
- **The gap:** the slice published pitch and roll without saying where they come from.
- **The reach:** the sim stays the only authority on the fall.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S31. Only aircraft smoke when hurt, and the sim owns the rule
- **When:** slice 11 (`Unit::smoking` in `crates/sim/src/units.rs`).
- **The choice:** a unit smokes when it is an aircraft, alive, and under `SMOKING_HP_SHARE` (0.5) of its hull HP (D34). Both the identified-enemy row (one coarse bit, never HP) and the own-unit row publish that same bit, so the web reads one bit for both. A falling airframe always smokes, keyed as it was alive, so the trail runs straight into the fall. The alternatives were for the web to derive own smoking from HP, which needs a TypeScript copy of the 50% threshold, or for every damaged hull to smoke.
- **The gap:** the slice's wording put own smoking on the web.
- **The reach:** extending smoke to ground vehicles is a one-line change at the owner, if you ask for it.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S32. The smoke trail is timed puffs
- **When:** slice 11 (`noteSmokers` in `packages/battle-renderer/src/effects/effectFrame.ts`; `damage_smoke` in `fixtures/game.json`).
- **The choice:** a smoker emits a puff every 1/20 s of presentation time, at fixed seeded birth times. Each puff is born where the hull was at that instant within its tick, from the hull's top. Moving, the puffs string out behind it. Hovering, they stack into a thin column. Albedo is 0.08 and opacity 0.45, growing 0.8 → 4.5 m over 2.5 s. At 8 Hz the trail looked like beads. The alternatives were emission by distance, as dust works (a hovering helicopter wouldn't smoke), or a ribbon like a missile trail.
- **The gap:** D19 asked for minimal smoke.
- **The reach:** it reuses the existing puff machinery.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S33. The airborne sign is the same contact glyph, turned to face the camera
- **When:** slice 13 (`buildContactGlyphs` in `packages/battle-renderer/src/contactGlyph.ts`; `screenAxes` in `packages/renderer-core/src/camera3d.ts`; `apps/battle-lab/src/contactFacing.ts`).
- **The choice:** the floating sign is a camera-facing pose of the one contact glyph, built from a ground pen and a facing pen. It's lit with the ground's normal so its red matches the ground glyph's. It follows the camera in 2° steps. The overlay is rebuilt every published tick anyway, so rebuilding it at a step costs nothing new. The alternatives were a second glyph module, or GPU billboarding with a new vertex attribute.
- **The gap:** slice 02 picked a look, not a mechanism.
- **The reach:** one glyph concept for every contact.
- **Verdict:** sound.
- **Confidence:** high.

### S34. Lifted airframes stay drawn while their shadow is on screen; flying raises no dust
- **When:** slice 06 (`ModelInstance.lift`, `packages/battle-renderer/src/models/modelDetail.ts`; `EffectShooter.airborne`).
- **The choice:** an aircraft's instance carries `lift`, its foot's height above the ground. That places its shadow along the sun's fall, and culling removes the model only when both the model and its shadow are off screen. Track dust is skipped for airborne units. An aircraft's sound class is `air_<weight>` (S17, N1). The alternative was a wider fixed culling margin for aircraft, based on the ceiling.
- **The gap:** none; culling and dust assumed ground units.
- **The reach:** this is exact at any altitude and leaves ground units untouched.
- **Verdict:** sound.
- **Confidence:** high.

### S35. The crash is a weapon row, and its fireball is that row's blast
- **When:** slices 04, 10 (`helicopter_crash` in `fixtures/game.json`; `apps/battle-lab/src/cookOffs.ts`).
- **The choice:** the impact is the weapon row `helicopter_crash`, extending `tank_he`: damage 50, penetration 25, 9 m blast, structural 300 (every trunk within about 6 m falls), and unlimited ammo, so it needs no resupply price. On screen the impact is that blast, drawn by the ordinary blast style. An airframe never "brews up" with a cook-off (`CookOffWatch` skips airborne types), because its death is the fall. The alternatives were a dedicated crash rule section, or a separate crash fireball.
- **The gap:** confirmed item 5 settled that it's data; the numbers were open.
- **The reach:** tune crash damage in data.
- **Verdict:** sound.
- **Confidence:** high.

### S36. A firing report's heard mask is two 24-bit words, capping the game at 48 weapon rows
- **When:** slice 07 (`heardLow`/`heardHigh` in `crates/sim/src/publication.rs`).
- **The choice:** a firing report tells the listener which weapon rows it heard, as a bit mask. The game hit 24 rows with `helicopter_crash`, and slice 09 added two more, so the mask became two 24-bit words: `MAX_WEAPON_ROWS` is 48. The alternative was dropping the crash row or shrinking the mask.
- **The gap:** a publication limit nobody had hit before.
- **The reach:** a loose tripwire. Row 49 will need a third word.
- **Verdict:** sound.
- **Confidence:** high.

### S37. `Motion` splits ground units from aircraft at the type level
- **When:** slice 01 (`units::Motion`, `Unit::ground`, `units::ground_mobility` in `crates/sim/src/units.rs`).
- **The choice:** a unit's `motion` is `Ground(Mobility)` or `Air(Flight)`. Code that only makes sense for ground units calls `Unit::ground()`, which panics for an aircraft. Catalog-level callers get `ground_mobility(...) -> Option`, so the compiler forced each of them to decide what an aircraft does. The alternative was a separate ground-only type threaded through every navigation signature.
- **The gap:** D21 forbade a third `MoverClass` without saying what to use instead.
- **The reach:** every new movement or navigation site must pick a branch.
- **Verdict:** sound.
- **Confidence:** high.

### S38. One test map, `air`, for every helicopter lab
- **When:** slices 06, 07, 10, 12, 13, 15, 18 (`fixtures/maps/air/`; `apps/battle-lab/src/routes/air.tsx`, `airHover.tsx`, `airCrash.tsx`, `airClosing.tsx`; `apps/battle-lab/src/labOverlay.ts`).
- **The choice:** all helicopter labs stand on one saved test map, `air`, at 640 × 520 m. It has a country road, a village of six templates (low homes under cruise height, a corner shop and a 7-floor block that lift the flight), a 20-floor tower across the flight line, a 70 m wood, and a Paris house for the roof checks. Each scene is a saved encounter of the map. There are four lab routes:
  - `/lab/air` (`flight`);
  - `/lab/air-hover`, with variants `hover`, `roof`, `fog`, `apache`, `lost`;
  - `/lab/air-crash` (`crash`);
  - `/lab/air-closing` (`closing`).

  The hover and closing labs share one overlay hook, `useLabOverlay`, which draws markers, drop lines and camera-facing contact signs. The flight and crash labs draw no overlay. The alternative was a map per lab.
- **The gap:** the slices named scenes, not maps.
- **The reach:** editing the map changes its hash and can retime every air scene. Only these labs read it.
- **Verdict:** sound.
- **Confidence:** high.

### Trivial discretion (5, sound)
- **Workbench fields:** the mechanics editor gained `mobility.air.*`, `targets` and `guidance` fields, so every authored value has one editing surface (slices 01/07/09).
- **Encounter test catalog:** `every_saved_encounter_makes_a_battle_on_its_map` (`crates/sim/tests/maps.rs`) runs test maps on `CatalogSet::Test`, the set their labs run (slice 18).
- **Top speed reader:** `topSpeedKmh` in `packages/scene-assets/src/units.ts` is the one TypeScript read of a mover's top speed (slice 06).
- **Spike script not shipped:** the slice 02 mockups are drawn over real game captures by a canvas script that only runs on one machine, so it isn't shipped. The images are the record.
- **Module visibility:** `movement::air` is `pub(crate)` so the battle's observation step can call `low_hover` when it decides a contact's `aloft` (S9).
