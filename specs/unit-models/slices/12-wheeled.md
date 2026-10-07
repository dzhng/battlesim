# 12 Wheeled, light and trucks

**Unlocks:** every runtime wheeled vehicle at the pilot's bar.

Families: LAV (25A2, AT), Boxer (APC, RCT30), VBCI, BTR-82A, ZBL-08, ACV-P;
HMMWV M1151, Tigr-M, Fennek, VBL; HEMTT M977, MAN HX, Ural-4320.

## Per family

As [slice 11](11-tracked.md), with wheeled specifics: axle count and spacing
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
them) gets crew through that module, in its existing fixed poses and with the
infantry art of its own faction. Crew stay presentation inside the vehicle's
appearance, not simulation soldiers. Their death presentation follows the
jeep's (`apps/battle-lab/src/cookOffs.ts`).

## Verify

As slice 11. User preview by group (wheeled armour, light, trucks), non-blocking as there.
