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

Each variant's shell is a generated skin (`aircraft_skin`, pinned in
`skins/`), made from shaded views of the measured lofted model in
guides/tornado.py, which it matches to a few centimetres but for the IDS's
standing 0.12 m high (`lower`) and both spans a few centimetres off (`span`). Its canopy is fitted to its hump; the
intakes' dark mouths, the nozzles and buckets, gear, stores, lights, the
variant's nose fittings and markings are built on it, seated where it is
(`Probe`). The radome stays in the airframe's grey: a split along its
joint would cut a ragged line across the skin.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

IDS, ECR = "europe_tornado_ids", "europe_tornado_ecr"
WING_Z = 2.3
SPAN = (2.2, 6.89)
# The tandem canopy's plan, the lofted model's framed canopy's outline, its
# opaque tail onto the spine; its heights are the skin's hump's.
CANOPY_GLASS = framed_plan(x_front=5.6, x_back=1.45, half_width=0.5, peak=0.4, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (1.05, 0.27), (0.65, 0.03)]
# Each raked intake's dark mouth inside its lips on the skin's face
# (`aircraft_skin.intake_mouth` corners, y and z), each variant's skin's own.
MOUTH = {ECR: ((0.88, 1.54), (1.38, 1.54), (1.38, 2.3), (0.88, 2.3)),
         IDS: ((0.88, 1.5), (1.4, 1.5), (1.4, 2.46), (0.88, 2.46))}
# The IDS's skin stood high on its outline; the ECR's matches it.
LOWER = {ECR: None, IDS: (0.12, 3.45)}
# The two RB199s in place of the skin's nozzles, the skin cut inside them.
NOZZLES = [dict(loc=(-6.3, y, 2.1), r_front=0.45, r_exit=0.42, length=0.95, petals=10) for y in (0.48, -0.48)]
# One box over both: between the pipes the skin's own tail would be left in shreds.
NOZZLE_CUT = [((-8.6, -6.4), (-1.08, 1.08), (1.5, 2.52))]
GEAR = [dict(name="nose", x=4.1, y=0.0, radius=0.28, width=0.15, wheels=2, spread=0.24, door=(0.8, 0.5), light=True),
        dict(name="main_L", x=-1.2, y=1.55, radius=0.46, width=0.24, door=(1.2, 0.55), rake=0.1),
        dict(name="main_R", x=-1.2, y=-1.55, radius=0.46, width=0.24, door=(1.2, 0.55), rake=0.1)]
# The pylons (x, y, length, depth) from the skin's underside and what hangs
# under each (kind, x offset, length, radius): tanks inboard and the
# self-defence pods outboard on the wing's swivel pylons, the variant's
# store on the shoulder pylons.
WING_STORES = [((-0.5, 3.6, 1.1, 0.36), ("tank", -0.1, 5.0, 0.44)), ((-1.45, 5.3, 0.9, 0.24), ("pod", 0.1, 3.0, 0.16))]
SHOULDER = {ECR: ((1.2, 0.95, 1.6, 0.18), ("missile", -0.2, 4.17, 0.13)),
            IDS: ((1.2, 0.95, 1.6, 0.18), ("bomb", 0.0, 2.6, 0.2))}
TIP_X = -2.3


def spec(variant):
    return dict(
        # The skin: its swing wings outboard of the gloves, its fin above
        # the spine; the tailerons stay with the body.
        skin=dict(path=f"assets/source/skins/tornado/{variant['id']}.glb", length=16.68, top=5.95, span=SPAN, lower=LOWER[variant["id"]],
                  cut=NOZZLE_CUT, canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(4.6, 2.9)),
                  parts=[("wing_L", ((-3.5, 1.5), (2.6, 8.0), (1.9, 2.7))),
                         ("wing_R", ((-3.5, 1.5), (-8.0, -2.6), (1.9, 2.7))),
                         ("tail_fin_0", ((-8.5, -2.8), (-0.3, 0.3), (3.05, 9.0)))]),
        nozzles=NOZZLES,
    )


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    shell = Probe(hull)
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(WING_STORES + [SHOULDER[variant["id"]]]):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull, rail=kind == "missile")
    tip = SPAN[1]
    fin = Probe(hull, ("tail_fin",))
    A.nav_lights(m, hull, tail=(-7.3, 0.0, fin.over(-7.3, 0.0) - 0.1),
                 **{k: (TIP_X, side * (tip - 0.15), shell.over(TIP_X, side * (tip - 0.15)) + 0.01)
                    for k, side in (("left", 1), ("right", -1))})
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH[variant["id"]]))
    # The thrust reverser buckets stowed on top of the nozzles.
    A.mirrored(lambda side, k: box(f"reverser_{k}", (0.9, 0.9, 0.08), (-6.55, side * 0.48, 2.58), m["paint"], hull,
                                   bevel=0.02, rot=(0, -0.1, 0), lods=MID))
    if variant["id"] == IDS:
        A.mirrored(lambda side, k: box(f"gun_port_{k}", (0.3, 0.1, 0.1), (4.9, shell.flank(4.9, side, 1.66), 1.66),
                                       m["black"], hull, lods=NEAR))
    else:
        # The emitter locator's fairings either side of the nose, where the
        # IDS has its cannon: long blisters, their dark antenna windows
        # facing forward.
        def els(side, k):
            y = shell.flank(5.0, side, 1.9) + side * 0.04
            box(f"els_fairing_{k}", (1.5, 0.18, 0.24), (5.0, y, 1.9), m["paint"], hull, bevel=0.06, taper=(0.8, 0.7),
                lods=MID)
            box(f"els_window_{k}", (0.03, 0.14, 0.16), (5.73, y, 1.9), m["dark"], hull, lods=NEAR)
        A.mirrored(els)
    A.blade_antenna("antenna_spine", (-2.4, 0, shell.over(-2.4, 0)), 0.22, m, hull)
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
