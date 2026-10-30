"""Panavia Tornado IDS and ECR, from assets/references/tornado/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/tornado.py -- [--variant=<card>] [--wreck]

What the photos settle: the long black radome, the tandem two-seat canopy, big
raked rectangular intakes either side behind the cockpit, the swing wings
outboard of fixed gloves (drawn spread at 25 degrees, as parked and taxiing),
the very tall fin, all-moving tailerons with anhedral, two RB199 nozzles
with their reverser buckets; tanks on the wing's swivel pylons. The IDS has
the two cannon under the nose; the ECR has none and carries HARMs on its
shoulder pylons. Luftwaffe grey.

The references' dimensions (Panavia figures): length 16.72 m,
span 13.91 m spread, height 5.95 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

IDS, ECR = "europe_tornado_ids", "europe_tornado_ecr"
WING_Z = 2.3


def spec(variant):
    ecr = variant["id"] == ECR
    s = dict(
        # The side photo (a Luftwaffe IDS): the long two-seat canopy from
        # 2.75 m behind the nose, the forward body narrow (the front photo:
        # the radome and cockpit no wider than one intake), its spine 3.1 m
        # up and its belly 1.3 m; the intakes under the rear seat, both gear aft.
        fuselage=[(8.36, 0.0, 2.0, 2.0), (7.5, 0.42, 1.7, 2.3, 2.0), (6.3, 0.56, 1.55, 2.62, 1.98, 2.2),
                  (4.8, 0.62, 1.45, 2.88, 2.0, 2.4), (3.0, 0.68, 1.36, 3.0, 2.05, 2.8), (1.0, 0.8, 1.3, 3.08, 2.1, 3.2),
                  (-2.5, 0.95, 1.32, 3.0, 2.1, 3.5), (-5.5, 1.0, 1.45, 2.76, 2.05, 3.0), (-7.0, 0.9, 1.55, 2.5, 2.0)],
        canopy=dict(x_front=5.6, x_back=1.45, sill=2.86, top=3.42, half_width=0.5, bows=(4.6, 2.9), peak=0.4,
                    tail=0.7),
        # The intake boxes stand proud of the body either side from under
        # the rear seat (the front photo: their outer walls 1.4 m out, as
        # wide as the radome each), slab-sided, fairing into the engine bay.
        bodies=[("fuselage_spine", [(1.55, 0.42, 2.95, 3.32, 3.05, 2.4), (-1.5, 0.5, 2.95, 3.2, 3.05, 2.6),
                                    (-4.6, 0.0, 2.85, 2.85)]),
                *[(f"fuselage_intake_{s}", [(1.95, 0.38, 1.42, 2.6, 2.0, 5.0), (0.2, 0.42, 1.36, 2.64, 2.0, 5.0),
                                             (-2.2, 0.38, 1.38, 2.56, 2.0, 4.0), (-4.4, 0.0, 1.9, 1.9)],
                   1.0 if s == "L" else -1.0, 16) for s in "LR"]],
        wing_hinge=(0.1, (0.75, 0, 1)),
        nav=dict(left=(-2.0, 6.8, WING_Z), right=(-2.0, -6.8, WING_Z), tail=(-7.3, 0.0, 3.05)),
        # The fixed glove: its sharp leading edge from the top of the intake
        # box out to the wing's pivot.
        strake=[(1.9, 1.3, 2.5, 3.4, 0.3), (0.5, 2.0, WING_Z + 0.04, 2.2, 0.2)],
        wing=[(0.6, 1.95, WING_Z, 2.6, 0.18), (-1.7, 6.88, WING_Z, 1.3, 0.06)],
        stab=[(-5.0, 0.9, 1.92, 2.8, 0.14), (-7.05, 3.45, 1.72, 1.25, 0.05)],
        # The fin's broad, squared tip (the side photo): swept less than
        # its root fairing suggests, its trailing edge near upright.
        fins=[dict(root_x=-3.0, root_z=2.95, height=3.0, root_chord=4.4, tip_chord=2.3, sweep_m=2.7)],
        # The mouths, raked with the top lip forward.
        intakes=[("rect", (2.02, 1.0, 2.0), 0.64, 1.06, 0.2, (0, 0.3, 0)),
                 ("rect", (2.02, -1.0, 2.0), 0.64, 1.06, 0.2, (0, 0.3, 0))],
        nozzles=[dict(loc=(-7.0, 0.48, 1.92), r_front=0.45, r_exit=0.42, length=0.95, petals=10),
                 dict(loc=(-7.0, -0.48, 1.92), r_front=0.45, r_exit=0.42, length=0.95, petals=10)],
        gear=[dict(name="nose", x=4.1, y=0.0, top=1.5, radius=0.28, width=0.15, wheels=2, spread=0.24,
                   door=(0.8, 0.5), light=True),
              dict(name="main_L", x=-1.2, y=1.55, top=1.42, radius=0.46, width=0.24, door=(1.2, 0.55), rake=0.1),
              dict(name="main_R", x=-1.2, y=-1.55, top=1.42, radius=0.46, width=0.24, door=(1.2, 0.55), rake=0.1)],
        # Tanks inboard and the self-defence pods outboard on the wing's
        # swivel pylons (the front photo), stores on the shoulder pylons.
        pylons=[((0.1, 3.6, WING_Z - 0.08), 1.1, 0.36), ((1.2, 0.95, 1.42), 1.6, 0.18),
                ((-1.45, 5.3, WING_Z - 0.06), 0.9, 0.24)],
        stores=[("tank", (0.0, 3.6, 1.42), 5.0, 0.44), ("pod", (-1.35, 5.3, 1.86), 3.0, 0.16)],
    )
    if ecr:
        s["stores"] += [("missile", (1.0, 0.95, 1.06), 4.17, 0.13)]
    else:
        s["stores"] += [("bomb", (1.2, 0.95, 1.02), 2.6, 0.2)]
    return s


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    # The thrust reverser buckets above the nozzles, stowed.
    A.mirrored(lambda side, k: box(f"reverser_{k}", (0.9, 0.9, 0.08), (-7.2, side * 0.48, 2.4), m["paint"], hull,
                                   bevel=0.02, rot=(0, -0.1, 0), lods=MID))
    if variant["id"] == IDS:
        A.mirrored(lambda side, k: box(f"gun_port_{k}", (0.3, 0.1, 0.1), (4.9, side * 0.58, 1.66), m["black"], hull,
                                       lods=NEAR))
    else:
        # The emitter locator's fairings either side of the nose, where the
        # IDS has its cannon: long blisters, their dark antenna windows
        # facing forward.
        A.mirrored(lambda side, k: box(f"els_fairing_{k}", (1.5, 0.18, 0.24), (5.0, side * 0.6, 1.9), m["paint"], hull,
                                       bevel=0.06, taper=(0.8, 0.7), lods=MID))
        A.mirrored(lambda side, k: box(f"els_window_{k}", (0.03, 0.14, 0.16), (5.76, side * 0.6, 1.9), m["dark"], hull,
                                       lods=NEAR))
    # The black radome.
    A.body("nose_radome", [(8.38, 0.0, 2.0, 2.0), (7.5, 0.43, 1.69, 2.31, 2.0), (7.05, 0.5, 1.62, 2.42, 2.0)],
           m["dark"], hull, seg=24)
    box("antiglare", (0.9, 0.6, 0.03), (6.05, 0, 2.62), m["dark"], hull, rot=(0, 0.16, 0), lods=MID)
    box("probe_fairing", (1.4, 0.12, 0.12), (4.9, -0.56, 2.72), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (-2.4, 0, 3.10), 0.22, m, hull)
    # The Luftwaffe's markings, as the photo: the cross and the 46+07
    # on the body, the flag on the fin, crosses on the wings.
    serial = "46+25" if variant["id"] == ECR else "46+07"
    A.markings(v, [
        ("insignia", dict(kind="de", centre=(2.85, 1.4, 1.85), normal=(0, 1, 0), up=(0, 0, 1), size=0.22,
                          onto=("fuselage",))),
        ("text", dict(text=serial.split("+")[0], height=0.3, centre=(3.6, 1.4, 1.85), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ("text", dict(text=serial.split("+")[1], height=0.3, centre=(2.15, 1.4, 1.85), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ("insignia", dict(kind="de_flag", centre=(-6.4, 0.3, 4.9), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("tail_fin_0",))),
        ("insignia", dict(kind="de", centre=(-1.4, 4.6, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.42,
                          onto=("wing_",))),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.6, wing_y=4.0, wing_side=1, tail_yaw=0.25, seed=12)


run("tornado", "nato_air_grey", build, wreck, script="tornado.py")
