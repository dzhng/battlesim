"""J-10C's guide: the lofted model, measured from its three-view, whose
renders the skin (`roster/j10.py`) was generated from. Never shipped:

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/guides/j10.py -- --variant=<id> --out=<path> --no-lint

What the photos settle: a single-engine canard delta. The pointed nose and
bubble canopy, the chin intake under the cockpit (the C's diverterless bump,
where the A had a splitter plate), canards just behind the cockpit, the delta
wing, one tall fin, two ventral fins canted outward under the tail, one
nozzle. Light grey.

The references' dimensions (published figures): length 16.9 m,
span 9.75 m, height 5.43 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

# Measured from Kaboldy's three-view (`eastern_j_10_vigorous_dragon_j_10c-
# top.png` in its references, CC BY-SA 3.0, a measurement reference only) at one scale,
# its length nose to tail (no pitot) the model's 16.5 m and its fin's top
# the frame's 5.43 m: the body's outline and the big chin intake's duct from
# the side, its floor straight back at 0.9 m; the canards, the delta's 48
# degree leading edge and straight trailing edge, from the plan.
WING_Z = 1.62
VENTRAL = dict(root_x=-4.8, root_z=1.05, height=0.55, root_chord=1.6, tip_chord=0.9, sweep_m=0.5, cant=2.7,
               rudder=False)
SPEC = dict(
    fuselage=[(8.45, 0.0, 1.8, 1.8), (7.8, 0.24, 1.62, 2.07, 1.85), (7.3, 0.38, 1.54, 2.25, 1.9, 2.0),
              (6.8, 0.48, 1.45, 2.37, 1.9, 2.2), (6.3, 0.54, 1.5, 2.49, 1.95, 2.2), (5.3, 0.61, 1.54, 2.55, 2.0, 2.4),
              (4.3, 0.69, 1.55, 2.6, 2.0, 2.6), (2.8, 0.72, 1.55, 2.62, 2.0, 2.6), (0.8, 0.85, 1.55, 2.6, 2.0, 2.8),
              (-1.2, 0.9, 1.5, 2.52, 2.0, 2.8), (-3.2, 1.0, 1.45, 2.5, 2.0, 2.8), (-5.2, 1.05, 1.4, 2.55, 2.0, 2.6),
              (-6.5, 0.75, 1.45, 2.6, 2.05)],
    bodies=[# The chin intake's duct under the body, its mouth 1.5 m wide.
            ("fuselage_intake", [(4.5, 0.75, 0.9, 1.6, 1.25, 3.0), (3.0, 0.78, 0.89, 1.7, 1.3, 3.0),
                                 (0.5, 0.78, 0.89, 1.75, 1.3, 2.8), (-2.2, 0.72, 0.89, 1.8, 1.35, 2.6),
                                 (-4.2, 0.6, 1.0, 1.85, 1.4, 2.4), (-5.8, 0.0, 1.4, 1.4)], 0.0, 16),
            ("fuselage_dsi", [(4.3, 0.0, 1.5, 1.5), (3.8, 0.3, 1.4, 1.62, 1.5), (3.0, 0.0, 1.55, 1.55)], 0.0, 16),
            # The spine the canopy fairs into, back to the fin.
            ("fuselage_spine", [(3.4, 0.4, 2.55, 2.94, 2.7, 2.4), (0.8, 0.45, 2.5, 2.83, 2.65, 2.6),
                                (-2.6, 0.0, 2.5, 2.5)])],
    canopy=dict(x_front=6.1, x_back=3.4, sill=2.55, top=3.15, half_width=0.46, bows=(5.5,), peak=0.57, tail=0.75),
    canard_hinge=(0.7,),
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-4.15, 4.8, WING_Z + 0.03), right=(-4.15, -4.8, WING_Z + 0.03), tail=(-7.9, 0.0, 5.3)),
    canard=[(3.6, 0.8, 2.1, 1.7, 0.08), (1.95, 2.2, 2.12, 0.45, 0.03)],
    # The delta from its root just behind the canards to its small cropped tip.
    wing=[(1.3, 0.85, WING_Z, 5.65, 0.3), (-3.95, 4.87, WING_Z, 0.45, 0.04)],
    fins=[dict(root_x=-2.6, root_z=2.5, height=2.92, root_chord=4.8, tip_chord=1.0, sweep_m=4.3),
          dict(VENTRAL, y=0.6), dict(VENTRAL, y=-0.6)],
    intakes=[("rect", (4.5, 0.0, 1.22), 1.5, 0.62, 0.25, (0, -0.1, 0))],
    nozzles=[dict(loc=(-6.6, 0.0, 2.05), r_front=0.62, r_exit=0.55, length=1.3, petals=14)],
    gear=[dict(name="nose", x=4.2, y=0.0, top=0.95, radius=0.26, width=0.15, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-1.2, y=1.3, top=1.5, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-1.2, y=-1.3, top=1.5, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-1.0, 2.2, WING_Z - 0.06), 1.1, 0.3), ((-1.8, 3.3, WING_Z - 0.04), 0.9, 0.24)],
    stores=[("tank", (-1.1, 2.2, WING_Z - 0.78), 4.0, 0.36), ("missile", (-1.8, 3.3, WING_Z - 0.4), 3.7, 0.1),
            ("missile", (-3.0, 4.3, WING_Z - 0.36), 2.9, 0.08)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    A.blade_antenna("antenna_spine", (0.2, 0, 2.87), 0.22, m, hull)
    # The PLA's red star and bars aft on the body and on the wings, the
    # photo's red 05 on the nose and the fin.
    A.markings(v, [
        ("insignia", dict(kind="cn_star", centre=(-3.9, 1.2, 2.15), normal=(0, 1, 0), up=(0, 0, 1), size=0.6,
                          onto=("fuselage",))),
        ("insignia", dict(kind="cn_star", centre=(-1.4, 3.0, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_",))),
        ("text", dict(text="05", height=0.42, centre=(6.6, 1.0, 2.2), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="red")),
        ("text", dict(text="05", height=0.4, centre=(-6.6, 0.3, 4.6), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="red")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=10)


run("j_10_vigorous_dragon", "chinese_air_light_grey", build, wreck, script="j10.py")
