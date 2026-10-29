"""JAS 39 Gripen E, from assets/references/jas_39_gripen/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/gripen.py -- [--wreck]

What the photos settle: a small single-engine delta, the close-coupled
canards mounted on the side intakes just behind the cockpit, one swept fin,
the single nozzle; wing-tip missiles. The E (from the NG demonstrator and
published figures) keeps the C's layout on a longer body with its main gear
moved out into the wing roots. Light grey.

The references' dimensions (Saab figures): length 15.2 m, span
8.6 m over the tip missiles, height 4.5 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

WING_Z = 1.6
SPEC = dict(
    # The side photo: the long canopy 2.4 m back from the nose, the spine
    # 2.5 m up behind it, the belly a metre off the ground, the intakes, the
    # canards and both gear well aft of the old stations. The rear body
    # stays deep to the tail (1.2 m from belly to spine under the fin) and
    # ends in the nozzle's own diameter, the fin set right over it.
    fuselage=[(7.6, 0.0, 1.7, 1.7), (6.9, 0.34, 1.46, 1.96, 1.72), (5.8, 0.5, 1.3, 2.2, 1.72, 2.2),
              (4.3, 0.56, 1.2, 2.34, 1.7, 2.4), (2.6, 0.62, 1.08, 2.46, 1.7, 2.6), (0.0, 0.7, 1.04, 2.52, 1.74, 2.8),
              (-3.0, 0.68, 1.16, 2.48, 1.78, 2.8), (-5.0, 0.62, 1.28, 2.42, 1.8, 2.6), (-6.4, 0.55, 1.4, 2.3, 1.82, 2.4),
              (-7.0, 0.5, 1.44, 2.2, 1.82)],
    bodies=[*[(f"fuselage_intake_{s}", [(2.65, 0.3, 1.2, 2.0, 1.6, 3.0), (0.5, 0.32, 1.18, 2.02, 1.6, 3.0),
                                         (-1.4, 0.0, 1.6, 1.6)], 0.8 if s == "L" else -0.8, 14) for s in "LR"],
            ("fuselage_spine", [(2.1, 0.36, 2.3, 2.76, 2.4, 2.4), (-0.4, 0.4, 2.3, 2.62, 2.45, 2.6),
                                (-3.6, 0.0, 2.42, 2.42)])],
    canopy=dict(x_front=5.25, x_back=2.0, sill=2.3, top=2.88, half_width=0.44, bows=(4.7,), peak=0.42, tail=0.7),
    canard=[(2.5, 0.95, 2.12, 1.5, 0.08), (1.35, 2.6, 2.12, 0.7, 0.03)],
    canard_hinge=(0.7,),
    wing_hinge=((0.8, 0, 1),),
    nav=dict(left=(-3.3, 4.0, WING_Z + 0.06), right=(-3.3, -4.0, WING_Z + 0.06), tail=(-6.42, 0.0, 4.3)),
    # The delta from the photo: its root from behind the intakes to just
    # ahead of the jet pipe, 45 degrees back to the tip rails.
    wing=[(-0.1, 0.7, WING_Z, 5.2, 0.24), (-3.6, 4.06, WING_Z, 1.1, 0.04)],
    # The fin over the nozzle: its leading edge leaves the spine 11 m behind
    # the nose, its tip a metre deep, its trailing edge over the jet pipe.
    fins=[dict(root_x=-3.4, root_z=2.38, height=2.12, root_chord=3.1, tip_chord=1.05, sweep_m=1.95)],
    intakes=[("rect", (2.68, 0.8, 1.6), 0.42, 0.66, 0.2), ("rect", (2.68, -0.8, 1.6), 0.42, 0.66, 0.2)],
    nozzles=[dict(loc=(-7.0, 0.0, 1.82), r_front=0.44, r_exit=0.4, length=0.6, petals=12)],
    gear=[dict(name="nose", x=3.7, y=0.0, top=1.25, radius=0.24, width=0.14, wheels=2, spread=0.2, light=True),
          dict(name="main_L", x=-2.0, y=1.25, top=1.15, radius=0.34, width=0.18, door=(0.9, 0.5)),
          dict(name="main_R", x=-2.0, y=-1.25, top=1.15, radius=0.34, width=0.18, door=(0.9, 0.5))],
    pylons=[((-1.6, 1.8, WING_Z - 0.06), 1.0, 0.28), ((-2.4, 2.8, WING_Z - 0.04), 0.9, 0.22)],
    stores=[("tank", (-1.7, 1.8, 0.98), 3.6, 0.32), ("missile", (-2.4, 2.8, 1.24), 3.65, 0.09),
            ("missile", (-3.9, 4.16, WING_Z), 2.94, 0.065)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    A.mirrored(lambda side, k: box(f"wing_tip_rail_{k}", (2.0, 0.07, 0.08), (-4.0, side * 4.1, WING_Z - 0.02),
                                   m["dark"], hull))
    box("antiglare", (0.8, 0.5, 0.03), (5.75, 0, 2.24), m["dark"], hull, rot=(0, 0.16, 0), lods=MID)
    box("probe_fairing", (1.0, 0.1, 0.1), (4.3, -0.49, 2.28), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (-1.0, 0, 2.58), 0.2, m, hull)
    # The Flygvapnet's roundel on the intakes and wings, the 270 of the photo on the fin.
    A.markings(v, [
        ("insignia", dict(kind="se", centre=(3.4, 1.2, 1.7), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="se", centre=(-3.0, 2.2, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.4,
                          onto=("wing_",))),
        ("text", dict(text="270", height=0.42, centre=(-5.0, 0.3, 3.55), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="lowvis_dark")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-2.8, wing_y=2.4, wing_side=-1, tail_yaw=-0.3, seed=39)


run("jas_39_gripen", "nato_air_grey", build, wreck, script="gripen.py")
