"""F/A-18E Super Hornet and EA-18G Growler, from assets/references/f_a_18_super_hornet/
and assets/references/ea_18g_growler/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/super_hornet.py -- [--variant=<card>] [--wreck]

What the photos settle: one airframe. Long leading-edge extensions from the
cockpit to the wing root, the caret intakes tucked under them, a shoulder
wing with dogtooth, twin fins canted out, stabilators below them, two
nozzles close together; tall twin nose wheels and main gear on the body's
flanks. The E is a single seat with wing-tip AIM-9s, tanks and missiles;
the Growler has the two-seat canopy, ALQ-218 receiver pods on the wing tips
and ALQ-99 jamming pods (with their ram-air turbines) under the wings and
the body. Both in the Navy's tactical greys.

The references' dimensions (Navy fact file figures agree): length
18.31 m, span 13.62 m over the tip stores, height 4.88 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

E, G = "us_f_a_18_super_hornet_f_a_18e", "us_ea_18g_growler_electronic_warfare_aircraft"

WING_Z = 2.45
# The nose's tip and the tail's last point (the nozzles' and fins' trailing edges).
NOSE_X, TAIL_X = 9.28, -8.9
TIP_Y = 6.72
# The wing's sections at its root and tip, and at the fold between them,
# where the outboard panel's leading edge stands proud of the inboard one's:
# the dogtooth (front photo).
# Measured on the public-domain F/A-18F plan silhouette: 0.8 m further aft
# than the lofted model had it.
_ROOT, _TIP = (0.95, 1.1, WING_Z, 4.7, 0.28), (-2.25, TIP_Y - 0.08, WING_Z - 0.02, 1.6, 0.06)
_F = (4.2 - _ROOT[1]) / (_TIP[1] - _ROOT[1])
FOLD = tuple(a + (b - a) * _F for a, b in zip(_ROOT, _TIP))
DOGTOOTH = 0.28
OUTER_WING = [(FOLD[0] + DOGTOOTH, FOLD[1], FOLD[2], FOLD[3] + DOGTOOTH, FOLD[4]), _TIP]
# An intake trunk's stations, about its own centreline 1 m off the body's.
TRUNK = [(3.7, 0.4, 1.47, 2.33, 1.9, 4.0), (1.6, 0.4, 1.44, 2.38, 1.9, 4.0), (-0.8, 0.34, 1.48, 2.3, 1.9, 3.5),
         (-2.8, 0.12, 1.62, 2.16, 1.9, 3.0)]


def spec(variant):
    growler = variant["id"] == G
    s = dict(
        # The windscreen 3.4 m back from the nose, over the nose gear (side
        # photo); the body deep under the LEX and the spine behind the cockpit.
        fuselage=[(9.28, 0.0, 2.0, 2.0), (8.5, 0.36, 1.74, 2.36, 2.04), (7.2, 0.58, 1.6, 2.76, 2.1),
                  (5.8, 0.68, 1.52, 2.96, 2.15, 2.5), (4.0, 0.8, 1.46, 2.98, 2.15, 2.8),
                  (2.0, 1.08, 1.36, 3.12, 2.15, 2.6), (-1.0, 1.16, 1.36, 3.18, 2.1, 2.8),
                  (-4.0, 1.06, 1.44, 2.86, 2.05, 2.6), (-6.6, 0.96, 1.54, 2.6, 1.96, 2.6),
                  (-7.7, 0.92, 1.58, 2.42, 1.96)],
        canopy=dict(x_front=5.95, x_back=2.0 if growler else 2.9, sill=2.86, top=3.52, half_width=0.5,
                    bows=(5.15, 3.5) if growler else (5.15,), peak=0.4, tail=0.7),
        # The spine stays high behind the canopy back to the fins (side photo).
        # The body's broad back stays high from the canopy to the fins (side
        # photo); the spine only fairs the canopy into it.
        bodies=[("fuselage_spine", [(2.2 if growler else 3.1, 0.38, 2.75, 3.36, 2.75, 2.2),
                                    (1.2, 0.5, 2.8, 3.23, 2.8, 2.0), (-0.8, 0.0, 3.1, 3.1)]),
                # The intake trunks under the LEX, boxy, from the caret mouths
                # aft into the body's flanks over the main gear.
                ("fuselage_trunk_L", TRUNK, 1.0, 16), ("fuselage_trunk_R", TRUNK, -1.0, 16)],
        wing_hinge=(0.12, 0.76),
        strake=[(6.4, 0.55, 2.55, 5.4, 0.1), (1.15, 1.85, WING_Z + 0.04, 0.95, 0.07)],
        # The inboard wing out to the fold; the outboard panel and its
        # dogtooth are `build`'s (`OUTER_WING`).
        wing=[_ROOT, FOLD],
        stab=[(-5.65, 0.9, 2.02, 2.7, 0.12), (-7.65, 3.45, 1.96, 1.25, 0.05)],
        fins=[dict(root_x=-3.55, root_z=2.5, height=2.5, root_chord=3.2, tip_chord=1.3, sweep_m=2.15, y=0.78,
                   cant=0.35),
              dict(root_x=-3.55, root_z=2.5, height=2.5, root_chord=3.2, tip_chord=1.3, sweep_m=2.15, y=-0.78,
                   cant=0.35)],
        # The caret mouths: big, square, right under the LEX, raked back
        # from the top lip and swept back outboard (front and side photos).
        intakes=[("rect", (3.72, 1.0, 1.9), 0.72, 0.84, 0.2, (0, 0.28, 0.22)),
                 ("rect", (3.72, -1.0, 1.9), 0.72, 0.84, 0.2, (0, 0.28, -0.22))],
        nozzles=[dict(loc=(-7.65, 0.5, 1.96), r_front=0.5, r_exit=0.46, length=0.98, petals=14),
                 dict(loc=(-7.65, -0.5, 1.96), r_front=0.5, r_exit=0.46, length=0.98, petals=14)],
        gear=[dict(name="nose", x=3.9, y=0.0, top=1.6, radius=0.33, width=0.17, wheels=2, spread=0.26,
                   door=(0.9, 0.5), light=True),
              dict(name="main_L", x=-1.6, y=1.56, top=1.4, radius=0.5, width=0.25, door=(1.2, 0.55), rake=0.1),
              dict(name="main_R", x=-1.6, y=-1.56, top=1.4, radius=0.5, width=0.25, door=(1.2, 0.55), rake=0.1)],
        pylons=[((-1.2, 2.6, WING_Z - 0.1), 1.2, 0.35), ((-1.8, 4.0, WING_Z - 0.08), 1.1, 0.3)],
        stores=[("tank", (-1.3, 2.6, 1.6), 4.7, 0.42)],
        nav=dict(left=(-1.8, TIP_Y - 0.3, WING_Z + 0.04), right=(-1.8, -TIP_Y + 0.3, WING_Z + 0.04),
                 tail=(-8.6, 0.0, 1.96)),
    )
    if growler:
        s["stores"] += [("jammer", (-1.7, 4.0, 1.66), 4.6, 0.36), ("jammer", (-1.0, 0.0, 0.98), 4.6, 0.36),
                        ("pod", (-2.4, TIP_Y, WING_Z), 3.0, 0.23)]
    else:
        s["stores"] += [("missile", (-1.8, 4.0, 1.9), 3.65, 0.09), ("missile", (-2.4, TIP_Y, WING_Z), 3.0, 0.065)]
    return s


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    if variant["id"] != G:
        box("gun_port", (0.25, 0.1, 0.12), (8.4, 0, 2.48), m["black"], hull, lods=NEAR)
        A.mirrored(lambda side, k: box(f"wing_tip_rail_{k}", (2.2, 0.08, 0.1), (-1.5, side * (TIP_Y - 0.02),
                                                                                 WING_Z - 0.02), m["dark"], hull))
    # The outboard wing panels, their leading edges stepped forward at the
    # fold into the dogtooth.
    A.paired_surface("wing_outer", OUTER_WING, m["paint"], hull, hinge=(0.12, 0.76), hinge_mat=m["black"])
    # The nose leg's catapult launch bar, stowed up and forward off the strut.
    nose_leg = next(o for o in hull.children if o.name == "gear_nose")
    box("gear_nose_launch_bar", (0.9, 0.07, 0.07), (0.45, 0, -0.78), m["gear"], nose_leg, rot=(0, -0.55, 0),
        lods=MID)
    # The refuelling probe's door, antiglare.
    box("probe_door", (0.8, 0.12, 0.03), (6.6, -0.46, 2.72), m["dark"], hull, rot=(0.5, 0, 0), lods=NEAR)
    box("antiglare", (0.9, 0.5, 0.03), (6.5, 0, 2.87), m["dark"], hull, rot=(0, 0.1, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-1.0, 0, 3.19), 0.24, m, hull)
    A.blade_antenna("antenna_belly", (0.4, 0, 1.38), 0.2, m, hull, down=True)
    # The arrestor hook under the tail.
    box("arrestor_hook", (1.6, 0.08, 0.08), (-6.8, 0, 1.5), m["steel"], hull, rot=(0, 0.12, 0), lods=MID)
    # The Navy's low-visibility greys: the insignia on the intake below the
    # LEX, NAVY aft, the squadron on the LEX and its air wing's code on the fins.
    code, squadron, modex = ("NL", "VAQ-133", "500") if variant["id"] == G else ("AJ", "VFA-31", "200")
    fin_n, fin_up = (0, math.cos(0.35), -math.sin(0.35)), (0, math.sin(0.35), math.cos(0.35))
    A.markings(v, [
        ("insignia", dict(kind="us_lowvis", centre=(4.6, 1.3, 1.95), normal=(0, 1, 0), up=(0, 0, 1), size=0.36,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.0, 4.2, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_L", "wing_outer_L"), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.0, -4.2, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.55, onto=("wing_R", "wing_outer_R"), mirror=False)),
        ("text", dict(text="NAVY", height=0.3, centre=(-4.6, 1.5, 2.05), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",))),
        ("text", dict(text=squadron, height=0.2, centre=(2.4, 1.6, 2.62), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage", "wing_strake"))),
        ("text", dict(text=modex, height=0.26, centre=(7.6, 1.0, 2.3), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",))),
        ("text", dict(text=code, height=0.6, centre=(-5.4, 0.78 + 1.5 * fin_up[1] + 0.3, 2.5 + 1.5 * fin_up[2]),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",))),
    ])
    # The stations run from the nose at 9.28 m to the tail at -8.9 m: the
    # airframe stands centred on its length, as its frame's hull is.
    hull.location.x = -(NOSE_X + TAIL_X) / 2


def wreck(variant, v):
    # Its litter kept within the wreck's footprint on this wide a span.
    A.crash(v, tail_x=-5.0, wing_y=3.6, wing_side=-1, tail_yaw=-0.3, seed=18, litter_reach=0.85)


run("f_a_18_super_hornet", "us_compass_grey", build, wreck, appearances=[E, G], script="super_hornet.py",
    receipt_file="assets/source/roster/f_a_18_super_hornet/super_hornet.source-receipt.json",
    references=[f"assets/references/{f}/references.json" for f in ("ea_18g_growler", "f_a_18_super_hornet")])
