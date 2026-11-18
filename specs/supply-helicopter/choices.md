# Supply helicopter: choices ledger

Every choice made during the build where the plan was silent, checked against
the shipped code at closeout. It is grouped by how sure the choice is, least
sure first. The planned decisions are in [`decisions.md`](decisions.md).

## Worth a look

1. **Where the landing search falls short.**
   - It treats a whole forest's bounding box as forest, so a clearing inside an
     irregular wood is refused.
   - It checks the ground only at the spot's centre, against the map's slope
     limit, not under the whole hull.
   - On a bridge it would touch down at the ground below, not on the deck.

   No supply-heli map has a bridge today. The forest and slope cases err toward
   "can't land here". *(`movement/air.rs::spot_blocked`)*
2. **A new contract field the plan didn't list.** A skirmish opening purchase
   (and the pending purchase it becomes) may carry a `condition`: authored
   starting wear such as low hp or spent rounds. Skirmish units arrive only by
   purchase, so this is the only way a lab can start with a damaged group. It
   mirrors what encounters already allow. *(`contract::players::OpeningPurchase`,
   `contract::skirmish::PendingPurchase`)*
3. **AI seats with an authored opening now ready up.** They send "ready" with
   their opening purchases, as any seat does. Without it the AI-airlift lab sat
   in preparation. The game's skirmish corpus and every skirmish test are
   unchanged; only labs with authored openings see a difference.
   *(`battle/skirmish_turn.rs`)*

## Provisional

4. **The blur-to-blades switch is at rotor speed 0.2.** It was picked from a
   still strip of a slowing rotor. Judge it in motion; it is one shared
   constant. *(`units::rotor::BLUR_CROSSOVER`, mirrored in `scene-assets/articulation.ts`)*
5. **A hovering heli with nowhere to land searches again every tick.** Each
   search is bounded (40 m at 1 m steps), which is fine for a few helis. Revisit
   if a battle ever fields many stuck at once.
6. **The rotor's shadow appears only once the blades draw as solid
   geometry.** Up close the thin blades' shadow breaks into soft blobs, the
   same as the Armory's parked helicopters. At normal zoom it reads as a plain
   shadow.
7. **The AI aims the airlift away from enemies it sees now**, not every enemy
   it remembers.

## Sound

8. **A helicopter "lands" when its deploy capability authors a drop.** Nothing
   else marks a lander. *(`deployment.rs`, `drop_duration > 0`)*
9. **The drop time is counted in the same steps as set-up progress**, so the
   two compare directly. *(`deployment.rs::initial`)*
10. **The heli becomes a ground vehicle at the end of its drop**, not at full
    set-up, and goes back to being an aircraft when packing reaches zero.
11. **The height it drops from is remembered**, and is part of the replay
    digest, since it decides the next moments of the battle.
12. **It picks its landing spot only once it has stopped.** Told to stop at
    cruise, it glides first, then searches from where it came to rest.
13. **The landing spot is an internal goal, not a player order.** It shows no
    order line or queue entry; a real order replaces it.
    *(`Unit::landing_goal`, hidden by `ordered_goal` / `ordered_route`)*
14. **The new state is folded into the digest only while it exists** (the
    landed flag, the drop height, the landing goal). Units that never land
    keep their digests.
15. **One ring search serves both the crash wreck and the landing.** The wreck's
    resting place is unchanged byte for byte. *(`crash::nearest_clear_spot`)*
16. **Ground rules read a landed heli's push class from its hull.** They never
    ask an aircraft for ground movement, so a landed heli is never shoved and
    fells no trees. *(`Unit::push_class`)*
17. **Two words for "in the air".** "Flies" means its motion is flight;
    "airborne" means it flies and is off the ground now. Carriage's
    loading-height test uses "airborne". *(`Unit::flies`, `Unit::airborne`)*
18. **Enemy memory updates what a unit is at every sighting**, and the replay
    digest includes it. A last-seen contact keeps the layer it was last seen in.
19. **Enemies get set-up progress, its direction, and "standing on the
    ground".** Without the direction the client couldn't tell spin-up from
    spin-down. The values are filled where the spotted unit's row is built.
20. **One helper turns those published facts into rotor speed.** The picture
    and the sound both read it. *(`units::rotor::rotor_speed`)*
21. **The articulation's `rotor_blur` switch became `rotor_speed`, a number from
    0 to 1**, in Rust, the TS mirror and the pose capture. The effect style
    named `rotor_blur` is a separate thing and kept its name.
22. **The client decides "in the air now" in one place.** `inTheAir` covers the
    marker, drop line and click rings; the game and the lab share these
    overlays.
23. **The rotor loop's volume and pitch scale with rotor speed**, silent when
    stopped. Jets and ground vehicles are untouched.
24. **Every vehicle class a catalog derives must have a sound row.** This
    replaces a test that pinned rows by weight.
25. **The AI's standing supplier is one that drives.** "Drives" means the type
    has ground locomotion. Only drivers count as "a supplier can reach it";
    the reserve heli is what answers the emergency.
    *(`skirmish_ai::purchase::drives`)*
26. **The airlift lives in one AI module**, covering combat groups, the trigger,
    the far side, the mission and the purchase response. The heli counts as
    arrived anywhere within the landing reach of its goal, plus 8 m.
    *(`skirmish_ai/airlift.rs`)*
27. **The tuning is data**: ammo share 0.5, health share 0.5, reach 30 s.
    *(`fixtures/skirmish-ai.json`)*
28. **Each transport became an abstract airframe that both the transport and
    the supply row extend.** All 139 existing unit rows resolve
    byte-identically. *(`fixtures/units/roster/*.json`)*
29. **Rotor radii come from the model build scripts**: 14.947 m for the
    Chinooks (the whole tandem), 10.997 m for the Mi-8. Re-measure if a model
    changes.
30. **Voice checks read the resolved catalog**, so abstract parents are skipped
    and inherited roster membership counts.
31. **In a shop tab, ground movers come before aircraft.** Only LOG mixes the
    two, so the truck stays first. *(`purchaseFilter.ts::shopOrder`)*
32. **The model exporter lets unit types that agree on hull and mounts share one
    model.** *(`catalog_frames.py::_variant`)*
33. **NO LANDING ZONE has its own icon (the set-down mark) in the warning
    colour.** It no longer looks like ROUTE BLOCKED.
34. **Battle-lab staging**:
    - a durable heli in `heli-fire`, so both halves of the demo play out;
    - the truck's wide camera on the newer heli variants;
    - scene pages closed between variants (the browser stalls after about six
      open WebGPU pages).

    The lab doesn't draw props added by an encounter's events, so the
    `heli-shift` and `heli-crowded` crates act but can't be seen. That predates
    this feature.
35. **Over-budget files gave up whole concepts to make room**, with no behaviour
    change:
    - `battle` → `spawn`;
    - `contract::catalog` → `inheritance` and `capabilities`;
    - `publication` → `identified_row` and `reader/identified`;
    - `skirmish_ai` → `support`, `doctrine`, `recon` and `airlift`;
    - `presentation` sound → `sound/frame/motion`;
    - several test files were split by subject.
