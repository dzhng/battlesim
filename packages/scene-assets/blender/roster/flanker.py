"""Su-27SM3, Su-30SM2, Su-35S and Su-34, from assets/references/sukhoi_flanker_family/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/flanker.py -- [--variant=<card>] [--wreck]

What the photos settle: one airframe family. A long drooping nose and a
bubble canopy (the Su-30's two-seat), the wide flat centre body blending into
the wing through long leading-edge extensions, two engine nacelles slung
apart under it with a tunnel between, their rectangular intakes raked under
the extensions, two upright fins on the nacelles, stabilators, ventral fins,
and the long tail stinger between the nozzles; tall gear with twin nose
wheels. The Su-30SM2 adds canards; the Su-35S drops the spine air brake and
carries wing-tip jamming pods; the Su-34 has the side-by-side cockpit under a
broad flattened nose, canards, tandem main wheels and a longer stinger. Flanker
blue-greys.

The references' dimensions (Sukhoi figures): Su-27SM3 21.9 x 14.7
x 5.92 m; Su-30SM2 21.935 x 14.7 x 6.36 m; Su-35S 21.9 x 15.3 x 5.9 m;
Su-34 23.34 x 14.7 x 6.09 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

SU27, SU30, SU35, SU34 = ("eastern_sukhoi_flanker_family_su_27sm3", "eastern_sukhoi_flanker_family_su_30sm2",
                          "eastern_sukhoi_flanker_family_su_35s", "eastern_sukhoi_flanker_family_su_34")
DIMENSIONS = {SU27: (21.9, 14.7, 5.92), SU30: (21.935, 14.7, 6.36), SU35: (21.9, 15.3, 5.9), SU34: (23.34, 14.7, 6.09)}

WING_Z = 2.55
# The engines' centres, and the tail booms' outboard of them (over the main
# wheels); the fins stand on the booms' tops.
NACELLE_Y, BOOM_Y, FIN_Z = 1.35, 2.3, 2.5


def spec(variant):
    vid = variant["id"]
    length, span, height = DIMENSIONS[vid]
    nose = length / 2
    su34 = vid == SU34
    # The Su-34 draws its nose broad and flat for the side-by-side cockpit.
    if su34:
        # The platypus: wide and flat, sharp chines along its sides (power
        # below 2) back to the cockpit.
        front = [(nose - 0.95, 0.0, 2.3, 2.3), (nose - 1.6, 0.7, 2.1, 2.52, 2.3, 1.6),
                 (nose - 3.0, 1.0, 1.9, 2.86, 2.3, 1.8), (6.4, 1.08, 1.76, 3.16, 2.3, 2.6),
                 (4.4, 1.12, 1.82, 3.12, 2.35, 3.0)]
    else:
        front = [(nose - 0.95, 0.0, 2.35, 2.35), (9.3, 0.42, 2.05, 2.65, 2.35), (8.0, 0.62, 1.85, 2.95, 2.32, 2.2),
                 (6.3, 0.72, 1.78, 3.12, 2.3, 2.4), (4.5, 0.9, 1.85, 3.05, 2.35, 2.6)]
    tail = -length / 2
    s = dict(
        fuselage=front + [(2.5, 1.6, 2.05, 2.96, 2.4, 3.5), (0.0, 2.0, 2.05, 2.9, 2.4, 4.0),
                          (-3.0, 1.9, 2.1, 2.85, 2.4, 4.0), (-6.0, 1.3, 2.15, 2.75, 2.42, 3.5),
                          (-8.5, 0.5, 2.25, 2.62, 2.42, 2.4), (tail + 0.05, 0.22, 2.35, 2.52, 2.42)],
        # The single seat's long bubble sits a fifth of the way back, behind
        # the IRST ball (three-quarter photos); the two-seater's runs on.
        canopy=(dict(x_front=7.4, x_back=4.6, sill=3.12, top=3.7, half_width=0.95, bows=(6.4,), peak=0.4, tail=0.7)
                if su34 else dict(x_front=6.6, x_back=2.6 if vid == SU30 else 3.6, sill=3.02, top=3.78,
                                  half_width=0.58, bows=(6.0, 4.4) if vid == SU30 else (6.0,), peak=0.4, tail=0.7)),
        # The Flanker's hump: the spine from the canopy back to the stinger.
        bodies=[("fuselage_spine", [((4.6 if su34 else 2.6 if vid == SU30 else 3.6), 0.5, 2.9, 3.56, 3.0, 2.4),
                                    (0.0, 0.62, 2.85, 3.22, 3.0, 2.6), (-6.0, 0.45, 2.6, 2.92, 2.7, 2.4),
                                    (tail + 0.6, 0.2, 2.42, 2.6)]),
                # The tail booms outboard of the nacelles (the front photos: the
                # fins stand over the main wheels), carrying fins and stabilators.
                *[(f"fuselage_boom_{k}", [(-2.4, 0.3, 2.15, 2.55, 2.35, 3.0), (-6.0, 0.36, 2.05, 2.6, 2.33, 3.0),
                                         (-9.2, 0.3, 2.1, 2.5, 2.32, 3.0), (-9.9, 0.12, 2.25, 2.4, 2.32)], y, 16)
                  for k, y in (("L", BOOM_Y), ("R", -BOOM_Y))]],
        wing_hinge=(0.12, (0.78, 0, 1)),
        nav=dict(left=(-3.4, span / 2 - 0.35, WING_Z), right=(-3.4, -span / 2 + 0.35, WING_Z),
                 tail=(tail + 0.3, 0.0, 2.6)),
        strake=[(6.0, 0.85, 2.86, 5.4, 0.12), (1.5, 1.95, WING_Z + 0.05, 0.4, 0.08)],
        wing=[(1.5, 1.9, WING_Z, 5.8, 0.36), (-3.6, 7.15, WING_Z - 0.1, 1.8, 0.07)],
        stab=[(-6.9, BOOM_Y, 2.32, 3.0, 0.14), (-8.9, 4.95, 2.3, 1.3, 0.05)],
        fins=[dict(root_x=-4.4, root_z=FIN_Z, height=height - FIN_Z, root_chord=3.6, tip_chord=1.4, sweep_m=2.4,
                   y=y) for y in (BOOM_Y, -BOOM_Y)]
             + [dict(root_x=-4.9, root_z=2.15, height=0.8, root_chord=1.6, tip_chord=0.9, sweep_m=0.6, y=y,
                     cant=2.95, rudder=False) for y in (BOOM_Y, -BOOM_Y)],
        intakes=[("rect", (3.02, NACELLE_Y, 1.66), 0.98, 0.94, 0.24, (0, 0.25, 0)),
                 ("rect", (3.02, -NACELLE_Y, 1.66), 0.98, 0.94, 0.24, (0, 0.25, 0))],
        nozzles=[dict(loc=(-8.4, NACELLE_Y, 1.85), r_front=0.62, r_exit=0.58, length=1.05, petals=14),
                 dict(loc=(-8.4, -NACELLE_Y, 1.85), r_front=0.62, r_exit=0.58, length=1.05, petals=14)],
        gear=[dict(name="nose", x=7.0, y=0.0, top=2.0, radius=0.34, width=0.18, wheels=2, spread=0.3,
                   door=(1.0, 0.6)),
              dict(name="main_L", x=-0.9, y=2.2, top=1.6, radius=0.52, width=0.28, door=(1.4, 0.7), rake=0.1,
                   tandem=1.0 if su34 else 0.0),
              dict(name="main_R", x=-0.9, y=-2.2, top=1.6, radius=0.52, width=0.28, door=(1.4, 0.7), rake=0.1,
                   tandem=1.0 if su34 else 0.0)],
        pylons=[((-0.6, 3.4, WING_Z - 0.1), 1.3, 0.34), ((-1.5, 4.8, WING_Z - 0.1), 1.1, 0.28)],
        stores=[("missile", (-0.8, 3.4, 1.92), 3.6, 0.12), ("missile", (-1.6, 4.8, 2.04), 3.0, 0.09),
                ("missile", (0.0, NACELLE_Y, 0.92), 3.6, 0.12)],
        nose_probe=((nose - 1.0, 0.0, 2.3 if su34 else 2.35), 0.95),
    )
    if vid in (SU30, SU34):
        s["canard"] = [(5.6, 1.0, 2.9, 1.7, 0.1), (4.7, 3.2, 2.9, 0.8, 0.04)]
    tip_y = span / 2 - 0.12
    if vid == SU35:
        s["stores"] += [("pod", (-3.0, tip_y, WING_Z - 0.1), 2.2, 0.12)]
    else:
        s["stores"] += [("missile", (-2.9, tip_y, WING_Z - 0.1), 2.9, 0.09)]
    if su34:
        s["stores"] += [("bomb", (1.0, 0.0, 1.55), 3.0, 0.25)]
    return s


def build(variant, v):
    s = spec(variant)
    m = A.jet(v, s)
    hull = v.hull
    vid = variant["id"]
    su34 = vid == SU34
    # The engine nacelles slung apart under the centre body.
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(3.0, 0.56, 1.16, 2.24, 1.66, 4.5), (0.0, 0.62, 1.12, 2.3, 1.68, 4.0),
                                (-4.0, 0.62, 1.2, 2.36, 1.75, 3.0), (-8.4, 0.6, 1.25, 2.45, 1.85)],
               m["paint"], hull, seg=20, loc=(0, side * NACELLE_Y, 0))
    tip_y = DIMENSIONS[vid][1] / 2 - 0.12
    A.mirrored(lambda side, k: box(f"wing_tip_rail_{k}", (2.4, 0.08, 0.1), (-3.0, side * tip_y, WING_Z - 0.08),
                                   m["dark"], hull))
    if vid != SU35:
        box("spine_brake", (2.0, 0.56, 0.05), (0.6, 0, 3.19), m["paint"], hull, bevel=0.02, lods=MID)
    # The infrared search sensor ahead of the windscreen.
    A.sensor_ball("irst", (7.05 if vid != SU34 else 7.6, 0.25 if vid != SU34 else 0.6, 3.06), 0.15, m, hull)
    box("antiglare", (1.0, 0.7 if vid != SU34 else 1.4, 0.03), (7.6 if vid != SU34 else 8.2, 0, 2.98), m["dark"],
        hull, rot=(0, 0.18, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-1.5, 0, 2.9), 0.25, m, hull)
    box("gun_port", (0.4, 0.1, 0.12), (3.8, -1.0, 2.7), m["black"], hull, lods=NEAR)
    # The VKS's red star on the fins (out and in) and under the wings; the
    # red bort number on the nose (52 and 01 in the side photos).
    fin_x = -4.4 - 2.4 * 0.72 - 1.0
    fin_z = FIN_Z + (DIMENSIONS[vid][2] - FIN_Z) * 0.72
    rows = [("insignia", dict(kind="ru_star", centre=(fin_x, y, fin_z), normal=(0, n, 0), up=(0, 0, 1), size=0.36,
                              onto=("tail_fin_0", "tail_fin_1"), mirror=False))
            for y, n in ((BOOM_Y + 0.25, 1), (BOOM_Y - 0.25, -1), (-BOOM_Y - 0.25, -1), (-BOOM_Y + 0.25, 1))]
    rows += [("insignia", dict(kind="ru_star", centre=(-1.6, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                               size=0.7, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)]
    bort = {SU27: "52", SU30: "27", SU35: "04", SU34: "01"}[vid]
    rows.append(("text", dict(text=bort, height=0.62, centre=((5.2 if su34 else 7.6), 1.4, 2.45), normal=(0, 1, 0),
                              up=(0, 0, 1), onto=("fuselage",), colour="red")))
    A.markings(v, rows)


def wreck(variant, v):
    # The span is the widest of the roster's: the torn panels lie a little
    # nearer, inside the wreck's footprint.
    A.crash(v, tail_x=-5.2, wing_y=4.6, wing_side=-1, tail_yaw=-0.25, seed=27, litter_reach=0.9)


run("sukhoi_flanker_family", "russian_air_blue", build, wreck, script="flanker.py")
