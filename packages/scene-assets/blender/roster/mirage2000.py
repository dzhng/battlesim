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
    # The side photo: the two-seat canopy 3 m behind the nose and 3.9 m
    # long, the spine 2.75 m up, the belly 1.1 m; the intakes' cones, the
    # fin and both gear further aft than drawn before.
    fuselage=[(7.18, 0.0, 1.9, 1.9), (6.5, 0.34, 1.65, 2.15, 1.9), (5.4, 0.5, 1.48, 2.38, 1.88, 2.2),
              (4.0, 0.58, 1.36, 2.56, 1.86, 2.4), (2.5, 0.66, 1.2, 2.66, 1.85, 2.6), (0.0, 0.76, 1.12, 2.7, 1.85, 2.6),
              (-3.0, 0.74, 1.2, 2.6, 1.85, 2.6), (-5.4, 0.6, 1.55, 2.2, 1.86, 2.4), (-6.4, 0.52, 1.6, 2.12, 1.86)],
    bodies=[*[(f"fuselage_intake_{s}", [(1.65, 0.42, 1.42, 2.26, 1.84), (-0.5, 0.44, 1.42, 2.28, 1.84),
                                         (-2.3, 0.0, 1.84, 1.84)], 0.82 if s == "L" else -0.82, 16) for s in "LR"],
            ("fuselage_spine", [(0.4, 0.36, 2.6, 3.0, 2.7, 2.4), (-2.5, 0.38, 2.55, 2.84, 2.65, 2.6),
                                (-5.2, 0.0, 2.4, 2.4)])],
    canopy=dict(x_front=4.2, x_back=0.3, sill=2.6, top=3.12, half_width=0.44, bows=(3.6, 2.0), peak=0.4, tail=0.7),
    wing_hinge=((0.85, 0, 1), 0.06),
    nav=dict(left=(-4.6, 4.45, WING_Z + 0.05), right=(-4.6, -4.45, WING_Z + 0.05), tail=(-7.1, 0.0, 2.0)),
    wing=[(2.0, 0.72, WING_Z, 7.6, 0.4), (-4.3, 4.56, WING_Z, 0.9, 0.05)],
    fins=[dict(root_x=-3.5, root_z=2.55, height=2.65, root_chord=3.6, tip_chord=1.2, sweep_m=2.45)],
    intakes=[("round", (1.68, 0.82, 1.84), 0.38, 0.16), ("round", (1.68, -0.82, 1.84), 0.38, 0.16)],
    nozzles=[dict(loc=(-6.4, 0.0, 1.86), r_front=0.52, r_exit=0.48, length=0.8, petals=14)],
    gear=[dict(name="nose", x=2.9, y=0.0, top=1.2, radius=0.25, width=0.14, wheels=2, spread=0.2, light=True),
          dict(name="main_L", x=-2.2, y=1.6, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.5)),
          dict(name="main_R", x=-2.2, y=-1.6, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.5))],
    pylons=[((-0.4, 2.0, WING_Z - 0.06), 1.2, 0.3)],
    stores=[("tank", (-0.5, 2.0, 0.98), 4.6, 0.4), ("bomb", (0.6, 0.45, 0.98), 2.4, 0.2),
            ("bomb", (-2.0, 0.45, 1.02), 2.4, 0.2)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The intakes' shock cones.
    A.mirrored(lambda side, k: cyl(f"shock_cone_{k}", 0.2, 0.55, (1.9, side * 0.82, 1.84), "X", m["paint"], hull,
                                   seg=14, r2=0.02))
    cyl("probe", 0.05, 1.1, (5.2, -0.36, 2.6), "X", m["steel"], hull, seg=8, lods=MID)
    cyl("probe_base", 0.08, 0.35, (4.6, -0.36, 2.55), "X", m["paint"], hull, seg=10, lods=MID)
    box("antiglare", (1.0, 0.5, 0.03), (5.0, 0, 2.48), m["dark"], hull, rot=(0, 0.14, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-2.0, 0, 2.84), 0.22, m, hull)
    # The Armée de l'air's roundels (aft on the body and on the wings) and
    # the 3-IM code of the photo on the nose.
    A.markings(v, [
        ("insignia", dict(kind="fr", centre=(-5.9, 1.0, 1.92), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="fr", centre=(-2.6, 3.0, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.42,
                          onto=("wing_",))),
        ("text", dict(text="3-IM", height=0.26, centre=(3.1, 1.0, 1.8), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.2, wing_y=2.8, wing_side=-1, tail_yaw=-0.3, seed=20)


run_disabled("mirage2000", CARDS, "french_air_camo", build, wreck)
