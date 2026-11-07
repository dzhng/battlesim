"""Dassault Rafale C (F4), from assets/references/rafale/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/rafale.py -- [--wreck]

What the photos settle: the canards mounted high, above and behind the two
crescent intakes set low under the cockpit, the fixed refuelling probe on the
right of the nose, the cranked delta wing, one tall swept fin, two M88
nozzles; wing-tip missiles, tanks and Meteors under the wing. Mid grey.

The references' dimensions (Dassault figures): length 15.27 m,
span 10.80 m, height 5.34 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`), made
from shaded views of the lofted model this script drew before it: it stood
0.22 m high and spanned 0.3 m wide on that outline (`lower`, `span`), and
carries the probe and the intakes. Its canopy is fitted to its hump; the
intakes' dark mouths, the nozzles, gear, stores, lights and markings are
built on it, seated where it is (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/rafale/europe_rafale_c_f4.glb"
WING_Z = 1.85
SPAN = (2.0, 5.25)
# The canopy's plan, the lofted model's framed canopy's outline (the long
# canopy from 2.3 m behind the nose), its opaque tail onto the spine; its
# heights are the skin's hump's.
CANOPY_GLASS = framed_plan(x_front=5.35, x_back=1.7, half_width=0.47, peak=0.42, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (1.3, 0.26), (0.9, 0.03)]
# Each crescent intake's dark mouth inside its lips on the skin's face
# (`aircraft_skin.intake_mouth` corners, y and z).
MOUTH = ((0.78, 1.45), (1.12, 1.45), (1.12, 1.95), (0.78, 1.95))
# The two M88s in place of the skin's nozzles, the skin cut inside them.
NOZZLES = [dict(loc=(-6.0, y, 2.02), r_front=0.42, r_exit=0.4, length=0.75, petals=12) for y in (0.42, -0.42)]
# One box over both: between the pipes the skin's own tail would be left in shreds.
NOZZLE_CUT = [((-7.3, -6.1), (-0.8, 0.8), (1.5, 2.4))]
SPEC = dict(
    # The skin: its wings out from the intakes' trunks, its canards high
    # over the intakes, its fin above the spine.
    skin=dict(path=SKIN, length=15.23, top=5.34, lower=(0.22, 3.5), span=SPAN, cut=NOZZLE_CUT,
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(4.75,)),
              parts=[("wing_L", ((-6.5, 1.3), (1.25, 6.0), (1.4, 2.4))),
                     ("wing_R", ((-6.5, 1.3), (-6.0, -1.25), (1.4, 2.4))),
                     ("wing_canard_L", ((1.0, 3.4), (1.0, 2.6), (2.15, 2.75))),
                     ("wing_canard_R", ((1.0, 3.4), (-2.6, -1.0), (2.15, 2.75))),
                     ("tail_fin_0", ((-7.8, -2.2), (-0.3, 0.3), (2.85, 9.0)))]),
    nozzles=NOZZLES,
)
GEAR = [dict(name="nose", x=3.05, y=0.0, radius=0.27, width=0.15, wheels=2, spread=0.22, light=True),
        dict(name="main_L", x=-1.8, y=1.32, radius=0.4, width=0.2, door=(1.0, 0.55), rake=0.1),
        dict(name="main_R", x=-1.8, y=-1.32, radius=0.4, width=0.2, door=(1.0, 0.55), rake=0.1)]
# The wing pylons (x, y, length, depth) from the skin's underside and what
# hangs under each (kind, x offset, length, radius): tanks inboard, Meteors
# outboard; the tips' missiles on their rails, measured at the tip's chord.
PYLONS = [((-2.0, 2.0, 1.1, 0.3), ("tank", -0.1, 4.2, 0.38)), ((-3.0, 3.3, 1.0, 0.24), ("missile", 0.0, 3.65, 0.09))]
TIP_X = -4.4


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(PYLONS):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull, rail=kind == "missile")
    tip = SPAN[1]
    for side, s in ((1, "L"), (-1, "R")):
        z = shell.under(TIP_X, side * (tip - 0.1))
        A.store(f"store_tip_{s}", "missile", (TIP_X, side * tip, z - 0.1), 3.1, 0.08, m, hull)
    fin = Probe(hull, ("tail_fin",))
    A.nav_lights(m, hull, tail=(-7.3, 0.0, fin.over(-7.3, 0.0) - 0.1),
                 **{k: (TIP_X, side * (tip - 0.3), shell.over(TIP_X, side * (tip - 0.3)) + 0.01)
                    for k, side in (("left", 1), ("right", -1))})
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH))
    A.blade_antenna("antenna_spine", (-1.4, 0, shell.over(-1.4, 0)), 0.22, m, hull)
    # The Armée de l'air's roundels and fin title, the 30-GT code of the photo.
    A.markings(v, [
        ("insignia", dict(kind="fr", centre=(1.2, 1.2, 2.2), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="fr", centre=(-3.6, 2.6, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.4,
                          onto=("wing_",))),
        ("text", dict(text="ARMEE DE L'AIR", height=0.12, centre=(-4.7, 0.3, 2.95), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="lowvis_dark")),
        ("text", dict(text="30-GT", height=0.22, centre=(3.3, 1.0, 1.85), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="lowvis_dark")),
    ])


def wreck(variant, v):
    # Its litter kept within the wreck's footprint (its wide delta).
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=7, litter_reach=0.9)


run("rafale", "nato_air_grey", build, wreck, script="rafale.py")
