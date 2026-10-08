"""Su-25SM3, from assets/references/su_25_frogfoot/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/su25.py -- [--wreck]

What the photos settle: a slim pointed nose and the armoured cockpit with
its small canopy, two engines in nacelles hugging the fuselage at the
roots of a high straight wing, round-cornered intakes ahead of them, a
tall fin, tailplane with dihedral, short nozzles ending at the trailing
edge; the nose wheel offset to the left, rough-field tyres; eight pylons
with rocket pods and bombs, the SM3's L-370 jamming pods on the wing tips.
Green and brown camouflage.

Dimensions stated from the references (Sukhoi figures): length 15.53 m,
span 14.36 m, height 4.8 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_su_25_frogfoot_su_25sm3": (15.53, 14.36, 4.8)}
WING_Z = 2.25
SPEC = dict(
    fuselage=[(7.77, 0.0, 1.95, 1.95), (7.3, 0.32, 1.68, 2.2, 1.95), (6.2, 0.52, 1.5, 2.42, 1.95, 2.2),
              (5.0, 0.6, 1.45, 2.62, 1.98, 2.4), (3.5, 0.65, 1.45, 2.64, 2.0, 2.6), (1.0, 0.65, 1.5, 2.52, 2.0, 2.6),
              (-2.0, 0.55, 1.6, 2.42, 2.0, 2.4), (-5.0, 0.4, 1.75, 2.32, 2.02, 2.2), (-7.4, 0.2, 1.9, 2.22, 2.05)],
    canopy=dict(x_front=5.6, x_back=4.0, sill=2.6, top=3.06, half_width=0.4, bows=(4.4,), peak=0.5),
    wing=[(1.6, 1.4, WING_Z, 3.6, 0.42), (-0.6, 7.0, WING_Z - 0.2, 1.4, 0.12)],
    stab=[(-5.6, 0.3, 2.36, 1.9, 0.1), (-6.6, 2.6, 2.6, 1.0, 0.05)],
    fins=[dict(root_x=-4.2, root_z=2.3, height=2.5, root_chord=2.6, tip_chord=0.9, sweep_m=1.7)],
    intakes=[("rect", (3.08, 1.0, 1.85), 0.66, 0.78, 0.22), ("rect", (3.08, -1.0, 1.85), 0.66, 0.78, 0.22)],
    nozzles=[dict(loc=(-3.4, 1.0, 1.8), r_front=0.42, r_exit=0.38, length=0.8, petals=0),
             dict(loc=(-3.4, -1.0, 1.8), r_front=0.42, r_exit=0.38, length=0.8, petals=0)],
    gear=[dict(name="nose", x=5.0, y=0.25, top=1.4, radius=0.33, width=0.2, door=(0.8, 0.4)),
          dict(name="main_L", x=0.2, y=1.25, top=1.3, radius=0.42, width=0.26),
          dict(name="main_R", x=0.2, y=-1.25, top=1.3, radius=0.42, width=0.26)],
    pylons=[((1.0, 2.4, WING_Z - 0.2), 1.0, 0.3), ((0.8, 3.3, WING_Z - 0.18), 1.0, 0.28),
            ((0.6, 4.2, WING_Z - 0.16), 0.9, 0.26), ((0.4, 5.1, WING_Z - 0.14), 0.9, 0.24)],
    stores=[("bomb", (0.9, 2.4, 1.5), 2.4, 0.2), ("pod", (0.7, 3.3, 1.42), 2.7, 0.26),
            ("pod", (0.5, 4.2, 1.48), 2.7, 0.26), ("missile", (0.2, 5.1, 1.7), 2.9, 0.1),
            ("pod", (-0.4, 7.12, WING_Z - 0.22), 2.0, 0.2)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(3.0, 0.48, 1.3, 2.3, 1.85, 3.0), (0.0, 0.52, 1.28, 2.32, 1.82, 3.0),
                                (-3.4, 0.44, 1.38, 2.22, 1.8)], m["paint"], hull, seg=18, loc=(0, side * 1.0, 0))
    # The gun's fairing under the nose, left; the armoured windscreen frame.
    box("gun_fairing", (1.6, 0.28, 0.26), (4.4, -0.42, 1.48), m["paint"], hull, bevel=0.04, lods=MID)
    box("gun_muzzle", (0.06, 0.12, 0.12), (5.22, -0.42, 1.48), m["black"], hull, lods=MID)
    box("windscreen_frame", (0.08, 0.7, 0.4), (5.55, 0, 2.82), m["dark"], hull, rot=(0, -0.5, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"wing_tip_brake_{k}", (1.4, 0.06, 0.18), (-0.9, side * 6.95, WING_Z - 0.2),
                                   m["paint"], hull, lods=MID))
    A.pitot("air_data_probe", (7.6, 0.3, 2.1), 0.8, m, hull)


def wreck(variant, v):
    A.crash(v, tail_x=-3.6, wing_y=4.6, wing_side=-1, tail_yaw=-0.32, seed=25)


run_disabled("su25", CARDS, "russian_helicopter_camo", build, wreck)
