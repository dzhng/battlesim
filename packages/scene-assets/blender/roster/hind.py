"""Mi-35M airframe adapted from the Mi-24. Repainted the exterior, opened and glazed the tandem cockpits, seated the landing gear and crew, and rebuilt both rotor assemblies."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe
from parts import box, cyl, flat_paint
from vehicle_parts import MID
from vehicle_export import run, rig


def build(variant, v):
    m = A.fit(v)
    hull = v.hull
    # The donor has separate cockpit and cabin panes. Keep the shared glass
    # role, but give these armored windows a controlled blue-grey tint and
    # enough coverage for the crew and seat silhouettes to read through them.
    m["canopy"] = flat_paint("hind_laminated_glass", (0.018, 0.028, 0.034), rough=0.13,
                             grime=0, coverage=("blended", 0.38), role="glass")
    fitted_airframe(v, "mi_24_mi_35_hind", m)
    if not v.wreck:
        # The donor canopy is the shell. These restrained interior pieces sit
        # below its sills so the tandem cockpit reads as occupied through the
        # glazing instead of as a black cavity.
        for label, x, z in (("front", 5.91, 1.49), ("rear", 7.48, 1.08)):
            box(f"cockpit_seat_{label}", (0.18, 0.46, 0.62), (x, 0, z), m["cockpit"], hull,
                bevel=0.025, rot=(0, -0.12, 0), lods=MID)
            box(f"cockpit_headrest_{label}", (0.16, 0.34, 0.20), (x - 0.05, 0, z + 0.38), m["black"], hull,
                bevel=0.018, lods=MID)
            box(f"cockpit_coaming_{label}", (0.42, 0.62, 0.12), (x + 0.30, 0, z + 0.32), m["cockpit"], hull,
                bevel=0.025, rot=(0, -0.08, 0), lods=MID)
            box(f"cockpit_panel_{label}", (0.05, 0.48, 0.16), (x + 0.50, 0, z + 0.27), m["dark"], hull,
                bevel=0.012, rot=(0, -0.18, 0), lods=MID)
        for name, x, z in (("gunner", 7.48, 1.08), ("pilot", 5.91, 1.49)):
            v.crew.append(
                (
                    name,
                    hull,
                    (x, 0, z),
                    ((x + 0.3, 0.15, z + 0.24), (x + 0.31, -0.15, z + 0.25)),
                    ((x + 0.54, 0.15, z - 0.34), (x + 0.54, -0.15, z - 0.34)),
                )
            )
    A.rotor("main", (1.83, 0.13, 3.80), 8.65, 5, 0.58, m, hull, hub=0.46, mast=0.45, droop=0.03, phase=0.10)
    A.rotor(
        "tail",
        (-7.53, 0.40, 3.69),
        1.66,
        4,
        0.24,
        m,
        hull,
        hub=0.17,
        mast=0.18,
        droop=0,
        rot=(-math.pi / 2, 0, 0),
        thick=0.12,
    )
    # The Mi-35M's chin turret sits in a short armored fairing. Giving the
    # receiver a shaped cheek keeps the twin barrels from reading as a loose
    # tube below the nose.
    A.body("chin_fairing", [(7.72, 0.0, 0.66, 0.66), (7.48, 0.22, 0.56, 0.92, 0.82),
                            (7.18, 0.28, 0.58, 0.98, 0.84), (6.98, 0.18, 0.68, 0.88, 0.78)],
           m["paint"], hull, seg=16)
    A.chin_gun(v, rig(v.frame, v.root), m, (-40, 13), 0.05, drum=0.26, barrels=(0.07, -0.07))
    A.markings(
        v,
        [
            (
                "insignia",
                dict(
                    kind="ru_star",
                    centre=(-3.05, 1.2, 1.77),
                    normal=(0, 1, 0),
                    up=(0, 0, 1),
                    size=0.39,
                    onto=("fuselage",),
                ),
            ),
            (
                "text",
                dict(
                    text="51",
                    height=0.24,
                    centre=(4.21, 1.35, 2.55),
                    normal=(0, 1, 0),
                    up=(0, 0, 1),
                    onto=("fuselage",),
                    colour="yellow",
                ),
            ),
        ],
    )


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-3.4, tail_yaw=-0.38, tail_drop=0.08, seed=24)


run(
    "hind",
    "russian_air_grey",
    build,
    wreck,
    crew_army="eastern_emr",
    references="assets/references/mi_24_mi_35_hind/references.json",
)
