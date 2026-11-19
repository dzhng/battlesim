"""Adapted into AH-6M and MH-6M variants with separate glazing, seated crew, animated rotors, and distinct side benches and stores."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe
from parts import flat_paint, box, cyl
from vehicle_export import run, rig


def build(variant, v):
    m = A.fit(v)
    hull = v.hull
    m["canopy"] = flat_paint(
        "bird_glass", (0.009, 0.020, 0.054), rough=0.10, metal=0.30, grime=0, coverage=("blended", 0.61), role="glass"
    )
    fitted_airframe(v, "ah_6_mh_6_little_bird", m)
    if not v.wreck:
        for s in (-1, 1):
            x, y, z = 2.31, s * 0.32, 0.83
            v.crew.append(
                (
                    "crew_" + str(s),
                    hull,
                    (x, y, z),
                    ((x + 0.30, y + 0.15, z + 0.24), (x + 0.30, y - 0.15, z + 0.24)),
                    ((x + 0.40, y + 0.13, z - 0.25), (x + 0.40, y - 0.13, z - 0.25)),
                )
            )
    A.rotor("main", (1.71, 0, 2.35), 4.17, 6, 0.20, m, hull, hub=0.24, mast=0.38, droop=0.02, phase=0.25)
    A.rotor(
        "tail",
        (-3.24, 0.34, 1.73),
        0.70,
        4,
        0.10,
        m,
        hull,
        hub=0.08,
        mast=0.18,
        droop=0,
        rot=(-math.pi / 2, 0, 0),
        thick=0.12,
    )
    for s in (-1, 1):
        if variant["id"].endswith("_mh_6m"):
            # Two longitudinal slats leave daylight through the rescue bench.
            for j, offset in enumerate((1.09, 1.37)):
                box(f"bench_{s}_{j}", (1.35, 0.24, 0.075), (1.52, s * offset, 0.74),
                    m["dark"], hull, bevel=0.023)
            for j, x in enumerate((1.02, 1.53, 2.03)):
                box(f"bench_reinforcement_{s}_{j}", (0.055, 0.51, 0.05),
                    (x, s * 1.23, 0.69), m["gear"], hull, bevel=0.009)
            for k, x in enumerate((1.08, 1.94)):
                A.line(f"bench_brace_{s}_{k}", (x, s * .77, .46), (x, s * 1.39, .68),
                       m["dark"], hull, width=.039, height=.050)
                A.line(
                    "bench_arm_" + str(s) + "_" + str(k),
                    (x, s * 0.70, 0.67),
                    (x, s * 1.39, 0.69),
                    m["steel"],
                    hull,
                    width=0.062,
                    height=0.065,
                )
        else:
            box("plank_" + str(s), (0.43, 0.84, 0.085), (1.46, s * 1.11, 0.76), m["dark"], hull, bevel=0.015)
            A.store("rocket_pod_" + str(s), "rockets", (1.47, s * 1.45, 0.53), 1.4, 0.19, m, hull)
            cyl("minigun_" + str(s), 0.08, 0.9, (1.75, s * 1.10, 0.55), "X", m["steel"], hull, seg=10, lods=A.MID)
    if variant["id"].endswith("_mh_6m"):
        A.door_guns(v, rig(v.frame, v.root), m, (-60.0, 20.0), inboard=0.4)

    A.markings(
        v,
        [
            (
                "text",
                dict(
                    text="U.S. ARMY",
                    height=0.095,
                    centre=(-1.42, 0.50, 1.63),
                    normal=(0, 1, 0),
                    up=(0, 0, 1),
                    onto=("fuselage",),
                    colour="lowvis_dark",
                ),
            )
        ] + ([("text", dict(text="25381", height=.115, centre=(1.31, .42, 1.84),
                           normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="lowvis_dark"))]
             if variant["id"].endswith("_mh_6m") else []),
    )


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-1.6, tail_yaw=0.5, tail_drop=0.12, seed=6)


run(
    "little_bird",
    "us_army_aviation",
    build,
    wreck,
    crew_army="us_army_ocp",
    references="assets/references/ah_6_mh_6_little_bird/references.json",
)
