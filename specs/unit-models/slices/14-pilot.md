# 14 Pilot: one of each class

**Unlocks:** the look and detail bar proven end to end on one unit of each
budget class, and the measured class budgets every lane is held to. The
coordinator builds it; it is the go/no-go for the fan-out.

| Class | Pilot | Why this one |
|---|---|---|
| Tracked heavy | Abrams SEPv3 Trophy (then SEPv2, SEPv2 Trophy) | seeded from the generic tank; turret wreck pieces |
| Wheeled medium | Stryker M1126 (then M1127, M1134, M1296) | wheeled running gear; a turreted variant (Dragoon) |
| Wheeled light | HMMWV M1151 | its frame is wrong today; exposed gunner crew |
| Soldier | `rifle_squad` kit | the commonest soldier; sets the soldier budget |

## Work

0. **References first** for these four families (README [References](../README.md#references)),
   committed under the slice 09 schema. This is also the early test of
   reference scarcity and generated views before the lanes depend on them.
1. **Abrams. Start from the generic tank** (`blender/tank.py`): an Abrams-like
   hull with tuned detail, and a wreck cut into hull and turret pieces. Lift
   what matches the references into `vehicle_parts.py` and
   `roster/abrams.py`; rebuild what the photos contradict. Seven road wheels a
   side with return rollers, rear sprocket, front idler, skirts with their real
   panel breaks, faceted turret cheeks, CITV, GPS, CROWS, Trophy radars and
   launchers as the existing static nodes, bustle rack with stowage, smoke
   dischargers, tow cables, tail lights, engine grille. Commander in his hatch
   where the photos show it. Variants as branches.
2. **Stryker M1126:** eight black treaded tyres with CTIS hubs, the hull's real
   chine, hatches, ramp, periscopes, RWS per the reference, smoke dischargers,
   stowage. Variants as branches.
3. **HMMWV M1151 (decision 3).** Its source is the JLTV's file and its frame
   the JLTV's (6.2 × 2.5 × 2.6 m, eye 2.885, HMG pivot z 2.665). Give it its
   real frame (about 5.2 × 2.1 × 2.2 m, eye 2.42, pivot z 2.28 per the archived
   manifest; confirm against the references), export the real HMMWV, turret
   gunner through the fixed crew module (slice 16's rules). A physics change:
   digests move; a quick balance sample for the HMMWV only, named in choices.md.
4. **`rifle_squad` kit:** uniform, helmet, carrier and rifle per slice 17's
   bar, on the shared rig.
5. **Own wrecks** (slice 11) for every vehicle variant: Abrams with hull and
   turret pieces; Strykers whole, the Dragoon with a turret piece; HMMWV whole.
6. **Budgets.** Measure each pilot's triangles per tier, bundle bytes, texture
   layers, soldier card bake time, and frame cost in a lab scene holding a
   column of the pilots and a few squads, against slice 09's baseline of the
   same scene with the old models, on the Mac mini, in instructions retired or
   GPU counters, not wall time. Fix per class (tracked heavy, tracked medium,
   wheeled light, wheeled medium, logistics, soldier; the classes of slice 01):
   triangles per tier, the minimum tier reduction ratio (slice 12), bundle
   bytes within slice 10's limit, and texture layers a family may add. Record
   them in choices.md and fill them into slice 12's budget rules. Detail over
   budget comes out of tier 0 first. Vehicles have no impostor
   (`modelLayer.ts:1591`): judge tier 3 at the widest battle view. If tier 3
   at that view costs too much, a vehicle impostor is the named alternative;
   don't build one otherwise.
7. Judge against the [look target](../assets/look-target.png) and the
   references: bevels, chunky small parts, edge highlights.
8. Delete the old Abrams, Stryker and HMMWV/JLTV branches from the legacy helpers.

## Verify

- `asset validate`, bake, check; mount fit at the yaw sweeps (validator).
- `asset sheet --references` for every pilot appearance.
- compare-screenshots: after against the before sheets and the references;
  silhouette and detail placement (materials were judged in slice 13).
- screenshot-critique, unprimed, last.
- In-battle look: the lab scene at each zoom where a tier changes.
- The menu exact-shot test (slice 08) still passes: the menu's HMMWV-looking
  unit takes the HMMWV hull, which step 3 changes; retime the menu, never the
  shots, if it fails.
- **Human checkpoint:** show the sheets and battle shots (preview-shots). This
  sets the bar for ~150 models, so wait about five minutes; if no answer,
  decide on the evidence, record it in choices.md, close the shots, and open
  the lanes (slices 15–18).

## Feedback that would change this

"Too busy at battle distance" lowers tier 1–2 detail. "Still too plain" raises
the budget if the frame cost allows.

## Delegated

Part sizes and positions within the references; tier reduction choices within
budget.
