"""F-16C Block 50, from assets/references/f_16_fighting_falcon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f16.py -- [--wreck]

What the photos settle: the chin intake under the cockpit, the frameless
bubble canopy high on the nose, the cropped delta wing blended into the body
by long strakes, the single fin over a brake-chute fairing, two canted
ventral fins, all-moving tailplanes with anhedral, the F110's nozzle with its
petals; wing-tip AIM-120 rails, tanks on the inner wing pylons, missiles
outboard, the targeting and HARM pods on the intake's cheeks (Block 50),
the tall nose gear behind the intake and the main gear folding into the body.

The references' dimensions (USAF fact sheet figures agree):
length 15.06 m, span 9.96 m over the tip rails, height 4.88 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402


WING_Z = 1.78
# The chin intake's mouth: its lip's centre, and the rake that puts the upper
# lip ahead of the lower, so the mouth looks forward and a little down.
INTAKE_LIP, INTAKE_RAKE = (3.45, 0.0, 1.11), 0.1
SPEC = dict(
    # One blended body: the radome's round ogive (2.7 m from the tip to the
    # windscreen in the side photo), flat-topped under the cockpit's sill so
    # the bubble sits on it, then the blended wing-body behind: a rounded top
    # falling to the chine the strakes and wings run from (`widest_z`).
    fuselage=[(7.16, 0.0, 1.95, 1.95), (6.7, 0.22, 1.76, 2.13, 1.93), (6.0, 0.42, 1.55, 2.33, 1.91, 2.1),
              (5.0, 0.6, 1.32, 2.56, 1.88, 2.6), (4.0, 0.68, 1.25, 2.64, 1.86, 2.9), (2.8, 0.76, 1.3, 2.62, 1.85, 2.8),
              (1.4, 0.9, 1.35, 2.68, 1.84, 1.9), (0.0, 1.1, 1.38, 2.78, 1.82, 1.4),
              (-2.0, 1.12, 1.4, 2.72, 1.82, 1.35), (-4.0, 0.95, 1.42, 2.56, 1.82, 1.55),
              (-5.6, 0.68, 1.45, 2.32, 1.82, 2.0), (-6.82, 0.56, 1.5, 2.14, 1.82, 2.2)],
    bodies=[
        # The chin intake's duct from behind its lip (under the canopy's
        # peak, the nose gear under it) back as the belly: the main gear's
        # bays bulge it under the wing roots, and it rises to the ventral fins.
        ("fuselage_intake", [(3.3, 0.66, 0.77, 1.47, 1.12, 2.6), (2.2, 0.7, 0.8, 1.5, 1.15, 2.8),
                             (0.8, 0.84, 0.86, 1.55, 1.2, 2.8), (-0.4, 0.94, 0.95, 1.6, 1.28, 2.8),
                             (-1.3, 1.05, 1.0, 1.62, 1.32, 2.8), (-2.2, 0.92, 1.08, 1.62, 1.35, 2.6), (-3.6, 0.72, 1.2, 1.62, 1.4, 2.4),
                             (-4.8, 0.55, 1.32, 1.6, 1.45, 2.2)]),
        # The canopy's aft fairing, a low dome melting into the body's ridge behind it.
        ("fuselage_spine", [(2.1, 0.0, 2.6, 2.6), (1.6, 0.36, 2.45, 2.83, 2.55, 2.4),
                            (0.2, 0.42, 2.5, 2.86, 2.5, 2.0), (-1.6, 0.36, 2.52, 2.8, 2.52, 2.0),
                            (-3.0, 0.0, 2.62, 2.62)]),
        # The brake-chute fairing at the fin's root.
        ("tail_chute", [(-5.7, 0.0, 2.4, 2.4), (-6.05, 0.2, 2.26, 2.56, 2.4), (-6.95, 0.19, 2.27, 2.55, 2.4),
                        (-7.25, 0.1, 2.33, 2.48)]),
    ],
    # The frameless bubble high on the nose, its sill on the body's top.
    canopy=dict(x_front=4.8, x_back=1.6, sill=2.48, top=3.2, half_width=0.5, bows=(), peak=0.4, tail=0.45),
    # The strakes' ogee edge from under the windscreen out into the wing's leading edge.
    strake=[(4.75, 0.5, 1.86, 3.9, 0.1), (2.6, 1.0, 1.82, 2.4, 0.08), (0.6, 1.5, WING_Z, 0.9, 0.06)],
    wing=[(0.9, 0.86, WING_Z, 4.7, 0.2), (-2.6, 4.72, WING_Z, 1.12, 0.05)],
    stab=[(-4.85, 0.6, 1.74, 2.3, 0.12), (-6.3, 2.79, 1.4, 0.95, 0.05)],
    fins=[dict(root_x=-3.35, root_z=2.42, height=2.46, root_chord=3.5, tip_chord=1.15, sweep_m=2.62),
          dict(root_x=-4.1, root_z=1.42, height=0.62, root_chord=1.25, tip_chord=0.7, sweep_m=0.45, y=0.5,
               cant=2.88, rudder=False),
          dict(root_x=-4.1, root_z=1.42, height=0.62, root_chord=1.25, tip_chord=0.7, sweep_m=0.45, y=-0.5,
               cant=2.88, rudder=False)],
    nozzles=[dict(loc=(-6.8, 0.0, 1.82), r_front=0.56, r_exit=0.5, length=0.74, petals=15)],
    wing_hinge=(0.15, 0.78),
    # Wheelbase 4.0 m and track 2.36 m (the fact sheets; the mains a little
    # inboard, on the gear bays rather than under the bare wing); the nose leg on the
    # duct behind the lip, the mains under the wing roots.
    gear=[dict(name="nose", x=2.7, y=0.0, top=0.82, radius=0.3, width=0.17, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-1.3, y=1.16, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-1.3, y=-1.16, top=1.25, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    # The tanks' noses reach forward under the intake, well ahead of the wing.
    pylons=[((-0.75, 2.1, WING_Z - 0.08), 1.0, 0.3), ((-1.3, 3.35, WING_Z - 0.06), 0.9, 0.22)],
    stores=[("tank", (0.3, 2.1, 0.98), 4.6, 0.41), ("missile", (-1.2, 3.35, 1.38), 3.0, 0.09),
            ("missile", (-1.35, 4.86, WING_Z), 3.65, 0.09)],
    nose_probe=((7.15, 0.0, 1.95), 0.38),
    nav=dict(left=(2.6, 0.67, 1.14), right=(2.6, -0.67, 1.14), tail=(-7.27, 0.0, 2.4)),
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
    # The chin intake's lip, raked, and its mouth dark inside it.
    A.body("fuselage_intake_lip", [(0.0, 0.66, -0.36, 0.34, 0.0, 2.6), (-0.5, 0.66, -0.36, 0.34, 0.0, 2.6)],
           m["paint"], hull, seg=24, loc=INTAKE_LIP, rot=(0, INTAKE_RAKE, 0), steps=(1, 1, 1, 1))
    A.body("intake_mouth", [(0.016, 0.58, -0.29, 0.27, -0.01, 2.6), (0.004, 0.58, -0.29, 0.27, -0.01, 2.6)],
           m["black"], hull, seg=24, loc=INTAKE_LIP, rot=(0, INTAKE_RAKE, 0), steps=(1, 1, 1, 1))
    # Wing-tip rails under their AIM-120s.
    A.mirrored(lambda side, s: box(f"wing_tip_rail_{s}", (2.4, 0.08, 0.1), (-1.4, side * 4.8, WING_Z - 0.02),
                                   m["dark"], hull, bevel=0.01))
    # Block 50's HARM targeting pod and the Sniper pod on the intake's cheeks.
    for side, s, length in ((1, "L", 2.5), (-1, "R", 2.2)):
        box(f"cheek_pylon_{s}", (0.8, 0.12, 0.16), (2.2, side * 0.62, 0.92), m["paint"], hull, bevel=0.02, lods=MID)
        A.store(f"cheek_pod_{s}", "pod", (2.2, side * 0.72, 0.72), length, 0.17, m, hull)
    # Anti-glare panel ahead of the canopy, the refuelling receptacle behind it.
    box("antiglare", (0.8, 0.3, 0.03), (5.3, 0, 2.46), m["dark"], hull, rot=(0, 0.12, 0), lods=MID)
    box("refuel_door", (0.4, 0.2, 0.03), (-0.6, 0, 2.84), m["dark"], hull, lods=NEAR)
    # Speed brakes either side of the nozzle, closed.
    A.mirrored(lambda side, s: box(f"speed_brake_{s}", (0.9, 0.06, 0.55), (-6.45, side * 0.62, 1.86), m["paint"],
                                   hull, bevel=0.02, lods=MID))
    A.blade_antenna("antenna_spine", (-1.6, 0, 2.8), 0.22, m, hull)
    A.blade_antenna("antenna_belly", (0.2, 0, 0.92), 0.22, m, hull, down=True)
    A.markings(v, MARKINGS)


def wreck(variant, v):
    # Its litter kept within the wreck's footprint (the frame `test_jet` borrows).
    A.crash(v, tail_x=-4.2, wing_y=2.8, wing_side=-1, seed=16, litter_reach=0.95)


run("f_16_fighting_falcon", "us_compass_grey", build, wreck, script="f16.py")
