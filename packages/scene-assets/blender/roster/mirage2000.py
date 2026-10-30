"""Mirage 2000D (RMV), from assets/references/mirage_2000/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mirage2000.py -- [--wreck]

What the photos settle: the two-seat canopy of the strike Mirage, its fixed
probe on the right of the windscreen, half-round intakes with their shock
cones either side of the cockpit, the pure delta wing low on the body, one
tall fin, the single M53 nozzle; big tanks under the wings, bombs under the
body. French strike camouflage, grey with dark grey-green patches.

The references' dimensions (Dassault figures): length 14.36 m,
span 9.13 m, height 5.2 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

WING_Z = 1.55
SPEC = dict(
    # The side photo: the two-seat canopy 3 m behind the nose and 4 m long,
    # its top 3.4 m up, the spine 3.25 m behind it and 2.9 m at the fin;
    # the belly 1.25 m, the nozzle's centre 2.05 m. The intakes' cones,
    # the fin and both gear further aft than drawn before.
    fuselage=[(7.18, 0.0, 1.88, 1.88), (6.5, 0.34, 1.62, 2.18, 1.9), (5.4, 0.5, 1.45, 2.48, 1.92, 2.2),
              (4.0, 0.58, 1.36, 2.72, 1.96, 2.4), (2.5, 0.68, 1.28, 2.9, 2.0, 2.6), (0.0, 0.78, 1.25, 3.0, 2.05, 2.6),
              (-3.0, 0.76, 1.3, 2.9, 2.05, 2.6), (-5.4, 0.62, 1.55, 2.62, 2.06, 2.4), (-6.4, 0.55, 1.58, 2.55, 2.06)],
    # The half-round intakes run back along the body sides and fair into
    # it over the wing root.
    bodies=[*[(f"fuselage_intake_{s}", [(1.65, 0.46, 1.56, 2.46, 2.0), (-0.5, 0.48, 1.54, 2.48, 2.0),
                                         (-2.4, 0.34, 1.62, 2.4, 2.02), (-4.0, 0.0, 2.05, 2.05)],
               0.84 if s == "L" else -0.84, 16) for s in "LR"],
            ("fuselage_spine", [(0.4, 0.36, 2.85, 3.25, 2.95, 2.4), (-2.5, 0.38, 2.8, 3.12, 2.9, 2.6),
                                (-5.4, 0.0, 2.7, 2.7)])],
    canopy=dict(x_front=4.3, x_back=0.25, sill=2.82, top=3.42, half_width=0.44, bows=(3.6, 2.0), peak=0.4, tail=0.7),
    wing_hinge=((0.85, 0, 1), 0.06),
    # The small fixed strake on each intake, over the wing's root.
    strake=[(1.0, 1.2, 2.3, 0.9, 0.05), (0.55, 1.6, 2.3, 0.4, 0.02)],
    nav=dict(left=(-5.0, 4.45, WING_Z + 0.05), right=(-5.0, -4.45, WING_Z + 0.05), tail=(-6.55, 0.0, 4.9)),
    # The pure delta, low on the body: its root from behind the intake lips
    # to the nozzle, its trailing edge straight across.
    wing=[(2.0, 0.72, WING_Z, 8.0, 0.4), (-4.4, 4.56, WING_Z, 1.5, 0.05)],
    # The fin leaves the spine 10.6 m behind the nose, 2.2 m tall to 5.2 m.
    fins=[dict(root_x=-3.45, root_z=3.0, height=2.2, root_chord=3.3, tip_chord=1.0, sweep_m=2.3)],
    intakes=[("round", (1.68, 0.84, 2.0), 0.42, 0.16), ("round", (1.68, -0.84, 2.0), 0.42, 0.16)],
    nozzles=[dict(loc=(-6.4, 0.0, 2.06), r_front=0.52, r_exit=0.5, length=0.8, petals=14)],
    gear=[dict(name="nose", x=2.9, y=0.0, top=1.3, radius=0.25, width=0.14, wheels=2, spread=0.2, light=True),
          dict(name="main_L", x=-2.2, y=1.6, top=1.3, radius=0.38, width=0.2, door=(1.0, 0.5)),
          dict(name="main_R", x=-2.2, y=-1.6, top=1.3, radius=0.38, width=0.2, door=(1.0, 0.5))],
    pylons=[((-0.4, 2.0, WING_Z - 0.06), 1.2, 0.14)],
    stores=[("tank", (-0.5, 2.0, 0.98), 4.6, 0.4), ("bomb", (0.6, 0.45, 0.98), 2.4, 0.2),
            ("bomb", (-2.0, 0.45, 1.02), 2.4, 0.2)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The intakes' shock cones.
    A.mirrored(lambda side, k: cyl(f"shock_cone_{k}", 0.22, 0.55, (1.9, side * 0.84, 2.0), "X", m["paint"], hull,
                                   seg=14, r2=0.02))
    # The fixed probe right of the windscreen: a strut raked up and forward
    # off the nose, its tip level.
    cyl("probe_base", 0.08, 1.0, (4.75, -0.36, 2.72), "X", m["paint"], hull, seg=10, rot=(0, -0.58, 0), lods=MID)
    cyl("probe", 0.045, 0.6, (5.35, -0.36, 3.0), "X", m["steel"], hull, seg=8, lods=MID)
    box("antiglare", (1.0, 0.5, 0.03), (5.0, 0, 2.55), m["dark"], hull, rot=(0, 0.16, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-2.0, 0, 3.04), 0.22, m, hull)
    # The Armée de l'air's roundels (aft on the body and on the wings) and
    # the 3-IM code of the photo on the nose.
    A.markings(v, [
        ("insignia", dict(kind="fr", centre=(-5.6, 1.0, 2.06), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="fr", centre=(-2.6, 3.0, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.42,
                          onto=("wing_",))),
        ("text", dict(text="3-IM", height=0.26, centre=(3.1, 1.0, 1.95), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.2, wing_y=2.8, wing_side=-1, tail_yaw=-0.3, seed=20)


run("mirage_2000", "french_air_camo", build, wreck, script="mirage2000.py")
