# 14 Wheeled, light and trucks

**Unlocks:** every runtime wheeled vehicle at the pilot's bar.

Families: LAV (25A2, AT), Boxer (APC, RCT30), VBCI, BTR-82A, ZBL-08, ACV-P;
HMMWV M1151, Tigr-M, Fennek, VBL; HEMTT M977, MAN HX, Ural-4320.

## Per family

As [slice 13](13-tracked.md), with wheeled specifics: axle count and spacing
from the photos (BTR-82A four axles with the gap between the second and
third; Boxer four evenly; VBL and Fennek two), tyre size and tread pattern
(military bar tread for the trucks, run-flat road tread for the APCs), rims at
their true diameter, CTIS hubs where fitted, mudflaps, spare wheels. Trucks get
cab interiors only as dark glass shows them (seats and dash silhouettes), real
cargo beds and tarps; HEMTT keeps its folded crane.

**Crew from the generic jeep.** The jeep's visible crew
(`blender/vehicle_crew.py`, commit `349fe766`) is reused, not re-made: every
roster vehicle whose references show an exposed gunner or commander (HMMWV
M1151 turret gunner, Tigr-M and VBL gunners, open hatches where photos show
them) gets crew through that module, in its existing fixed poses. Crew stay
presentation inside the vehicle's appearance, not simulation soldiers. Fix the
module first (coordinator-owned):

- source soldier per faction, not the hard-coded `assets/source/infantry/rifle.glb`
  (`vehicle_crew.py:18`, which slice 05 relabels as test art);
- crew from the soldier's tier 1, counted in the class budget;
- hide the carried weapon by node or material role, not by the material name
  `gun_black` plus `hand_r` weight (`:60-70`), which slice 15 may rename;
- `preserve_materials` (`:133-144`) must not overwrite the vehicle's own
  role-tagged materials (slice 10).

Crew vanish when the wreck appears (`cookOffs.ts` has no crew handling):
accepted; they were inside.

**Own wreck.** Each variant exports its own wreck (slice 09): the burnt hull
at the unit's size, plus a turret piece where it has a turret, replacing its
interim wreck.

## Verify

As slice 13. User preview by group (wheeled armour, light, trucks), non-blocking as there.
