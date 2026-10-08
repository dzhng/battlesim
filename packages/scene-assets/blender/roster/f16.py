"""F-16C Block 50, from assets/references/f_16_fighting_falcon/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f16.py -- [--wreck]

What the photos settle: the chin intake under the cockpit, the frameless
bubble canopy high on the nose, the cropped delta wing blended into the body
by long strakes, the single fin over a brake-chute fairing, two canted
ventral fins, all-moving tailplanes with anhedral, the F110's nozzle with its
petals; wing-tip AIM-120 rails, tanks on the inner wing pylons, missiles
outboard, the targeting and HARM pods on the intake's cheeks (Block 50),
the tall nose gear behind the intake and the main gear folding into the body.

Dimensions stated from the references (USAF fact sheet figures agree):
length 15.06 m, span 9.96 m over the tip rails, height 4.88 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

CARDS = {"f_16c_block_50": (15.06, 9.96, 4.88)}

WING_Z = 1.78
SPEC = dict(
    # The long radome and the nose ahead of the windscreen (2.7 m back from
    # the tip in the side photo), the cockpit's sill low under the bubble.
    fuselage=[(7.16, 0.0, 1.98, 1.98), (6.75, 0.3, 1.72, 2.2, 1.96), (6.0, 0.5, 1.52, 2.36, 1.94),
              (4.8, 0.6, 1.38, 2.42, 1.9, 2.4), (3.3, 0.68, 1.26, 2.44, 1.85, 2.6), (1.8, 0.84, 1.2, 2.56, 1.82, 2.8),
              (-0.5, 0.9, 1.2, 2.62, 1.8, 2.8), (-3.2, 0.86, 1.25, 2.44, 1.8, 2.6), (-5.4, 0.66, 1.4, 2.26, 1.82, 2.4),
              (-6.82, 0.56, 1.5, 2.12, 1.82)],
    bodies=[
        # The chin intake's duct under the forward fuselage, back into the body.
        ("fuselage_intake", [(4.62, 0.52, 0.66, 1.3, 0.98, 2.6), (3.4, 0.56, 0.7, 1.45, 1.05, 2.6),
                             (1.6, 0.6, 0.86, 1.55, 1.2, 2.6), (0.0, 0.5, 1.1, 1.6, 1.3, 2.4)]),
        # The dorsal spine the canopy fairs into, back to the fin.
        ("fuselage_spine", [(1.9, 0.0, 2.5, 2.5), (1.35, 0.36, 2.4, 2.9, 2.5, 2.4), (-0.5, 0.42, 2.4, 2.84, 2.5, 2.4),
                            (-3.0, 0.36, 2.3, 2.62, 2.4, 2.4), (-4.4, 0.2, 2.3, 2.46, 2.36)]),
        # The brake-chute fairing at the fin's root.
        ("tail_chute", [(-4.6, 0.0, 2.36, 2.36), (-5.4, 0.26, 2.2, 2.62, 2.4), (-7.0, 0.24, 2.2, 2.64, 2.42),
                        (-7.42, 0.12, 2.3, 2.5)]),
    ],
    canopy=dict(x_front=4.5, x_back=1.35, sill=2.3, top=3.06, half_width=0.46, bows=(2.15,), peak=0.45, tail=0.8),
    strake=[(4.4, 0.5, 1.82, 3.9, 0.12), (1.2, 0.9, WING_Z, 1.0, 0.1)],
    wing=[(0.9, 0.86, WING_Z, 4.7, 0.2), (-2.6, 4.72, WING_Z, 1.12, 0.05)],
    stab=[(-4.85, 0.72, 1.72, 2.25, 0.12), (-6.3, 2.79, 1.4, 0.95, 0.05)],
    fins=[dict(root_x=-3.35, root_z=2.42, height=2.46, root_chord=3.5, tip_chord=1.15, sweep_m=2.62),
          dict(root_x=-4.1, root_z=1.42, height=0.62, root_chord=1.25, tip_chord=0.7, sweep_m=0.45, y=0.5,
               cant=2.88, rudder=False),
          dict(root_x=-4.1, root_z=1.42, height=0.62, root_chord=1.25, tip_chord=0.7, sweep_m=0.45, y=-0.5,
               cant=2.88, rudder=False)],
    intakes=[("rect", (4.6, 0.0, 0.98), 0.98, 0.62, 0.22, (0, -0.12, 0))],
    nozzles=[dict(loc=(-6.8, 0.0, 1.82), r_front=0.56, r_exit=0.5, length=0.74, petals=15)],
    wing_hinge=(0.15, 0.78),
    gear=[dict(name="nose", x=3.75, y=0.0, top=0.92, radius=0.3, width=0.17, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-0.55, y=1.18, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-0.55, y=-1.18, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-0.4, 2.1, WING_Z - 0.08), 1.0, 0.3), ((-1.3, 3.35, WING_Z - 0.06), 0.9, 0.22)],
    stores=[("tank", (-0.5, 2.1, 0.98), 4.6, 0.41), ("missile", (-1.2, 3.35, 1.38), 3.0, 0.09),
            ("missile", (-1.35, 4.86, WING_Z), 3.65, 0.09)],
    nose_probe=((7.15, 0.0, 2.0), 0.38),
    nav=dict(left=(2.6, 0.64, 1.2), right=(2.6, -0.64, 1.2), tail=(-7.35, 0.0, 2.62)),
)
# Shaw's 20th Fighter Wing, as the side photo: low-visibility star-and-bar on
# the fuselage and the wings (upper left, lower right), "SW" and the serial on the fin.
MARKINGS = [
    ("insignia", dict(kind="us_lowvis", centre=(-4.4, 1.0, 2.0), normal=(0, 1, 0), up=(0, 0, 1), size=0.36,
                      onto=("fuselage",))),
    ("insignia", dict(kind="us_lowvis", centre=(-1.5, 3.1, WING_Z + 0.1), normal=(0, 0, 1), up=(1, 0, 0), size=0.5,
                      onto=("wing_L",), mirror=False)),
    ("insignia", dict(kind="us_lowvis", centre=(-1.5, -3.1, WING_Z - 0.1), normal=(0, 0, -1), up=(1, 0, 0),
                      size=0.5, onto=("wing_R",), mirror=False)),
    ("text", dict(text="SW", height=0.62, centre=(-5.75, 0.1, 3.7), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("tail_fin_0",))),
    ("text", dict(text="AF 91 359", height=0.18, centre=(-5.35, 0.1, 3.15), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("tail_fin_0",))),
    ("text", dict(text="USAF", height=0.3, centre=(-1.4, -3.3, WING_Z + 0.1), normal=(0, 0, 1), up=(-1, 0, 0),
                  onto=("wing_R",), mirror=False, colour="lowvis_dark")),
]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # Wing-tip rails under their AIM-120s.
    A.mirrored(lambda side, s: box(f"wing_tip_rail_{s}", (2.4, 0.08, 0.1), (-1.4, side * 4.8, WING_Z - 0.02),
                                   m["dark"], hull, bevel=0.01))
    # Block 50's HARM targeting pod and the Sniper pod on the intake's cheeks.
    for side, s, length in ((1, "L", 2.5), (-1, "R", 2.2)):
        box(f"cheek_pylon_{s}", (0.8, 0.12, 0.16), (2.2, side * 0.62, 0.92), m["paint"], hull, bevel=0.02, lods=MID)
        A.store(f"cheek_pod_{s}", "pod", (2.2, side * 0.72, 0.72), length, 0.17, m, hull)
    # Anti-glare panel ahead of the canopy, the refuelling receptacle behind it.
    box("antiglare", (0.9, 0.5, 0.03), (5.0, 0, 2.4), m["dark"], hull, rot=(0, 0.06, 0), lods=MID)
    box("refuel_door", (0.5, 0.28, 0.03), (-0.6, 0, 2.85), m["dark"], hull, lods=NEAR)
    # Speed brakes either side of the nozzle, closed.
    A.mirrored(lambda side, s: box(f"speed_brake_{s}", (0.9, 0.06, 0.55), (-6.45, side * 0.62, 1.86), m["paint"],
                                   hull, bevel=0.02, lods=MID))
    A.blade_antenna("antenna_spine", (-1.6, 0, 2.78), 0.22, m, hull)
    A.blade_antenna("antenna_belly", (0.2, 0, 1.2), 0.22, m, hull, down=True)
    A.markings(v, MARKINGS)


def wreck(variant, v):
    A.crash(v, tail_x=-4.2, wing_y=2.8, wing_side=-1, seed=16)


run_disabled("f16", CARDS, "us_compass_grey", build, wreck)
