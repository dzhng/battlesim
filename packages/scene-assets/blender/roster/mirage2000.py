"""Mirage 2000D (RMV), from assets/references/mirage_2000/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mirage2000.py -- [--wreck]

What the photos settle: the two-seat canopy of the strike Mirage, its fixed
probe on the right of the windscreen, half-round intakes with their shock
cones either side of the cockpit, the pure delta wing low on the body, one
tall fin, the single M53 nozzle; big tanks under the wings, bombs under the
body. French strike camouflage, grey with dark grey-green patches.

Dimensions stated from the references (Dassault figures): length 14.36 m,
span 9.13 m, height 5.2 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"europe_mirage_2000_2000d_rmv": (14.36, 9.13, 5.2)}
WING_Z = 1.72
SPEC = dict(
    fuselage=[(7.18, 0.0, 1.9, 1.9), (6.5, 0.34, 1.65, 2.15, 1.9), (5.4, 0.5, 1.5, 2.36, 1.88, 2.2),
              (4.0, 0.56, 1.44, 2.5, 1.86, 2.4), (2.5, 0.62, 1.42, 2.5, 1.85, 2.6), (0.0, 0.74, 1.4, 2.4, 1.85, 2.6),
              (-3.0, 0.72, 1.45, 2.3, 1.85, 2.6), (-5.4, 0.6, 1.55, 2.2, 1.86, 2.4), (-6.4, 0.52, 1.6, 2.12, 1.86)],
    bodies=[(f"fuselage_intake_{s}", [(2.75, 0.42, 1.42, 2.26, 1.84), (0.6, 0.44, 1.42, 2.28, 1.84),
                                       (-1.2, 0.0, 1.84, 1.84)]) for s in "LR"],
    canopy=dict(x_front=5.5, x_back=2.6, sill=2.42, top=2.96, half_width=0.42, bows=(4.0,), peak=0.4),
    wing=[(2.0, 0.72, WING_Z, 7.6, 0.4), (-4.3, 4.56, WING_Z, 0.9, 0.05)],
    fins=[dict(root_x=-2.6, root_z=2.3, height=2.9, root_chord=3.8, tip_chord=1.2, sweep_m=2.8)],
    intakes=[("round", (2.78, 0.82, 1.84), 0.38, 0.16), ("round", (2.78, -0.82, 1.84), 0.38, 0.16)],
    nozzles=[dict(loc=(-6.4, 0.0, 1.86), r_front=0.52, r_exit=0.48, length=0.8, petals=14)],
    gear=[dict(name="nose", x=4.5, y=0.0, top=1.2, radius=0.25, width=0.14, wheels=2, spread=0.2),
          dict(name="main_L", x=-0.6, y=1.6, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.5)),
          dict(name="main_R", x=-0.6, y=-1.6, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.5))],
    pylons=[((-0.4, 2.0, WING_Z - 0.06), 1.2, 0.3)],
    stores=[("tank", (-0.5, 2.0, 0.98), 4.6, 0.4), ("bomb", (0.6, 0.45, 0.98), 2.4, 0.2),
            ("bomb", (-2.0, 0.45, 1.02), 2.4, 0.2)],
)


def build(variant, v):
    s = dict(SPEC)
    bodies = s.pop("bodies")
    m = A.jet(v, s)
    hull = v.hull
    for name, stations in bodies:
        A.body(name, stations, m["paint"], hull, seg=16, loc=(0, 0.82 if name.endswith("L") else -0.82, 0))
    # The intakes' shock cones.
    A.mirrored(lambda side, k: cyl(f"shock_cone_{k}", 0.2, 0.55, (3.0, side * 0.82, 1.84), "X", m["paint"], hull,
                                   seg=14, r2=0.02))
    cyl("probe", 0.05, 1.1, (5.9, -0.34, 2.5), "X", m["steel"], hull, seg=8, lods=MID)
    cyl("probe_base", 0.08, 0.35, (5.3, -0.34, 2.45), "X", m["paint"], hull, seg=10, lods=MID)
    box("antiglare", (1.0, 0.5, 0.03), (6.1, 0, 2.25), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-0.8, 0, 2.4), 0.22, m, hull)


def wreck(variant, v):
    A.crash(v, tail_x=-3.2, wing_y=2.8, wing_side=-1, tail_yaw=-0.3, seed=20)


run_disabled("mirage2000", CARDS, "french_air_camo", build, wreck)
