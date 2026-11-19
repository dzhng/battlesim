"""Ka-52M Alligator, from assets/references/ka_52_alligator/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/ka52.py -- [--wreck]

What the photos settle: two three-blade rotors stacked coaxially on one
mast (no tail rotor), the wide side-by-side cockpit under a big glazed
canopy, the GOES sight ball on the cockpit roof and the sensor turret under
the nose, the 30 mm cannon on the right flank, engines either side of the
gearbox, broad stub wings with two stations each and wing-tip pods, the
tail with a horizontal stabiliser carrying end-plate fins; retractable
tricycle gear. Camouflaged green, sand and black over a pale blue belly.

The airframe, cockpit, gear, cannon and stores are 42manako's CC BY Ka-52
(`skins/ka_52_alligator.json`), fitted to the catalog frame (rotorcraft
rule: fuselage length without blades, width over the wing-tip pods, height
to the top of the upper rotor head): 13.65 x 7.3 x 4.85 m. The rotors, mast
and markings are built here.

The cannon on the right flank barely traverses, so it is fixed in the hull
with the missiles; the airframe turns to bring them to bear.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from parts import cyl, flat_paint  # noqa: E402
from vehicle_export import run  # noqa: E402

# The donor's coaxial column: its axis, where it leaves the roof, and its
# two hubs. The mast's top is the frame's height.
MAST_X, ROOF_Z, LOWER_Z, UPPER_Z, TOP_Z = 1.70, 3.0, 3.57, 4.75, 4.85


def build(variant, v):
    m = A.fit(v)
    hull = v.hull
    m["canopy"] = flat_paint("ka52_canopy_glass", (0.018, 0.028, 0.034), rough=0.13,
                             grime=0, coverage=("blended", 0.38), role="glass")
    fitted_airframe(v, "ka_52_alligator", m)
    cyl("mast", 0.16, TOP_Z - ROOF_Z, (MAST_X, 0, (TOP_Z + ROOF_Z) / 2), "Z", m["dark"], hull, seg=16)
    A.rotor("lower", (MAST_X, 0, LOWER_Z), 7.25, 3, 0.5, m, hull, hub=0.42, mast=0.0, droop=0.02, phase=0.0)
    A.rotor("upper", (MAST_X, 0, UPPER_Z), 7.25, 3, 0.5, m, hull, hub=0.42, mast=0.0, droop=0.02, phase=1.047)
    # The donor's tail already carries its red star; a bort number on the
    # engine fairing, as the photos show it.
    A.markings(v, [
        ("text", dict(text="12", height=0.36, centre=(2.2, 2.0, 2.5), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage", "airframe"), colour="yellow")),
    ])


def wreck(variant, v):
    A.rotorcraft_crash(v, lifting=("upper", "lower"), tail_x=-2.8, tail_yaw=0.35, tail_drop=0.1, seed=52)


run("ka52", "russian_air_grey", build, wreck, references="assets/references/ka_52_alligator/references.json",
    authoring="Adapted from 42manako's CC BY 4.0 Russian Ka-52 Alligator (blender/skins/ka_52_alligator.json).")
