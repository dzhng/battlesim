# 10 Pilot: Abrams SEPv3 and Stryker M1126

**Unlocks:** the detail bar proven on one tracked and one wheeled vehicle, and
the triangle budget every later family is held to.

Two vehicles, because tracked and wheeled share almost no running gear and
the budget differs by class.

## Work

1. **Start from the generic tank** (`blender/tank.py`): it is already an
   Abrams-like hull with tuned detail and the hull/turret wreck pieces the
   cook-off throws. Lift what matches the Abrams references into
   `vehicle_parts.py` and `roster/abrams.py`; whatever the photos contradict
   is rebuilt. Then rebuild `us_m1_abrams_sep_v3_trophy` from its references, entirely through
   `vehicle_parts.py` plus Abrams-specific geometry in `roster/abrams.py`:
   seven road wheels a side with return rollers, sprocket at the rear, idler at
   the front, skirts with their real panel breaks, turret with its real
   faceted cheeks, CITV, GPS, CROWS, Trophy radars and launchers as the
   existing static nodes, bustle rack with stowage, smoke dischargers,
   tow cables, tail lights, engine grille. Then SEPv2 and SEPv2 Trophy as
   variant branches of the same script.
2. Rebuild `us_stryker_m1126_icv`: eight black treaded tyres with CTIS hubs,
   the hull's real chine and slat-free sides, hatches, ramp, periscopes, RWS
   (M151 or CROWS per the reference), smoke dischargers, stowage. Then the
   other three Stryker variants as branches.
3. **Budget.** Measure each rebuilt vehicle's triangles per tier and the
   frame cost in a lab scene holding a column of the pilots, against slice 07's
   baseline of the same scene with the old models.
   Fix, on this Mac mini (the target machine), a per-tier triangle budget per class (MBT, IFV/APC, light, truck) with
   margin, record it in choices.md, and add it to validation as a vehicle
   budget finding so later slices are held to it. Detail beyond the budget
   comes out of tier 0 first.
4. Delete the old Abrams and Stryker branches from the legacy helpers.

5. Judge the pair against the [look target](../assets/look-target.png) as
   well as the references: bevels, chunky small parts, edge highlights.

## Verify

- validate, bake, check; mount fit at the yaw sweeps unchanged (validator).
- `asset sheet --references` for each of the seven appearances.
- compare-screenshots: after against before, and against references. Judge
  silhouette and detail placement; materials were judged in slice 08.
- screenshot-critique, unprimed, last.
- In-battle look: one lab scene with both pilots at the usual camera distance,
  to check that detail reads and the impostor still matches the mesh.
- **Human checkpoint:** show the user the sheets and the battle shot
  (preview-shots). This sets the bar for 30 more vehicles, so wait about five
  minutes for a response; if none, decide on the evidence, record it in
  choices.md, close the shots, and continue to slice 11.

## Feedback that would change this

"Too busy at battle distance" lowers tier 1–2 detail. "Still too plain"
raises the budget if the benchmark allows it.

## Delegated

Part sizes and positions within the references; tier reduction choices
within budget.
