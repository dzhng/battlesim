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
2. **Opening wear stays in the purchase contract.** Skirmish admission refuses
   authored forces, and player command entitlement comes from the entered
   purchase receipt. Owner-tagged `UnitSetup` rows therefore cannot replace the
   lab opening without adding a second fielding path. `OpeningPurchase.condition`
   and `PendingPurchase.condition` remain; every present condition is included
   in the reservation digest, including before the unit arrives.
   *(`contract::players::validate_player_contract`, `skirmish::digest`)*
3. **An authored AI opening readies after its purchase batch.** That batch
   bypasses the normal doctrine decision, so it includes the preparation-ready
   command the normal decision would otherwise supply. This lets the supply-AI
   lab start its match with the authored damaged group; ordinary purchases still
   use the doctrine's readiness path. *(`battle/skirmish_turn.rs`)*

## Provisional

- **After a refused far side the heli goes home, and if the need lasts it is sent again.** It may shuttle between its post and the same refused spot until the group recovers or a truck reaches it. Simple and visible; revisit if it reads badly in play.

4. **The blur-to-blades switch is at rotor speed 0.2.** It was picked from a
   still strip of a slowing rotor. Judge it in motion. Rust and TS deliberately
   mirror the threshold; the existing draw guard stays. *(`units::rotor::BLUR_CROSSOVER`, mirrored in `scene-assets/articulation.ts`)*
5. **A hovering heli with nowhere to land searches again every tick.** An
   inbound mission returns to its support post when its far-side goal is
   refused or unroutable. Each search is bounded (40 m at 1 m steps), which is
   fine for a few helis. Revisit if a battle ever fields many stuck at once.
6. **The rotor's shadow appears only once the blades draw as solid
   geometry.** Up close the thin blades' shadow breaks into soft blobs, the
   same as the Armory's parked helicopters. At normal zoom it reads as a plain
   shadow.
7. **The AI aims the airlift away from enemies it sees now**, not every enemy
   it remembers.

## Settled

8. **A landing type has one contract accessor.** `UnitType::landing` returns
   its authored drop and rotor clearance together. Admission requires rotor
   flight; `deployment::initial` converts the drop to ticks, and
   `Deployment::lands` answers whether a unit lands at all. Flight, low hover
   and the AI read the same type accessor.
9. **The drop time is counted in the same steps as set-up progress**, so the
   two compare directly. *(`deployment.rs::initial`)*
10. **The heli becomes a ground vehicle at the end of its drop**, not at full
    set-up, and goes back to being an aircraft when packing reaches zero.
11. **The height it drops from is remembered**, and is part of the replay
    digest, since it decides the next moments of the battle.
12. **It picks its landing spot only once it has stopped.** Told to stop at
    cruise, it glides first, then searches from where it came to rest.
13. **The landing spot belongs to flight.** The flight law steers the short
    hop directly to `landing_goal`; order goals and routes contain only orders.
    Every enqueue, including a queued order, clears the checked spot. Deployment
    advances only at a checked spot, posted to set up, with no order goal and
    no horizontal glide. A stationary packed aircraft searches if it holds none.
    Candidates must also have a direct leg clear of physical air-blocking
    footprints. Air navigation owns that check, permits leaving an obstructed
    start, and excludes routing padding.
    A refused landing still follows normal flight height and facing. A packed
    hop holds at the low hover before translating sideways.
14. **The new state is folded into the digest only while it exists** (the
    landed flag, the drop height, the landing goal). Units that never land
    keep their digests.
15. **One ring search serves both the crash wreck and the landing.** The wreck's
    resting place is unchanged byte for byte. *(`crash::nearest_clear_spot`)*
16. **The hull owns push class.** Ground movement and landed-aircraft readers
    use `Unit::push_class`. Ground motion stores no copied mobility record;
    navigation receives a fresh catalog-derived query. This removes the
    persistent second push-class owner without a wrapper or aircraft ground
    mobility. The ownership refactor preserves battle digests.
17. **Two words for "in the air".** "Flies" means its motion is flight;
    "airborne" means it flies and is off the ground now. Carriage's
    loading-height test uses "airborne". *(`Unit::flies`, `Unit::airborne`)*
18. **Enemy memory updates what a unit is at every sighting**, and the replay
    digest includes it. A last-seen contact keeps the layer it was last seen in.
19. **Enemies get set-up progress, its target, and "standing on the ground".**
    Target remains in the shared publication codec; grounded distinguishes the
    grounded pose from a descent at the same progress, and presentation reads
    grounded, progress and timings for rotor speed. Own and identified rows
    share deployment encoding and decoding.
20. **One eased progress curve drives grounded rotor speed**, regardless of
    deployment direction, so reversals are continuous. Airborne rotors run at
    full speed. Drawing and sound read the same result; the presentation feed
    carries grounded state and progress, while deployment target remains only
    on the wire publication.
    *(`units::rotor::rotor_speed`)*
21. **Articulation carries `rotor_speed`, a number from 0 to 1**, in Rust, the TS mirror and the pose capture. The effect style
    named `rotor_blur` is a separate thing and kept its name.
22. **Each client side owns one current-flight predicate.** A type `flies`;
    `inTheAir` in scene-assets and `UnitType::in_the_air` in presentation combine
    that with published grounding. Current-pose flags retain `airborne`.
23. **The rotor loop's volume and pitch scale with rotor speed**, silent when
    stopped. Jets and ground vehicles are untouched.
24. **Every vehicle class a catalog derives must have a sound row.** This
    replaces a test that pinned rows by weight.
25. **The AI's standing supplier is one that drives.** "Drives" means the type
    has ground locomotion. Only drivers count as "a supplier can reach it";
    the reserve heli is what answers the emergency.
    `standing_supplier` and `flying_supplier` own the two supplier jobs.
    *(`skirmish_ai::purchase`)*
26. **The airlift lives in one AI module**, covering combat groups, the trigger,
    the far side, the mission and the purchase response. The heli counts as
    arrived anywhere within the landing reach of its goal, plus 8 m.
    An emergency suppresses shopping only when a flyer is sent, bought, or
    pending. Inbound retains the original need and returns if that recipient
    disappears or the group no longer needs airlift; absent flyers without a
    reservation release the mission. Returning finishes on arrival within
    reach of its support post, without waiting for setup; a refused landing
    elsewhere keeps it flying home. The
    outbound goal follows the needy unit's current position rather than a stored
    destination; the far-side point is relative to the needy unit, and a
    serving flyer relocates when its needy recipient leaves service range.
    *(`skirmish_ai/airlift.rs`)*
27. **The tuning is data**: ammo share 0.5, health share 0.5, reach 30 s.
    *(`fixtures/skirmish-ai.json`)*
28. **Each transport and supply variant extend a shared abstract airframe.** All 139 existing unit rows resolve
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
    - an empty passive opponent in supply-AI, so ongoing fire cannot consume
      the reserve before the authored damage is repaired;
    - scene pages closed between variants (the browser stalls after about six
      open WebGPU pages).

    The lab doesn't draw props added by an encounter's events, so the
    `heli-shift` and `heli-crowded` crates act but can't be seen. That predates
    this feature.
35. **The size ratchet is met by moving whole concepts.** Type semantics,
    soldier threat evaluation, doctrine state, publication codecs and domain
    test suites live with their own concepts. Callers consume those owners
    directly; there are no forwarding modules or size-limit exemptions.
