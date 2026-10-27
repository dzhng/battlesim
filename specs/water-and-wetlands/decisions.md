# Decisions

These are the user's decisions from the 2026-10-27 unknowns walk
(`explore-unknowns`), each with its reason. Slices carry them as givens. Entries
marked *agent* were answered on the user's behalf from their confirmed
preferences, and the user may overrule them.

**Confirmed preferences** (they scope the *agent* entries):

- Easy to balance comes first. Each number means exactly what it says.
- A filmic distinction is worth a number when it changes which unit is right for
  the job.
- Land that reads as real beats simpler generation.
- Otherwise, take the recommendation.
- Where the user picks several looks, the map uses them all as variants.

## Terrain and rule

1. **Wetlands dig their own shallow dip.** Each puddle or swamp carries a depth.
   The ground lowers inside its outline with soft edges, and the water sits flat
   at the old ground level. This joins the river and bridge carving in
   `crates/sim/src/world/carve.rs`; it is not a second terrain owner. The
   simulation's height, slope and line of sight see the dip too. *Why:* it works
   on any ground, and the simulation and the look read the same depth.
2. **The slowdown comes from the wetland's kind, not its depth.** Each wetland
   kind has a row in the game rules, and the whole area slows by it, from edge to
   middle. Depth is only for the terrain and the look. The route planner counts
   slow tiles by kind (`navigation.rs` `SlowGround`). *Why:* depth-driven speed
   would be hard to balance.
3. **Each kind has one factor per drive:** foot, wheeled and tracked, from
   `Mobility` in `crates/contract/src/catalog.rs`. Puddles and swamps make six
   numbers. Air units are unaffected. *Why:* trucks bogging down worse than tanks
   is the film picture, and it makes choosing the unit for a swamp part of the
   game. Forest's existing two-way split (`forest_infantry_multiplier`,
   `forest_vehicle_multiplier`) is unchanged.
4. **A road across a wetland keeps road speed. Where a wetland and a forest
   overlap, the slower factor applies.** Craters stack on top as they do today.
   *Why:* each number means what it says. A road over a swamp reads as a
   causeway.
5. **Placement:** swamps form in flat ground near the river, and sometimes away
   from it on maps with no river. Puddles scatter along dirt tracks and open
   fields. Each map type in `fixtures/map-presets.json` has its own chances, like
   `river_chance`. Swamps stay clear of settlements, buildings and bridge
   approaches. *Why:* it reads as real land, and it can be tuned in the map
   workbench.
6. **Swamps give no concealment.** They are slow, open ground. No sight rule
   changes. Any reeds drawn stay knee-high, so the screen never suggests cover the
   rule doesn't give. *Why:* crossing a swamp is a clear risk taken for a
   shortcut.
7. *agent* **Wetlands take no track marks,** as water already refuses them
   (`crates/sim/src/ground.rs`). Marks on the dry ground around a wetland still
   show. *Why:* water keeps one rule for marks, and the wake is the trace a unit
   leaves in water.
8. *agent* **A hull in standing water raises no dust; it makes a wake instead.**
   *Why:* this is the film picture. Today dust ignores the ground type
   (`presentation/src/effects/frame.rs` `note_movers`).

Rivers stay impassable (`specs/done/battle-foundation/requirements.md`). This
feature adds no fording.

## Look

9. **River water has two variants: temperate green-grey and murky lowland.** No
   tropical turquoise. "Same quality as RedGPU" means the same techniques
   (depth colour, bed, refraction, caustics, ripples) in our palette, not their
   colours.
10. **Swamps have two variants: flooded meadow and low reed marsh.** Reeds stay
    knee-high (decision 6).
11. **Puddles have two variants: muddy and mirror.**
12. **How a variant is chosen:**
    - each river and swamp gets one variant, chosen by the map seed from weights
      in the biome data;
    - a puddle's variant follows the ground under it: mirror on paved roads and
      hard tracks, muddy on dirt tracks and fields;
    - variants are look only, and the speed factors stay the same.

    *Why:* replays stay exact, two swamps on one map can differ, and puddles
    read as cause and effect.
13. **Wakes show clear rings, plus a V-wake once the unit moves.** The style
    follows the unit's speed: standing or wading slowly gives rings, and a
    moving hull or a squad on the move drags a V. *Why:* this is what a viewer
    expects, and it needs no placement rule.
14. *agent* **The wake area follows the camera and is sized to the tactical view.
    Wakes fade out as you zoom far out.** The exact size comes from a cost
    measurement in the wake slice. *Why:* at far zoom a wake is smaller than a
    pixel.
15. *agent* **Screen-space reflections are not in the first version.** They are
    reconsidered after the highlights slice, on the menu reel's low shots. *Why:*
    from a steep RTS camera most reflected rays leave the screen.
16. *agent* **Normal maps are generated in a shader once at load,** not shipped as
    image files. *Why:* RedGPU's maps carry no licence, and a generated texture
    comes through the shared shader export, so both clients get it with no new
    runtime assets.

## Scope and acceptance

17. **The water must look its best close up (~65 m), at tactical play (~260 m)
    and in the menu reel's low angles.** Far zoom only has to not look wrong.
18. **The Low graphics setting drops refraction, caustics and wakes, and keeps
    the depth colour.**
19. **Browser first; native catches up step by step.** This replaces the
    placeholder's "both clients in the same step".
20. **Done is a side-by-side read:** at our camera, next to RedGPU, ours reads as
    good. Slice 0's captures are the target for that comparison.

## From the landmine sweep

The sweep of unknown unknowns (2026-10-27) covered the simulation and map
generation, the renderer and native, and presentation. The findings that need
no taste call are under [sharp edges](README.md#sharp-edges). These are the
ones decided from the confirmed preferences.

21. *agent* **Wetlands are shallow and wide-edged.** Depth stays well under the
    fog target height (`fog_target_height_m`, 1.0 m in `fixtures/game.json`),
    and the soft edge spans several 4 m height cells. Admission checks depth
    against edge width when the map loads. *Why:* a deep rim would hide units
    on flat ground, which is the concealment decision 6 rules out. A steep rim
    would add the slope slowdown (`slope_multiplier`, `navigation.rs`) on top
    of the kind's factor, so speed would leak back to depth (decision 2). A
    test proves sight is unchanged by a wetland.
22. *agent* **A wetland has a minimum size of about three height cells
    across.** Anything smaller can't be carved, and a hull's speed is sampled
    at its centre only. Smaller puddles are paint only (see question 8). *Why:*
    a puddle that slows some tanks and not others reads as a bug.
23. *agent* **Rounds landing in a wetland burst at the water's surface and leave
    no crater.** They make a splash, which is a later wake source. Wetlands
    refuse craters and track wear alike. *Why:* water keeps one rule for marks
    (decision 7). Today only `burst` refuses river water; `wear` has no surface
    check at all (`crates/sim/src/ground.rs`).
24. *agent* **A wake already on the water may keep showing a unit's heading for
    a few seconds after the unit leaves sight.** The wave field clears on a
    side switch, world reload or reset. *Why:* the side saw the wake being
    made, and dust puffs already behave the same way. Clearing on a side
    switch stops one side's waves leaking to the other.
25. *agent* **Water is drawn single-sampled after the MSAA resolve, unless rung
    2's cost probe shows that storing the samples is cheap.** *Why:* the soft
    shoreline blurs water edges anyway, and the world pass discards its MSAA
    samples on purpose to save bandwidth on Apple GPUs (commit `54bc94c03`).
    Glass moves after water and follows the same choice.
26. *agent* **The one wind keeps the grass heading (35°).** Smoke and dust drift
    turns about 8° to match. The record carries both gust speed and drift
    speed, because they are different quantities. *Why:* grass is the most
    visible wind in the menu reel's liked shots, so it stays put.
27. *agent* **Variants are picked from the biome seed plus each river's or
    wetland's identity,** as tree stands are seeded per cell. This needs a
    river index and a wetland index in the export. *Why:* presentation has no
    map seed, and this follows the existing pattern.
28. *agent* **A puddle's look follows the surface kind at its centre:** mirror
    on `road` and `paving`, muddy on `country_road`, `dirt_track` and open
    ground. *Why:* "hard track" wasn't a real kind; this maps decision 12 onto
    the kinds that exist.
