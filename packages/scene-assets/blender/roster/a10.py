"""A-10C Thunderbolt II, from assets/references/a_10_warthog/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/a10.py -- [--wreck]

What the photos settle: the blunt nose with the GAU-8's seven-barrel muzzle
under it and the nose gear offset to the right, the high bubble canopy on an
armoured tub, the thick straight low wing with the main gear half-retracted
into pods under it and the tips drooped, two engines in round nacelles on
pylons high on the rear fuselage, the tailplane with twin fins at its ends;
eleven pylons. Two-tone grey.

Dimensions stated from the references (USAF fact sheet figures agree):
length 16.26 m, span 17.53 m, height 4.47 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import FINE, MID, NEAR  # noqa: E402

CARDS = {"us_a_10_warthog_a_10c_retained_as_an_iconic_exception_if_needed": (16.26, 17.53, 4.47)}

# The side photo stands the A-10 tall on its gear: its low wing 1.6 m up,
# its forward body 1.4-3.2 m, the canopy's top 3.6 m.
WING_Z = 1.6
FIN = dict(root_x=-6.1, root_z=1.92, height=2.55, root_chord=2.0, tip_chord=1.45, sweep_m=0.45)
SPEC = dict(
    fuselage=[(7.72, 0.0, 2.15, 2.15), (7.35, 0.46, 1.7, 2.6, 2.15), (6.3, 0.72, 1.55, 2.95, 2.2, 2.4),
              (4.6, 0.8, 1.45, 3.15, 2.25, 2.6), (2.5, 0.82, 1.4, 3.1, 2.2, 2.6), (0.0, 0.76, 1.42, 2.85, 2.1, 2.4),
              (-3.0, 0.56, 1.5, 2.45, 1.95, 2.2), (-6.0, 0.32, 1.6, 2.0, 1.78), (-7.6, 0.16, 1.72, 1.95, 1.84)],
    bodies=[*[(f"nacelle_gear_{s}", [(1.9, 0.0, 1.4, 1.4), (1.4, 0.34, 1.1, 1.62, 1.4), (-0.8, 0.34, 1.1, 1.6, 1.38),
                                     (-1.6, 0.0, 1.5, 1.5)], 2.1 if s == "L" else -2.1, 14) for s in "LR"],
            # The spine behind the canopy, falling to the engines.
            ("fuselage_spine", [(3.4, 0.42, 3.0, 3.4, 3.1, 2.4), (1.0, 0.42, 2.9, 3.1, 3.0, 2.4),
                                (-1.6, 0.0, 2.8, 2.8)])],
    canopy=dict(x_front=6.1, x_back=3.3, sill=3.02, top=3.62, half_width=0.56, bows=(5.6,), peak=0.5, tail=0.6),
    wing=[(1.48, 0.72, WING_Z, 3.1, 0.46), (1.42, 2.7, WING_Z, 3.0, 0.4), (0.92, 8.38, WING_Z + 0.4, 1.7, 0.16),
          (0.86, 8.76, WING_Z + 0.3, 1.6, 0.1)],
    wing_hinge=((0.78, 0, 3),),
    nav=dict(left=(0.4, 8.7, WING_Z + 0.32), right=(0.4, -8.7, WING_Z + 0.32)),
    stab=[(-6.25, 0.2, 1.92, 1.95, 0.16), (-6.4, 2.86, 1.92, 1.65, 0.12)],
    fins=[dict(FIN, y=2.86), dict(FIN, y=-2.86)],
    gear=[dict(name="nose", x=5.0, y=-0.36, top=1.55, radius=0.33, width=0.2, door=(0.8, 0.4), light=True),
          dict(name="main_L", x=-0.3, y=2.1, top=1.2, radius=0.5, width=0.3),
          dict(name="main_R", x=-0.3, y=-2.1, top=1.2, radius=0.5, width=0.3)],
    pylons=[((0.9, 3.4, WING_Z - 0.18), 1.2, 0.26), ((0.9, 4.6, WING_Z - 0.08), 1.1, 0.24),
            ((0.6, 5.8, WING_Z + 0.04), 1.0, 0.22)],
    stores=[("missile", (0.8, 3.4, 0.98), 2.5, 0.15), ("bomb", (0.8, 4.6, 1.1), 2.3, 0.2),
            ("pod", (0.5, 5.8, 1.33), 2.4, 0.15)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The GAU-8 under the nose: its muzzle fairing and seven barrels.
    cyl("gun_fairing", 0.24, 0.5, (7.6, 0.06, 1.85), "X", m["paint"], hull, seg=16, bevel=0.02)
    for k in range(7):
        a = math.tau * k / 7
        cyl(f"gun_barrel_{k}", 0.035, 0.5, (8.0, 0.06 + math.cos(a) * 0.12, 1.85 + math.sin(a) * 0.12), "X",
            m["steel"], hull, seg=8, lods=MID if k % 2 == 0 else NEAR)
    cyl("gun_muzzle", 0.17, 0.04, (8.24, 0.06, 1.85), "X", m["black"], hull, seg=14, lods=MID)
    # The engines: round nacelles on pylons over the rear fuselage.
    for side, s_ in ((1, "L"), (-1, "R")):
        y = side * 1.38
        A.body(f"nacelle_engine_{s_}", [(-1.6, 0.0, 3.05, 3.05), (-1.8, 0.62, 2.43, 3.67, 3.05),
                                        (-4.4, 0.62, 2.43, 3.67, 3.05), (-5.15, 0.44, 2.61, 3.49, 3.05)],
               m["paint"], hull, seg=20, loc=(0, y, 0))
        cyl(f"nacelle_engine_{s_}_fan", 0.52, 0.04, (-1.8, y, 3.05), "X", m["dark"], hull, seg=20, lods=MID)
        cyl(f"nacelle_engine_{s_}_spinner", 0.14, 0.2, (-1.72, y, 3.05), "X", m["steel"], hull, seg=10,
            r2=0.03, lods=NEAR)
        cyl(f"nacelle_engine_{s_}_exhaust", 0.36, 0.05, (-5.17, y, 3.05), "X", m["black"], hull, seg=16, lods=MID)
        box(f"nacelle_pylon_{s_}", (2.2, 0.16, 0.8), (-3.3, side * 0.95, 2.5), m["paint"], hull, bevel=0.03,
            rot=(side * 0.6, 0, 0))
    box("antiglare", (1.1, 0.7, 0.03), (6.5, 0, 2.86), m["dark"], hull, rot=(0, 0.25, 0), lods=MID)
    box("refuel_door", (0.5, 0.3, 0.03), (7.0, 0, 2.6), m["dark"], hull, rot=(0, 0.35, 0), lods=NEAR)
    for side, s_ in ((1, "L"), (-1, "R")):
        box(f"wing_flap_seam_{s_}", (0.03, 5.4, 0.03), (-1.0, side * 5.4, 1.84), m["dark"], hull,
            rot=(0, 0, side * 0.05), lods=FINE)
    A.blade_antenna("antenna_spine", (0.8, 0, 3.08), 0.2, m, hull)
    # Moody's FT on the fins in black, the low-visibility insignia behind the
    # cockpit and on the wings, the serial on the nose gear door's fuselage.
    A.markings(v, [
        ("insignia", dict(kind="us_lowvis", centre=(2.5, 1.0, 2.55), normal=(0, 1, 0), up=(0, 0, 1), size=0.42,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(0.2, 6.2, WING_Z + 0.4), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.6, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(0.2, -6.2, WING_Z), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.6, onto=("wing_R",), mirror=False)),
        ("text", dict(text="FT", height=0.5, centre=(-6.9, 3.1, 3.75), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_",), colour="black")),
        ("text", dict(text="AF 80 223", height=0.16, centre=(-6.9, 3.1, 3.2), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_",), colour="black")),
        # The 23rd Wing's Flying Tiger shark mouth under the nose (the photo).
        ("insignia", dict(kind="shark_mouth", centre=(6.9, 1.0, 1.95), normal=(0, 1, 0), up=(0, 0, 1), size=0.85,
                          onto=("fuselage", "gun_fairing"), ahead=(1, 0, 0))),
        ("text", dict(text="0223", height=0.18, centre=(-0.3, 0.9, 1.75), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-4.9, wing_y=5.0, wing_side=-1, tail_yaw=0.32, seed=10)


run_disabled("a10", CARDS, "us_gunship_grey", build, wreck)
