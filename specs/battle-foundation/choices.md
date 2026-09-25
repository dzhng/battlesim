# Choices ledger

Decisions the implementation made where the spec was silent. Each entry says what triggered it, what the code does, what the alternative would have done, and the verdict. User rules live in requirements.md; spec-authored resolutions in decisions.md.

## Slice 01 — stack and 3D reproduction

### Screenshots are regenerated, not committed
- **When:** slice 01.
- **The choice:** A slice's evidence images (full frames, zoomed crops, metadata) are written by its browser scene into `throwaway/evidence/<fixture-id>/`. That folder is ignored by git. The committed record is the slice verdict, which gives the numbers and each critique finding with its outcome. For example, to see slice 01's depth frames, run `bun run --cwd web scene -- foundation` and open the folder. The spec had asked for images under `specs/battle-foundation/assets/evidence/01/`, which would put every re-capture's PNGs in git history.
- **The gap:** The spec said to commit evidence images. The implementation workflow says raw captures stay out of git.
- **The reach:** Later slices follow the same rule. A reviewer who wants images must rerun the scene. Old captures are not preserved between runs unless copied aside first.
- **Verdict:** sound. Every image is reproducible from a seed-free fixture and one command. validation.md and every slice now say this.
- **Confidence:** medium.

### Lab fixtures have one registry file and one scene each
- **When:** slice 01.
- **The choice:** `apps/battle-lab/src/fixtures.json` lists each lab fixture: an id such as `foundation`, a route such as `/lab/foundation`, and a description. The React lab reads it to route pages. The browser test runner `web/scene.mjs` reads it too, and refuses to run if any fixture lacks a `web/scenes/<id>.mjs` scene or any scene lacks a fixture. The runner starts its own Vite dev server, so `bun run --cwd web scene -- foundation` works with nothing else running. The sibling repo instead needed a dev server already running and discovered scenes by walking folders.
- **The gap:** The spec asked for "one lab/scene registry" and a documented `scene -- <fixture-id>` parser, without saying what form either takes.
- **The reach:** Every later lab page (geometry, authority, movement, …) adds one JSON row and one scene file.
- **Verdict:** sound.
- **Confidence:** high.

### Browser checks run headless with the real GPU
- **When:** slice 01.
- **The choice:** The runner launches Playwright's Chromium with `channel: "chromium"` (Chrome's full headless mode) and WebGPU flags. On this Mac, that headless browser gets the hardware Apple Metal adapter, so screenshots and timing come from the real GPU. The sibling defaulted to SwiftShader, a CPU software renderer.
- **The gap:** The spec allowed a headful probe only if headless differed. It didn't say which headless mode or adapter to use.
- **The reach:** Performance slices (07, 16) measure the real GPU. A machine without a hardware adapter would fail the "hardware adapter reported" check instead of silently falling back.
- **Verdict:** sound.
- **Confidence:** high.

### The scene owns and destroys every GPU buffer itself
- **When:** slice 01.
- **The choice:** Tearing down a TypeGPU root in 0.12.5 does not free the buffers it created. The foundation test caught this: 11 buffers leaked per rebuild. `createScene` now records every allocation in an `owned` list and destroys each one in `dispose()`. The lab wraps the device's `createBuffer`/`createTexture` to count live resources, and the scene asserts the count returns to baseline after resize and rebuild cycles.
- **The gap:** The spec required cleanup to baseline but not how ownership works.
- **The reach:** Every future renderer module (fog, contacts, projectiles, rings) must use the same pattern, or the endurance slice will leak.
- **Verdict:** sound.
- **Confidence:** high.

### One pipeline draws both the world mesh and proxies
- **When:** slice 01.
- **The choice:** The renderer has a single TypeGPU pipeline. Its per-vertex input is position, normal and colour; its per-instance input is a placement (x, y, z, heading) and a tint (colour plus a highlight flag). Proxies (tank, soldier, truck, crate) are drawn instanced. The static world mesh is drawn once with an identity instance. 4× MSAA smooths edges. Lighting is one fixed sun plus ambient, with faces flipped toward the eye so hand-built wedges light correctly whatever their triangle winding.
- **The gap:** Primitive topology and scene factoring were explicitly delegated. This entry exists so later slices know the one pipeline to extend.
- **The reach:** Translucent forest canopies and fog will need a second (blended) pipeline variant.
- **Verdict:** sound (delegated discretion).
- **Confidence:** high.

### The camera uniform keeps only what the new scene reads
- **When:** slice 01.
- **The choice:** The sibling's camera uniform packed 48 floats, including focus, zoom, time and sun direction for its old environment shaders. The new one packs 40: view-projection, its inverse, eye, near plane and viewport size. A unit test checks the byte size against the WGSL struct.
- **The gap:** research.md said to port camera primitives. It didn't say whether to keep unused fields.
- **The reach:** Adding a field later means updating one packer, the struct and the size test together.
- **Verdict:** sound.
- **Confidence:** high.

### Web dependencies pinned to exact versions
- **When:** slice 01.
- **The choice:** `web/package.json` lists exact versions (`"typegpu": "0.12.5"`, not `"^0.12.5"`). They match what the sibling's lockfile resolved, and the sibling's lockfile was the starting lock. three.js, Tailwind and Radix were omitted.
- **The gap:** The spec said to preserve pinned resolved versions without saying how.
- **The reach:** Upgrading anything is now an explicit edit.
- **Verdict:** sound.
- **Confidence:** high.

## Slice 02 — world geometry

### Traversability is judged per triangle, at its centroid
- **When:** slice 02.
- **The choice:** Whether ground units may stand somewhere depends on the slope of the ground triangle under them, plus whether it is water. The export tags every terrain triangle once, by querying the surface at the triangle's centroid. The centroid lies inside that triangle, so its slope is exactly that triangle's. The first attempt tagged grid vertices instead. A vertex touches up to six triangles, so its tag came from whichever triangle the lookup happened to pick, and colours blurred across cells.
- **The gap:** The spec fixed the 35° rule and the triangulation, but not how traversability is sampled for export.
- **The reach:** Navigation in slice 04 should use the same per-triangle rule, or the overlay and the router will disagree.
- **Verdict:** sound.
- **Confidence:** high.

### Trunks and bridge decks do not block ground movement
- **When:** slice 02.
- **The choice:** `PropKind::blocks_movement()` returns false for forest trunks and bridge decks, and true for buildings, walls, crates, wrecks and ruins. Trunks still stop bullets and sight rays, since they are solid for `raycast`. A tank can therefore drive through a forest (slower, per M02) without routing around every trunk. A bridge deck is something you drive on, not around.
- **The gap:** The spec made trunks projectile colliders and forests traversable, but never said whether trunks block movement.
- **The reach:** Navigation, and later wreck and ruin behaviour, use this single rule.
- **Verdict:** sound; M02 requires it.
- **Confidence:** high.

### Plateau ("mesa") relief replaced the ramp primitive
- **When:** slice 02.
- **The choice:** Map relief has two shapes: `ridge` (the village formula) and `mesa`, a flat-topped rectangle whose sides fall at a set angle. The first version had a one-sided `ramp`. It dropped to zero at its sides, which made accidental cliffs that read as broken geometry. The lab now shows the slope limit with a 20° mesa (passable) and a 45° mesa (blocked).
- **The gap:** The spec asked for "hill, slope threshold" without a shape vocabulary.
- **The reach:** Later maps are authored with ridges and mesas; new shapes are added to one enum.
- **Verdict:** sound.
- **Confidence:** medium.

### Water surfaces and bridge decks
- **When:** slice 02.
- **The choice:** Water is a rectangle whose ground is lowered to `bed_z`, drawn as a translucent sheet at `surface_z`. The sheet is not solid, so a ray passes through it to the bed. A bridge is an oriented deck prop, solid for rays and bullets, whose top `surface_at` reports as walkable. The ground underneath is still water.
- **The gap:** The spec required "an explicit traversable top surface over impassable water", but not whether water stops rays or bullets.
- **The reach:** Rounds fired at a river strike its bed. If water should stop bullets or sight, that needs a new rule.
- **Verdict:** sound for now.
- **Confidence:** medium.

## Slice 03 — authority, transport and replay

### A malformed command still uses up its sequence number
- **When:** slice 03.
- **The choice:** Every command carries a per-side sequence number (`seq`) that must be exactly the next one: 1, 2, 3, … If the number is wrong (a skipped or repeated command), the command is refused and the counter does not move. If the number is right but the content is bad (it names someone else's unit, a unit that does not exist, or a point off the map), the command is refused, but its number is used up and it is written into the replay log. So "move unit 99" as seq 2 is rejected, and the next command must be seq 3. A replay re-submits it and gets the same rejection. The alternative, logging only valid commands, would let replay acknowledgements drift from what the player saw.
- **The gap:** The spec asked for ordered acks and a replay of "accepted commands", but didn't define whether an invalid one counts as accepted.
- **The reach:** Networking and replay export will rely on this numbering.
- **Verdict:** sound.
- **Confidence:** medium.

### The main thread builds its own copy of the static map
- **When:** slice 03.
- **The choice:** The battle authority runs in a web worker. The page also builds a `WorldView`, which is Rust's world-geometry code compiled to WASM, from the same map JSON. It uses that view to draw the terrain and to find the ground point under a right-click. Only the fixed starting map is copied, which every side knows from the start. Anything that changes later, such as wrecks or ruins, must arrive through that side's observations. The alternative was asking the worker for every ground pick, which is an asynchronous round trip on each click.
- **The gap:** The spec said picks use "the authoritative surface representation" and that static geometry is public, but not where the pick runs.
- **The reach:** Slice 09's wrecks and slice 11's ruins must update this view from observed prop changes, never from hidden truth.
- **Verdict:** sound.
- **Confidence:** medium.

### Worker and in-thread parity is proven by replay
- **When:** slice 03.
- **The choice:** The same authority code runs in a worker (production) or in the page thread ("direct"). The lab's "Check replay" button asks the worker for its replay log, plays it back in the page thread, and compares the state digest (a fingerprint of the full battle state) at every tick. Identical digests prove same-build replay and worker/direct equivalence in one check. The in-thread copy really detaches transferred buffers, just like a worker, so a bug that reuses a buffer after handing it over fails here too.
- **The gap:** The spec asked for "direct versus worker parity" without saying how.
- **The reach:** Later slices keep this check green with no new wiring, since the digest covers all authoritative state.
- **Verdict:** sound, provided every new piece of authoritative state is added to `Battle::digest`.
- **Confidence:** high.

### The simulation stops at four catch-up ticks and says it is slow
- **When:** slice 03.
- **The choice:** If the page falls behind (a stall or slow machine), each wake-up runs at most 4 ticks. Then it drops the remaining wall-clock backlog and reports "running slow". It never skips ticks. The battle simply runs slower than real time. A consumer that stops returning buffers stops the ticking entirely, shown as "waiting-consumer". A hidden tab suspends. Resuming restarts the clock from now rather than bursting through the missed time.
- **The gap:** The spec required bounded catch-up and explicit slowdown; 4 is a chosen number.
- **The reach:** Slice 16's endurance run will show whether 4 is right.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 04 — routing and group intent

### Camera pans with arrows and the screen edge, not WASD
- **When:** slice 04.
- **The choice:** The spec's control list says "WASD/edge drag pans". It also binds S to Stop and A to attack-move, and those can't both hold. Pressing S would both stop the selected units and scroll the camera down. The build keeps the command keys (S stops; A will start an attack-move in slice 08) and pans with the arrow keys and by resting the pointer at the screen edge. Middle-drag orbits and the wheel zooms.
- **The gap:** contracts.md contradicts itself.
- **The reach:** Every later keyboard command (A, G, E) assumes letters are commands.
- **Verdict:** needs-user. The provisional call is arrows plus edge pan. To reverse it, map the letters to pan and move the commands onto other keys in `useUnitControl` and `LabViewport`.
- **Confidence:** medium.

### Routes are planned on 2 m cells, so narrow gaps must be about 4 m for infantry
- **When:** slice 04.
- **The choice:** The planner divides the map into 2 m squares. A square counts as blocked if any solid prop overlaps it, which is conservative: it never plans a route through a wall. The consequence is that a gap between two walls must be about 4 m wide before infantry are guaranteed a free square, and about 6 m before a tank's 3.6 m-wide footprint fits. The rule "infantry can use gaps a vehicle can't" holds for gaps of roughly 4–6 m. A 2 m doorway would stay closed to everyone. Actual movement uses exact prop geometry; only planning is coarse.
- **The gap:** The spec fixed the rule, but not the planning resolution.
- **The reach:** Village streets are tens of metres wide, so this doesn't matter there. Later maps with alleys narrower than 4 m would need 1 m planning cells, which cost 4× the memory and search time.
- **Verdict:** sound for the village.
- **Confidence:** medium.

### Vehicles break head-on deadlocks by id priority
- **When:** slice 04.
- **The choice:** Two tanks driving at each other on open ground both stop nose to nose ("waiting for tank #1" / "waiting for tank #0"). After 2 s the tank with the higher id replans as if the other tank were a wall, drives round it, and both continue. If no way round exists, it keeps waiting, still naming the blocker, and tries again after another 2 s. Squads never block like this; they sidestep vehicles and softly push apart from each other.
- **The gap:** The spec asked that vehicles "avoid one another or wait" and that a crowd "makes progress or explains blockage", without a deadlock rule.
- **The reach:** Large convoys in slice 16 will show whether 2 s is too slow.
- **Verdict:** sound.
- **Confidence:** medium.

### Obstacles that appear mid-battle are learned by bumping into them
- **When:** slice 04.
- **The choice:** Each side plans with the map's authored props plus the new obstacles it knows about. Slice 04 has no sensing yet, so a side learns a new obstacle (for example the lab's wall that drops across the road at tick 150) only when one of its units comes within 2 m. A tank planning a route far away still plans straight through the unseen wall. When it arrives it learns the wall, replans and detours. The enemy side, which never went near it, never learns it. Slice 05 adds learning by sight.
- **The gap:** The spec said new remains enter a side's knowledge "when observed or physically encountered", but didn't say how near "encountered" is.
- **The reach:** Wrecks (09) and ruins (11) enter side knowledge the same way.
- **Verdict:** sound.
- **Confidence:** high.

### Squads are drawn and hit as individual soldiers in two loose ranks
- **When:** slice 04.
- **The choice:** A rifle squad is still one unit that you select and order. Internally it now holds 8 soldiers (4 recon, 3 AT) standing in two staggered ranks 2.5 m apart around the squad's position and facing. The renderer draws each soldier. Slice 05 casts sight rays to each soldier, and later slices will hit and kill individual soldiers. The spacing is a presentation-and-collision choice, not a formation command. Players can't set it.
- **The gap:** The spec required per-soldier sight and casualties but no formation shape.
- **The reach:** Hit rates in slice 09 depend on how spread out squads are.
- **Verdict:** sound; the spacing is tunable later.
- **Confidence:** medium.

### Group moves keep each unit's offset from the group centre
- **When:** slice 04.
- **The choice:** Select three units in a column and right-click far away. Each unit's destination is the click point plus its current offset from the group's average position, so the column arrives as a column. Offsets wider than 40 m are scaled down. A destination inside an obstacle moves to the nearest standing room within 16 m; failing that, it uses the click point. Each unit plans and drives on its own at its own speed. Queued (Shift) moves compute offsets from positions at the moment they are issued.
- **The gap:** The spec said "preserve relative destination positions where space permits" without numbers.
- **The reach:** The village bot and scripted demos rely on this.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 05 — sensors and shared identification

### The fog display refreshes five times a second; identification runs every tick
- **When:** slice 05.
- **The choice:** Two separate things answer "what can I see". Identification decides which enemies your side knows about. It fires exact rays from every observer to every nearby enemy soldier or hull, every tick (30 per second), so an enemy stepping out from behind a hill is spotted the next tick. The ground fog is the darkening you see on the map. It comes from a sweep of sight lines over terrain in 8 m squares, recomputed every 6 ticks per side, so the darkened shape can trail the true line of sight by up to 0.2 s. The sweep costs roughly 5–12 ms per side on the village map, which is too much to run 30 times a second for both sides.
- **The gap:** The spec asked for sensing each tick and allowed reduced cadence "with documented evidence and bounded visibility delay"; it didn't separate fog from identification.
- **The reach:** Slice 16's performance work may move the fog sweep to a cheaper algorithm and a faster cadence.
- **Verdict:** sound.
- **Confidence:** medium.

### An enemy's handle survives a brief loss of sight, then is replaced
- **When:** slice 05.
- **The choice:** Your side never learns an enemy's real id. It sees "contact 7", a number assigned when that enemy was first identified. If contact 7 ducks behind a wall and reappears within 1.5 s (the agreed aim-grace period), it keeps the number 7, so your units can keep aiming at the same thing. If it stays hidden longer, the number is retired. When it reappears it becomes "contact 12", as if it might be a different tank, because your side can't know it's the same one. The alternative, keeping one number forever, would quietly tell the player "that's the tank you saw 5 minutes ago".
- **The gap:** The spec required side-scoped handles and a 1.5 s acquisition grace, but not when a handle is reissued.
- **The reach:** Weapon lock-on (08) and last-seen contacts (06) key on these handles.
- **Verdict:** sound.
- **Confidence:** medium.

### A partly seen squad is reported where its visible soldiers stand
- **When:** slice 05.
- **The choice:** If three soldiers of an eight-man squad step out from behind a building, you see those three. The squad's reported position is the average of those three, not the squad's true centre, which may be hidden behind the building. Its reported velocity likewise comes from those observed positions.
- **The gap:** The spec said to publish only observed member poses, but didn't say which position represents a partly seen unit.
- **The reach:** Weapons aim at observed positions; aiming at the hidden centre would leak information.
- **Verdict:** sound.
- **Confidence:** high.

### Fixture scripts move the opposing side in labs
- **When:** slice 05.
- **The choice:** A lab needs the enemy to move, for example red's tank driving through a forest so blue can watch detection change. The fixture can list timed orders ("at tick 30, red tank 3 moves to (480, 300)"). These are part of the scenario itself, like the tick-150 wall. They run identically in live play and in replay, and they don't use the player's command sequence numbers. The village's defending AI (slice 15) is different: it chooses orders from what red observes, so its commands go through the normal command path and are recorded in the replay.
- **The gap:** The spec mentioned "deterministic command scripts" in fixtures but not how they enter the authority.
- **The reach:** Every lab with a moving enemy uses this.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 07 — physical flight (built in parallel, merged)

### A shot through a ridge is refused, never re-aimed over it
- **When:** slice 07.
- **The choice:** Before a weapon fires, the solver flies the intended curved path against the real terrain and props. If something (a ridge crest, a wall) would stop the round more than 0.5 m short of the target, the answer is "blocked" and nothing fires. A tank never quietly switches to a lobbed high arc to get over the hill. Only weapons marked as indirect fire (mortars and artillery, arriving in slice 22) try the high arc first. Accuracy spread is applied after this check, so a scattered round that clips the crest is a genuine miss, not a refusal.
- **The gap:** P03 forbids ignoring the ridge, but gave no arrival tolerance or order for trying arcs.
- **The reach:** Slice 08's "blocked trajectory" reason comes from this.
- **Verdict:** sound.
- **Confidence:** medium.

### Weapons fire direct unless the fixture marks them indirect
- **When:** slice 07.
- **The choice:** A weapon row without a `trajectory` field fires low, direct arcs. Only `"trajectory": "indirect"` enables high arcs. No village weapon is indirect. The ballistics lab uses a lab-only 45 m/s "mortar" row to show a high arc.
- **The gap:** The fixture had no such field.
- **The reach:** Slice 22's artillery must set it.
- **Verdict:** sound.
- **Confidence:** high.

### Round lifetime comes from the weapon row, capped by physics
- **When:** slice 07.
- **The choice:** A round expires after its weapon's `lifetime_s` (the ATGM's 12 s), or after the 30 s physics cap for rows without one. A row asking for more than the cap is a configuration error. The launch solver only considers intercepts within that lifetime.
- **The gap:** "Bounded lifetime" gave no source for the number.
- **The reach:** Every weapon.
- **Verdict:** sound.
- **Confidence:** medium.

### Rounds leaving the map end there
- **When:** slice 07.
- **The choice:** A round that is off the map and moving away is removed with a "left the map" ending event, since nothing (no wind) can bring it back. This is bookkeeping, not a cap on how many rounds exist.
- **The gap:** Only hit and lifetime endings were specified.
- **The reach:** Every consumer of flight events sees exactly one ending per round.
- **Verdict:** sound.
- **Confidence:** high.

### A unit is not suppressed by its own outgoing fire
- **When:** slice 07.
- **The choice:** A near miss is measured from the round's flown path to a body's surface. Each round reports at most one near miss per unit per tick (the closest body). The firing unit is excluded, so a rifle squad's own bullets whizzing past its members do not pin it down. Squadmates standing in front of the muzzle can still be physically hit (P09); they just don't get suppression from their own squad's fire. The struck body is excluded from near-miss reporting only in the tick it is hit.
- **The gap:** The spec never said whether own fire suppresses the firer.
- **The reach:** Slice 09's suppression.
- **Verdict:** needs-user. The provisional call is "own fire never suppresses the firing unit"; reversing it is a one-line filter in the near-miss pass.
- **Confidence:** medium.

### Turning vehicles are checked with a slightly generous box
- **When:** slice 07.
- **The choice:** Within each flight step, a turning tank's box is tested at its middle heading, grown by the farthest any corner moves while turning. Steps that register a hit are halved until that growth is under 1 mm. A hit can therefore land up to 1 mm early, but a real hit is never missed.
- **The gap:** The spec asked for conservative bounds and narrowed time of impact without a method.
- **The reach:** Every vehicle hit.
- **Verdict:** sound (delegated).
- **Confidence:** high.

### Scatter is solved on the same arc; unreachable scatter is a refused shot
- **When:** slice 07.
- **The choice:** Accuracy spread moves the aim point sideways and up or down in the plane facing the shooter. The round is then solved to hit that moved point on the same (low or high) arc. At the very edge of range the moved point can be out of reach. The shot is then refused, though the random draw is still used up. It does not quietly fire the perfect unscattered shot.
- **The gap:** The spec converts spread before solving, but not the edge case.
- **The reach:** Only fire at the edge of range.
- **Verdict:** sound.
- **Confidence:** medium.

### One seeded random generator for all combat randomness
- **When:** slice 07.
- **The choice:** `sim::rng::Rng` is SplitMix64: one 64-bit number of state, easy to fold into replay digests. Normal samples use Box–Muller; the ±3σ cut uses rejection. Results repeat exactly within one build, which is the replay promise.
- **The gap:** No generator existed.
- **The reach:** Combat, contact uncertainty and bot policy each get their own stream from it.
- **Verdict:** sound.
- **Confidence:** high.

### Flight events are ordered by time within the tick
- **When:** slice 07.
- **The choice:** Events in one tick are sorted by when they happen within the tick. Ties go by round id, then near misses before that round's ending, then by unit.
- **The gap:** "Ordered events" had no defined order.
- **The reach:** Slice 09 applies damage and suppression in this order.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 06 — uncertain evidence and sound

### A lost identification leaves a full-size area at the last sighting
- **When:** slice 06.
- **The choice:** When your side loses sight of an identified enemy, an orange disc of the standard 100 m contact radius appears, centred exactly where it was last seen, and fades over 8 s. It never moves. If the unit is re-identified, the disc disappears. The alternative, a smaller disc for "last seen" since you knew exactly where it was, would suggest precision that decays at an unknown rate.
- **The gap:** V09 said "using the spotted-contact visual language" without a radius.
- **The reach:** The village AI's retreat logic and players' return fire both use these areas.
- **Verdict:** sound.
- **Confidence:** medium.

### Each sound goes to the nearest listener that heard it
- **When:** slice 06.
- **The choice:** Every half-second, each unseen enemy produces at most one cue per sound type (engine or footsteps, plus gunfire if it fired). The cue belongs to the nearest friendly unit within hearing range for that type: 200 m for infantry, 650 m for vehicles, 1000 m for shots. The caption reads "Heard engine, moving, near, east of recon #0". Direction snaps to 8 compass points; distance is near (within half the hearing range) or far. Identified enemies produce no cues, since you can see them. Idle units still make noise ("voices", "engine, idling").
- **The gap:** V11 fixed the rules, but not which listener owns a cue heard by several.
- **The reach:** Audio and captions everywhere.
- **Verdict:** sound.
- **Confidence:** medium.

### Your own overlays are drawn over the fog
- **When:** slice 06.
- **The choice:** Route lines, destination rings, contact discs and remembered obstacles draw at full brightness even over fogged ground. Terrain, units and props under fog are darkened. These overlays are your side's own knowledge, and a firing area is most useful exactly where you can't see.
- **The gap:** The spec didn't say how overlays and fog combine.
- **The reach:** Every later overlay (rings, aim lines) follows the same rule.
- **Verdict:** sound.
- **Confidence:** high.

## Slice 08 — independent weapons and engagement policy

### Moving fire spreads √2 wider
- **When:** slice 08.
- **The choice:** A movement-capable weapon firing on the move multiplies its spread by √2 (`physics.moving_scatter_multiplier`). For a small target, the chance to hit falls with the square of the spread, so this halves hits: the brief's "50% accuracy reduction". The alternative was halving some separate hit chance, but no such chance exists: every round flies physically.
- **The gap:** W04 gave the target ("50% accuracy") but not what to scale in a physical-flight model.
- **The reach:** Every shot fired while moving, and the village's balance.
- **Verdict:** sound.
- **Confidence:** medium.

### A blocked target can be swapped for a shootable one while reloading
- **When:** slice 08.
- **The choice:** A weapon reconsiders its target while reloading, and automatic choice only picks targets it could actually shoot now: in range, with a clear trajectory, and no friendly vehicle in the way. So if the tank you were shooting slips behind a wall's edge mid-reload, the gun may switch to a firing area it can reach. If nothing better exists, it keeps the old target and shows "blocked trajectory". The grace still protects a target that is merely out of sight while the gun is aiming.
- **The gap:** The contract said "highest-cost identified damageable target" without saying whether a blocked one counts.
- **The reach:** Target churn around cover, and the AI's apparent persistence.
- **Verdict:** sound.
- **Confidence:** medium.

### An attack-move holding to shoot shows "halted to engage"
- **When:** slice 08.
- **The choice:** A unit on attack-move stops advancing while any of its weapons is aiming, reloading, traversing or firing at something it can engage. While stopped, its movement state is a new published value, `halted`, rather than "moving". Once it cannot engage anything (for example, the target's last-seen area has faded), it resumes. A target it cannot hurt, or cannot shoot at, never halts it.
- **The gap:** W16 said "stops for reachable targets" but not how "stopped" is shown or exactly which weapon states count.
- **The reach:** Attack-move feel, and the slice 14 readouts.
- **Verdict:** sound.
- **Confidence:** high.

### A squad weapon fires one round per living soldier
- **When:** slice 08.
- **The choice:** The rifle mount of a squad is one weapon with one aim and one reload, but each shot launches one round from every living soldier. The rounds aim at seen enemy soldiers in turn, or at points sampled inside a firing area. The alternative, a single representative round, would make squad strength irrelevant to firepower.
- **The gap:** The spec gave squads a rifle mount without saying how members contribute.
- **The reach:** Squad firepower, and damage from slice 09 on.
- **Verdict:** sound.
- **Confidence:** medium.

### Friendly-vehicle withholding uses a 1 m margin
- **When:** slice 08.
- **The choice:** A weapon holds fire when its predicted path passes within 1 m of a friendly vehicle's hull, or when the aim point's blast radius covers one (`physics.friendly_prefire_margin_m`). Friendly infantry never stops a shot.
- **The gap:** P11 said "obstruct the predicted path" with no tolerance.
- **The reach:** Tanks firing past each other in column.
- **Verdict:** sound.
- **Confidence:** medium.

### Mounts are authored as named groups of ammunition kinds
- **When:** slice 08.
- **The choice:** `village.json` mounts became records, for example the tank's `cannon` with `tank_ap` and `tank_he`, which share one aim and reload, plus a separate `HMG`. Records also say whether the mount is a squad weapon and whether it sits on a turret, which traverses at `movement.turret_turn_deg_s`. Weapons gained `anti_armor` (never engages infantry) and `armor_piercing` (never fired at an area) flags. A weapon row flagged `default` (the rifle and the HMG, both unlimited) is its unit's default gun.
- **The gap:** The fixture listed weapons as loose strings such as "cannon: tank_ap | tank_he".
- **The reach:** Every later slice that reads weapons: missiles, supply, readouts.
- **Verdict:** sound.
- **Confidence:** high.

### Enemy tracers are clipped to seen ground, not to line of sight per round
- **When:** slice 08.
- **The choice:** An enemy round's visible stretch is drawn only where it flies over ground your side currently sees, using the same 8 m visibility grid as the fog, refreshed every 6 ticks. Each stretch stops at its last sample over seen ground, never beyond. A per-round sight cast from every friendly eye was the alternative, and would cost a cast per round per tick.
- **The gap:** The slice 06 carry-in said "ground visibility field plus line of sight".
- **The reach:** A tracer high above a hidden valley stays hidden even if someone could, in principle, see the sky above it. Tracers can lag fog changes by up to 0.2 s.
- **Verdict:** needs review. It is conservative (it never reveals more than the fog), but it is not the stated rule.
- **Confidence:** medium.

## Slice 09 — consequences of physical fire

### A squad's own rounds never strike its own soldiers
- **When:** slice 09. This changes a slice 07 choice.
- **The choice:** A round cannot hit any soldier of the unit that fired it. Before this, only the firing soldier was exempt. So a squad's back rank shot its own front rank every volley and lost soldiers without an enemy in sight. Soldiers of any *other* unit still take the round, friend or foe (P09), and near misses still skip the firing unit. The alternative was to make back-rank soldiers hold fire when a squad-mate is in the way, but P11 says infantry never withholds fire.
- **The gap:** P09 says collisions apply "regardless of allegiance", but not whether a squad's synchronized fire can pass through itself.
- **The reach:** Every squad volley. The slice 07 flight test now pins "another friendly unit's soldier takes the round; your own squad-mate does not".
- **Verdict:** needs review. It reverses an earlier pinned behaviour, although that behaviour came from a choice made without the user.
- **Confidence:** medium.

### Blast damage uses the weapon's one damage figure
- **When:** slice 09.
- **The choice:** An explosive round's direct hit does its `damage`, and its blast does that same `damage` scaled by `(1 − r/R)` to each soldier whose fragment roll hits. The directly struck body is skipped for the blast. No separate blast-damage number was added to the fixture.
- **The gap:** The contracts say "configured blast damage", but the fixture has only one damage value per weapon.
- **The reach:** HE, grenade and ATGM lethality.
- **Verdict:** sound. A separate figure can be added later as a tuning key without a rule change.
- **Confidence:** medium.

### Which armour face a hit meets
- **When:** slice 09.
- **The choice:**
  - For a round, the face is read from where it struck. Take the point just outside the hull, in the hull's own frame, divided by the box's half-sizes. It is the roof if the point is above the hull more than beside it; otherwise it is front, rear or side, whichever axis dominates.
  - For a blast, the same rule is applied to the burst point, and damage falls with distance to the hull's surface.
  - The burst must have a clear line to the hull's centre.
- **The gap:** P10 names four faces but not how to pick one at an edge or corner.
- **The reach:** Flanking value against tanks, and AT ambush angles in slice 10.
- **Verdict:** sound.
- **Confidence:** medium.

### Suppression from one round is its strongest effect on each squad
- **When:** slice 09.
- **The choice:** In a tick, a round's near miss and its impact nearby could both suppress the same squad. Only the stronger counts. Impacts are measured to the squad's nearest standing soldier. Vehicles are never suppressed, because suppression slows infantry movement and fire. Friendly rounds suppress friendly squads as well, since the rule measures distance, not who fired; the one exception is the unit that fired the round.
- **The gap:** The contracts say "one near-miss suppression event per squad per tick" but not how near misses and impacts combine.
- **The reach:** How quickly squads get pinned.
- **Verdict:** sound.
- **Confidence:** medium.

### A death your side sees ends the track; a death it doesn't see says nothing
- **When:** slice 09.
- **The choice:** If your side had an enemy identified when it died, the identification ends at once, with no "last seen" area left behind, and an attack order on it is complete. If it died unseen, nothing changes: the identification lapses as usual after 1.5 s, and a last-seen area follows. Enemy fallen soldiers are shown once your side has had the ground they lie on in view, and remembered afterwards. Your own fallen are always shown.
- **The gap:** The spec says "a dead visible target completes the attack" and "corpses remain visual records", but not who learns of a death, or when.
- **The reach:** Attack orders, the knowledge the village AI works from, and what a player can infer.
- **Verdict:** sound.
- **Confidence:** medium.

### A wreck blocks infantry as well as vehicles
- **When:** slice 09.
- **The choice:** A destroyed vehicle becomes a permanent `wreck` prop the size of its hull. Like every other solid prop, it blocks all ground movement: soldiers walk around it, they don't climb through it. It blocks sight and rounds by its shape (M07). Each side reroutes only once it has seen the wreck.
- **The gap:** M06 says "vehicle wrecks obstruct vehicles" without saying whether infantry pass.
- **The reach:** Narrow village lanes after a tank dies in one.
- **Verdict:** needs review. Letting infantry pass would need a second movement rule for one prop kind.
- **Confidence:** medium.

### Destroyed units leave the own list and refuse orders
- **When:** slice 09.
- **The choice:** A destroyed vehicle or an eliminated squad disappears from its side's unit list; its wreck and fallen stay on the map. An order naming it is rejected as `destroyed`. The pre-weapon labs (sensors, contacts) now have every unit holding fire, so what they demonstrate isn't disturbed by a firefight.
- **The gap:** The spec didn't say how a dead unit appears to its owner.
- **The reach:** Selection, and the slice 15 battle report, which will need its own loss list.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 12 — deployment

### Progress counts whole ticks
- **When:** slice 12.
- **The choice:** A deploying unit stores how many ticks of setup it has completed, from 0 to the full duration (15 s × 30 Hz = 450 ticks, from `service.deploy_and_pack_s`). Each tick moves it one step toward its target. The published progress is that count divided by the duration. So a truck 40% deployed packs in exactly 180 ticks, and one that is half packed redeploys in exactly 225. The alternative was a fraction that grows by 1/450 each tick; its rounding error would make the two directions differ by a tick.
- **The gap:** L01 asked for equal, reversible durations. It did not say how progress is stored.
- **The reach:** Every duration check in the tests is exact. Slice 13 reads readiness from the same count.
- **Verdict:** sound.
- **Confidence:** high.

### The stored end state is the posture a stopped unit holds
- **When:** slice 12.
- **The choice:** Besides progress, each deploying unit stores one posture, deployed or packed, that it holds when it has nowhere to go. Deployed is the default, because a stopped supply unit sets up. Pack sets it to packed. The target that progress heads to is worked out each tick: packed while the unit has a movement goal, and otherwise the stored posture. That target is published as the desired end state. Stop, Deploy and any new move or attack order put the stored posture back to deployed. The alternative was to store the target itself and flip it when orders start and end. That needs a second memory of "was I moving?", which is the duplicate flag the slice forbids.
- **The gap:** The contract gives the rules (move packs, a stopped unit deploys, Pack keeps it packed) but not what is stored.
- **The reach:** An explicit Pack is consumed by the next move: the truck sets up again where it arrives. To arrive packed, press Pack while it drives.
- **Verdict:** sound.
- **Confidence:** medium.

### A supply truck starts packed and sets up at once
- **When:** slice 12.
- **The choice:** A deploying unit enters the battle at progress 0. It has no orders, so it starts deploying on the first tick and is ready after the full duration. The alternative was an authored "starts deployed" field on the scenario's unit setup.
- **The gap:** The spec did not give an initial deployment state.
- **The reach:** In the village, the supply truck can serve 15 s after the start if nobody moves it. A move ordered at once leaves without delay.
- **Verdict:** sound; an authored start state can be added when an encounter needs one.
- **Confidence:** medium.

### Deploy ends movement; Pack does not
- **When:** slice 12.
- **The choice:** The new `set_deployment` order acts immediately; the Shift (queued) flag is ignored. Deploy (`deployed: true`) clears the unit's orders and route, like Stop, but leaves its weapons alone. Progress then reverses toward deployed from wherever it stands. Pack (`deployed: false`) only changes the stored posture, so a moving unit keeps moving. Units that never deploy ignore the order entirely, so Deploy on a mixed selection never stops a tank.
- **The gap:** The contracts sketch lists `SetDeployment { units, deployed }` without saying how it meets the order queue.
- **The reach:** Pressing Deploy on a moving truck makes it stop and set up where it is.
- **Verdict:** sound.
- **Confidence:** medium.

### A unit waiting to pack neither drives nor turns, and says "packing"
- **When:** slice 12.
- **The choice:** The movement gate sits in `movement::step_unit`: a unit with a route but some setup remaining neither translates nor turns. It reports a new published movement state, `packing`, and its stall watch is paused, as with `halted`. It still plans its route, which is drawn. Progress moves before movement in each tick, so the unit moves on the same tick that packing finishes.
- **The gap:** "Translate only at zero" did not say whether turning in place counts, or what the unit reports meanwhile.
- **The reach:** The command panel and later readouts can say why a truck with orders is not moving. Hearing still treats it as idle.
- **Verdict:** sound.
- **Confidence:** high.

### Deployment is shown as a ground ring and a primitive pose
- **When:** slice 12.
- **The choice:** Around a deploying unit, the ground carries a thin light track ring and a bright arc. The arc runs clockwise from the unit's nose, and its length is always how deployed the unit is. It is green while deploying and amber while packing. A white arrowhead at the arc's moving end points the way progress runs, clockwise while deploying and back along the arc while packing, so direction never depends on colour alone. When fully deployed, a solid dark-green disc fills the ring under the unit. The ring draws above route ribbons. The panel's bar reads "deployed 11.2/15.0 s", and its label ("↻ deploying", "↺ packing", "✓ fully deployed", "packed") uses the ring's colours and arrows. The truck's pose is a pure function of progress, blended between ticks like its position: four stabiliser legs slide out from under the cargo bed and drop to the ground, and a mast with a lamp rises out of the bed. When fully packed every part is hidden inside the hull.
- **The gap:** The primitive folded/unfolded pose was delegated. The progress indicator's form was not specified before slice 14's readouts.
- **The reach:** Slice 14 may fold the ring into its readouts. Real models later replace the parts without touching the rule.
- **Verdict:** sound (delegated discretion).
- **Confidence:** medium.

### Deployment rules live in a `service` section of the rules
- **When:** slice 12.
- **The choice:** `Rules` gained `service: ServiceRules`, which reads `deploy_and_pack_s` from the fixture's existing `service` section. Only the supply unit kind deploys: `deployment::initial` maps each unit kind to a duration, the same way `units::mobility` maps kinds to speeds. No fixture value was added.
- **The gap:** None of the numbers were missing. Which unit kinds deploy was implied by L02 but not stated as data.
- **The reach:** Slice 13 adds the stock and rate fields to the same struct. A later deploying kind, such as radar in slice 19, adds one match arm.
- **Verdict:** sound.
- **Confidence:** high.

## Slice 10 — supported AT guidance

### Guided missiles fly straight at constant speed and chase what the launcher sees
- **When:** slice 10.
- **The choice:** A weapon row with `turn_deg_s` is guided:
  - It flies at a constant speed with no gravity, turning at most `turn_deg_s` toward its commanded point.
  - Launch solving, the friendly-vehicle check and flight all use that same gravity-free path.
  - While supported, the point is the target's position as the launcher last saw it, with no lead. At 180 m/s against a 6 m/s tank, chasing the target is enough, and it never uses a prediction the launcher couldn't make.
- **The gap:** The contracts say steering "follows observed target motion within configured turn limits" without giving a flight model or a steering law.
- **The reach:** Every ATGM shot, and later AA missiles in slice 19.
- **Verdict:** sound.
- **Confidence:** medium.

### Support is renewed from the last sensing, one tick behind
- **When:** slice 10.
- **The choice:** Each tick, before rounds fly, a launcher keeps its missile only if it stood still, lives, and its own sensors identified the target at the last sensing. Sensing runs after flight within a tick, so that is the previous tick's sight. Otherwise the missile is released at once and for good: its point drops to the ground straight beneath its last position, and the missile flies on to it. Stop releases through the same check.
- **The gap:** The spec lists what releases support but not when in the tick it is judged.
- **The reach:** Escapes are decided within a thirtieth of a second of sight being lost.
- **Verdict:** sound.
- **Confidence:** high.

### A launcher's new reasons: guiding, and no own sight
- **When:** slice 10.
- **The choice:** Two action reasons were added.
  - "Guiding": a loaded next missile waits while one is still in flight.
  - "No own sight": a launcher whose target only the team (for example, a scout) identifies.
  - Neither names what blocks the view.
- **The gap:** The listed reasons include "guiding" but nothing for the own-sight rule.
- **The reach:** The slice 14 readouts.
- **Verdict:** sound.
- **Confidence:** high.

### The ambush lab's escape uses cover beside the tank, not a hill
- **When:** slice 10.
- **The choice:** A missile covers 500 m in about 3 s, and a tank moves 6 m/s. So only cover a few metres away can break sight in time. A hill tens of metres away never can. The lab therefore puts a building right beside the tank.
  - **Prompt escape:** the tank ducks behind the building at launch, and is untouched.
  - **Late escape:** the tank is hit.
  - **Prepared crossfire:** a second team still sees the tank behind the building, and it is hit.
- **The gap:** The slice named a "hill escape".
- **The reach:** Lab only. The village's own geometry decides real escapes.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 11 — garrisons and ruins

### Occupants stand on slots just outside the walls
- **When:** slice 11.
- **The choice:** A garrisoned soldier stands at a perimeter slot 0.45 m outside a facade (`garrison.slot_standoff_m`). Its hit capsule, its eyes and its muzzle all sit there. The building has one slot per soldier of capacity (16). Slots are split evenly over the four facades, with any remainder going to the longer ones, and spaced evenly along each facade. The squad's own position becomes the building's centre, which is where firing reports and sounds come from.
- **The gap:** The contract said capsules sit "on the exterior side of the facade" and outgoing origins "just outside". It gave no distance and no slot layout.
- **The reach:** A round that misses a soldier meets the wall right behind it. A round leaving by a facing facade can never re-enter its own building, so no collider is ever switched off. The lab draws each occupant on a small pad at its slot.
- **Verdict:** sound.
- **Confidence:** high.

### A slot faces a target only by a clear margin
- **When:** slice 11.
- **The choice:** A soldier fires only from a slot whose facade the line to the target leaves by more than 6° (`garrison.slot_facing_min_deg`). The check is made for each round's own aim point. The widest truncated spread in the fixture is about 2.6°, so a round can never graze back into its own wall.
- **The gap:** The contract said "a facade facing its observed target point", with no rule for grazing angles.
- **The reach:** A target almost parallel to a wall is served by the next facade round the corner. Targets at the corners are served by two facades.
- **Verdict:** sound.
- **Confidence:** high.

### Soldiers move to facing slots one tick after a weapon locks
- **When:** slice 11.
- **The choice:** Each tick, before weapons act, `garrison::allocate_slots` reads each garrisoned mount's current lock and the point its own side sees it at. A squad weapon moves every living soldier; a single weapon moves its operator, the first living soldier. A soldier already facing the target stays put. Otherwise it takes the free facing slot closest to it, with the lower slot index winning ties. If no facing slot is free, it waits, and a weapon with nobody facing reports `no_facing_slot`. A fresh lock therefore gets its soldiers on the next tick; the aim time (0.8 s or more) hides that.
- **The gap:** "Slot relocation is a one-tick garrison abstraction" did not say when it runs, or who moves for which weapon.
- **The reach:** In a full building a target on one side is fired on only by the soldiers already on that facade. The rest wait, as the contract asks.
- **Verdict:** sound.
- **Confidence:** medium.

### Squads spread evenly when they enter
- **When:** slice 11.
- **The choice:** An entering squad takes free slots in turn from each facade: the first slot on each of the four facades, then the second on each, and so on. So two rifle squads each put two soldiers on every facade.
- **The gap:** "Initial occupancy is distributed evenly" did not say across what.
- **The reach:** Every facade is watched from the moment a squad enters.
- **Verdict:** sound (delegated slot distribution).
- **Confidence:** high.

### Capacity is checked at the order and again at the door
- **When:** slice 11.
- **The choice:** A garrison order is refused with `capacity_full` if the ordered squads, the side's occupants and the squads already heading in would not all fit. Only the side's own units are counted, so the refusal reveals nothing hidden. When the entry timer ends, the squad is checked again against everyone actually inside. If it no longer fits, or enemies hold the building, it waits beside the building (`waiting_for_room`) and tries each tick. It never splits.
- **The gap:** The contract said to reject a squad that does not fit. It did not cover two orders in the same tick, or a building the enemy holds.
- **The reach:** A player learns an enemy holds a building only by walking up to it. That is physical contact, the same way an unseen wreck is learned.
- **Verdict:** sound.
- **Confidence:** medium.

### Entering and leaving are stationary, and suspend weapons
- **When:** slice 11.
- **The choice:** A squad walks to a point 2 m outside the facade nearest to it. Once within 4 m of the walls (`garrison.entry_distance_m`), it stands still for `enter_exit_s` (2 s), then is seated. Leaving takes the same time. While entering or leaving, the squad has no movement goal and its weapons report `changing_position`. A move or attack-move order given to a squad inside makes it leave first, then go. Stop during the entry timer cancels the entry. Stop during the exit timer keeps the squad inside. An attack order given to a squad inside fires from the building and never walks out to pursue.
- **The gap:** The contract gave the timers but not how other orders interact with them.
- **The reach:** Garrison orders queue with Shift like any other (U01). Exit has no key; the lab and the command bar use a Leave building button.
- **Verdict:** sound.
- **Confidence:** medium.

### A squad leaves as a formation, beside where it is going
- **When:** slice 11.
- **The choice:** A leaving squad is placed at the point round the building nearest to its next destination, or nearest to where it entered if it has none, among points sampled every 2 m outside. The point must give every soldier in the formation standing room. If there is none, the squad stays inside and tries again. Placement is deterministic.
- **The gap:** The contract asked for "deterministic free positions outside the perimeter" without saying which.
- **The reach:** Entry and exit are symmetric: soldiers leave their slots for a formation outside, just as they left the formation for slots.
- **Verdict:** sound.
- **Confidence:** medium.

### Building cover goes to a target seen at its slots
- **When:** slice 11.
- **The choice:** A round aimed at an identified squad that is garrisoned spreads wider by the building multiplier. Blast fragments reach its soldiers with the building's lower probability. Where forest cover also applies, the stronger protection wins; the two are never multiplied. Fire at a contact or at ground gets terrain cover only. A blast skips the occupants' own building when testing for walls in the way, because the building already counts as their cover.
- **The gap:** The contract says building strength is authored and applies "once", but not how an aimer knows a target is in a building.
- **The reach:** Garrisoned soldiers can only be seen at their slots, so "seen garrisoned" is what the aimer observes. A hit on a soldier still does full damage (P12).
- **Verdict:** sound.
- **Confidence:** medium.

### Garrisons use the fixture's building concealment
- **When:** slice 11.
- **The choice:** Garrisoned soldiers are detected at `sensors.building_range_multiplier` (0.2) times normal range. That value was in the fixture but unused. As with cover, the strongest concealment source wins. Infantry then see a garrison at 120 m, and a tank at 70 m.
- **The gap:** No slice said what the value was for. The brief says a building covers much better than a forest.
- **The reach:** Enemies spot a garrison mostly when it fires, which gives a contact area. The lab's red tank has to rely on its squad to spot for it.
- **Verdict:** open to review. It changes how the village plays, and the value came from the provisional fixture rather than from the user.
- **Confidence:** medium.

### Only direct hits wear a building down
- **When:** slice 11.
- **The choice:** A round that strikes a building takes its `structural_damage` off the building's health. Blast near the building does not. Rounds with no structural damage, such as rifles and the HMG, never wear it. Building health lives in `garrison::Structures`, not in the world, which owns only geometry.
- **The gap:** L10 said only structural weapons damage buildings. It did not say whether blast counts.
- **The reach:** A tank's HE brings the village's 400 hp buildings down in four hits on the walls.
- **Verdict:** sound.
- **Confidence:** medium.

### A collapse swaps in a lower ruin and survivors escape on foot
- **When:** slice 11.
- **The choice:** At zero health the building is removed. A ruin of the same footprint and `buildings.ruin_height_m` (2 m) takes its place, through `world.add_prop`. Each occupant survives with `garrison.survival_probability_on_collapse`, rolled on the damage stream. A survivor searches rings 1 m apart around its slot, out to `exit_search_radius_m`. It takes the first point it can stand on and walk to in a straight line without crossing a solid or water. Survivors keep at least 1 m apart. A survivor with no such point dies where it stood. The squad gathers on the survivor nearest their middle and carries at least `suppression.collapse_level` suppression. Scattered soldiers walk back to their places in the formation at infantry speed. Squads still outside entering are unharmed.
- **The gap:** The contract gives the rules, not the search, the gathering or what a scattered squad does next.
- **The reach:** The squad drifts back into shape over a few seconds. The published known prop gains `replaces`, so a side that sees the ruin stops drawing the building it replaced.
- **Verdict:** sound (the collapse itself is a placeholder swap, as delegated).
- **Confidence:** medium.

### Every side plans round a ruin, seen or not
- **When:** slice 11.
- **The choice:** A side learns the ruin as a known prop only when some of its footprint comes into view. Its route planning, though, always includes ruins. A ruin stands exactly on the authored building it replaced, so including it means an unseen collapse can never open a route through that footprint. Dynamic props are now learned when any fog cell under their footprint is seen, not only their centre, because a 24 m ruin hides the ground at its own middle.
- **The gap:** The contract says unseen changes must not alter a side's routes. Removing an authored building would otherwise have done exactly that.
- **The reach:** Wrecks are learned the same way as before, only sometimes earlier.
- **Verdict:** sound.
- **Confidence:** high.

### The digest carries order kinds and soldier offsets
- **When:** slice 11.
- **The choice:** `Battle::digest` now tags each queued order with its kind and includes each soldier's offset. Garrison state (building, phase and timer, the entry point and every seat) and building health and ruins go in with presence and length tags.
- **The gap:** Earlier slices did not need either: move and attack-move digested the same, and offsets never changed.
- **The reach:** Replays of a collapse compare scattered soldiers exactly.
- **Verdict:** sound.
- **Confidence:** high.

### The garrison lab draws buildings apart from the world mesh
- **When:** slice 11.
- **The choice:** `buildWorldMeshes(..., "apart")` leaves buildings out of the static mesh, and `buildStandingStructures` draws the ones the side has not seen fall. A known ruin is drawn as a 5 × 5 grid of heaps, none of them above the ruin's collider. The lab route draws occupant pads, an arc for the entry or exit timer, and a red ring for a squad waiting for room. The scene harness clears React's development performance measures as it fast-forwards. Without that, thousands of ticks exhaust the buffer and the page fails with "Data cannot be cloned, out of memory".
- **The gap:** Presentation of garrisons and ruins was delegated. The React issue affects any long development-mode session.
- **The reach:** Other routes keep buildings in the world mesh until they need collapses. A long manual session in development mode can still hit the React limit.
- **Verdict:** sound for the lab; the growth of React's measures is noted for slice 16's longevity pass.
- **Confidence:** medium.

### Right-click on a building garrisons it
- **When:** slice 11.
- **The choice:** `useUnitControl` gives `PointerPick` an optional `building`. A right-click whose camera ray first meets a static building sends a garrison order for the selection; Shift queues it. `exitBuilding()` sends `exit_building`. The protocol mirrors the two new orders.
- **The gap:** The controls contract lists garrison among queueable orders but gives no gesture.
- **The reach:** Slice 14's command bar can add a button for leaving.
- **Verdict:** sound.
- **Confidence:** medium.

### The ground an observer stands on is seen
- **When:** slice 11.
- **The choice:** `visibility::sweep` now marks the fog cell under each eye as seen. Its rays start one cell out, so a lone squad used to stand on a dark square of unseen ground. The critique of the garrison lab caught it.
- **The gap:** Slice 05 left the observer's own cell to its neighbours' sweeps, and never said whether that was intended.
- **The reach:** Fog under isolated units is no longer dark. Identification is unchanged, because it is judged per target, not by fog.
- **Verdict:** sound.
- **Confidence:** high.

## Slice 13 — finite supply

### Every eligible unit is served at once; stock is paid in unit order
- **When:** slice 13.
- **The choice:**
  - A set-up truck serves every eligible unit in reach at the same time, each at the configured rates: 1 round/s, 2 hp/s, one soldier per 5 s.
  - When an item completes, it is paid for in ascending unit id. So if stock runs short, the lower-numbered unit gets the last of it.
  - An item is never part-paid: the unit waits with "no stock".
  - A unit within reach of two trucks is served by the lower-numbered one only.
  - Progress toward an item pauses (it isn't lost) while the unit is ineligible.
- **The gap:** The contract's "round-robin one service quantum per eligible recipient" could mean serving one unit after another, or every unit at once.
- **The reach:** How fast a battered group recovers, and who comes first when stock runs low.
- **Verdict:** sound.
- **Confidence:** medium.

### Replacement soldiers take a fallen soldier's place and a new id
- **When:** slice 13.
- **The choice:** A replacement is a new soldier with a new id, standing in a fallen soldier's formation spot. The fallen soldier's record stays exactly where it lies. A squad inside a building gets ammunition only, no replacements, because its seats are fixed per soldier. An eliminated squad (nobody standing) is never served.
- **The gap:** The contract required new ids and kept corpses, but not where replacements stand or what happens in buildings.
- **The reach:** Squad strength after resupply, and garrisons.
- **Verdict:** sound.
- **Confidence:** medium.

### Scenarios can start units worn and trucks with a set stock
- **When:** slice 13.
- **The choice:** A unit in a scenario can start with:
  - lower vehicle health;
  - soldiers already fallen (their records lying in formation);
  - rounds already spent, by weapon.
  A supply truck can start with any stock. These exist for labs and for authoring the encounter.
- **The gap:** The lab needed "a damaged tank, depleted AT squad, casualty rifle squad, empty truck", which the scenario format could not express.
- **The reach:** Labs and slice 15's encounter authoring.
- **Verdict:** sound.
- **Confidence:** high.

### "Not firing" means not fighting at all
- **When:** slice 13.
- **The choice:** A unit counts as firing, and so isn't served, while any of its weapons is aiming at, reloading on, turning toward or firing at a target, or is guiding a missile. It does not count only on the instant a round leaves. Likewise, "stationary" means no movement order at all: a tank turning on the spot to drive off is already moving. Supply trucks are never serviced, not even by another truck.
- **The gap:** L04 says "stationary and not firing" without saying whether the gaps between shots count.
- **The reach:** Squads must break off a fight to be resupplied, which is the rotation L04 wants.
- **Verdict:** sound.
- **Confidence:** medium.

### A unit waits for its next item in order, even if something cheaper could be paid
- **When:** slice 13.
- **The choice:** Service follows the contract's order: ammunition, then vehicle health, then soldiers. If the truck cannot pay for the next item, the unit waits with "no stock", even when a cheaper later item would fit. For example, an AT team whose next missile costs 20 still waits when 15 stock would buy a soldier. A repair point costs a whole point of stock even when less than one point is missing.
- **The gap:** The contract gives the order but not what happens when stock is short.
- **The reach:** The last few points of a truck's stock.
- **Verdict:** needs review. Skipping to what can be paid would use stock more fully, but it would reorder service.
- **Confidence:** medium.

## Slice 14 — weapon readouts

### What a weapon's ring shows
- **When:** slice 14.
- **The choice:** Above each own unit, a dark box holds one ring per weapon. Each ring shows:
  - a dashed amber arc while reloading (the empty ring is the track);
  - a solid cyan inner arc while aiming at a target;
  - in the middle, the rounds left of the loaded kind, prefixed AP or HE on the tank cannon, with ∞ for unlimited;
  - a small caption naming the weapon (CANNON, HMG, RIFLES, GREN, ATGM);
  - an upper badge while guiding a missile;
  - a lower badge carrying a glyph for why it cannot fire (out of range, no clear shot, holding fire…).

  Plain progress (firing, aiming, reloading) gets no badge. Finished timers vanish. A supply truck adds a square: ▲ setting up, ▼ packing, ✓ set up. When the camera is farther than 700 m, rings stay only over selected units; the selection panel keeps every detail in words at any zoom.
- **The gap:** U02 and U03 fixed rings, numbers and the guidance icon, but not how reasons, weapon identity or zoom appear.
- **The reach:** The village battle UI.
- **Verdict:** sound.
- **Confidence:** medium.

### Keys and right-click commands
- **When:** slice 14.
- **The choice:**
  - Right-click on an identified enemy attacks it.
  - A and G arm attack-move and attack-ground for the next right-click on the ground, then revert to plain move. Escape disarms them.
  - E toggles the selection to "return fire only", or back to "fire at will" if all of it already holds.
  - S stops.
  - Keys are ignored while typing.
  - The command bar shows every one of these, plus Deploy, Pack and Leave building. Garrison stays on right-clicking a building.
- **The gap:** The contract named the keys but not how a mode ends or what E does to a mixed selection.
- **The reach:** Every player command.
- **Verdict:** sound.
- **Confidence:** medium.
