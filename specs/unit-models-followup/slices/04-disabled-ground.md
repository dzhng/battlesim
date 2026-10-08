# 04 Disabled cards at the roster bar: ground, support, drones, infantry

**Unlocks:** the 45 remaining disabled cards (artillery, air defence, drones,
deferred ground vehicles including the M1E3, CV90120, Centauro II,
T-14, T-15, Type 15, BRM-3K, Jaguar, Challenger 3, and the five infantry
cards) meet the roster bar, ready to become playable.

Raise every family to the roster bar (the unit-models detail bar and look):
truck cabs with real glazing, doors, mirrors and grilles; launcher and gun
systems at real shape (tubes, cradles, radars, outriggers); drones at their
real airframe and launcher; the M1E3's remote station and turret from its
references; infantry team weapons and kit per the infantry bar; real tiers,
own wrecks, slice 13 materials, national paint. Work family by family
through `vehicle_parts.py` / `vehicle_export.py`, committing each.

**Verify:** per family, re-export twice byte-identical; `asset validate`;
a reference sheet; compare-screenshots against references; critique last.

**Not here:** the M10 and the Stryker hull under M-SHORAD/M1129 (slice 01 owns them). Wreck debris comes from slice 02's shared helper once it lands: merge it in, then re-export your wrecks through it.
