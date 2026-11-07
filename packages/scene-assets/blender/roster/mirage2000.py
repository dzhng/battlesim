"""Mirage 2000D (RMV), from assets/references/mirage_2000/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mirage2000.py -- [--wreck]

What the photos settle: the two-seat canopy of the strike Mirage, its fixed
probe on the right of the windscreen, half-round intakes with their shock
cones either side of the cockpit, the pure delta wing low on the body, one
tall fin, the single M53 nozzle; big tanks under the wings, bombs under the
body. French strike camouflage, grey with dark grey-green patches.

The references' dimensions (Dassault figures): length 14.36 m,
span 9.13 m, height 5.2 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`), made
from shaded views of the measured lofted model in guides/mirage2000.py (its
fin seated on the spine: drawn floating, the generator dropped it); on that
outline it stood 0.33 m high and spanned 0.16 m short (`lower`,
`span`), and it carries the probe. Its canopy is fitted to its hump; the
intakes' dark mouths and shock cones, the nozzle, gear, stores, lights and
markings are built on it, seated where it is (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from parts import cyl  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/mirage_2000/europe_mirage_2000_2000d_rmv.glb"
WING_Z = 1.55
SPAN = (1.5, 4.57)
# The two-seat canopy's plan, the lofted model's framed canopy's outline
# (3 m behind the nose and 4 m long), its opaque tail onto the spine; its
# heights are the skin's hump's.
CANOPY_GLASS = framed_plan(x_front=4.3, x_back=0.25, half_width=0.44, peak=0.4, tail=0.7)
CANOPY_TAIL = [CANOPY_GLASS[-1], (-0.15, 0.24), (-0.55, 0.03)]
# Each half-round intake's dark mouth inside its lips on the skin's face
# (`aircraft_skin.intake_mouth` corners, y and z), outboard of the cone the
# skin drew (a mouth across it would stand out at its tip); the shock cone
# built over the skin's.
MOUTH = ((0.98, 1.54), (1.16, 1.54), (1.16, 2.18), (0.98, 2.18))
CONE = (1.9, 0.78, 1.84)
# The M53 in place of the skin's nozzle, the skin cut inside it.
NOZZLE = dict(loc=(-6.4, 0.0, 2.06), r_front=0.52, r_exit=0.5, length=0.8, petals=14)
NOZZLE_CUT = ((-7.6, -6.5), (-0.48, 0.48), (1.58, 2.54))
SPEC = dict(
    # The skin: its delta out from the intakes, its fin above the spine.
    skin=dict(path=SKIN, length=14.38, top=5.2, lower=(0.33, 3.25), span=SPAN, cut=[NOZZLE_CUT],
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(3.6, 2.0)),
              parts=[("wing_L", ((-6.5, 2.5), (1.35, 6.0), (1.1, 1.95))),
                     ("wing_R", ((-6.5, 2.5), (-6.0, -1.35), (1.1, 1.95))),
                     ("tail_fin_0", ((-7.5, -2.8), (-0.3, 0.3), (2.7, 9.0)))]),
    nozzles=[NOZZLE],
)
GEAR = [dict(name="nose", x=2.9, y=0.0, radius=0.25, width=0.14, wheels=2, spread=0.2, light=True),
        dict(name="main_L", x=-2.2, y=1.6, radius=0.38, width=0.2, door=(1.0, 0.5)),
        dict(name="main_R", x=-2.2, y=-1.6, radius=0.38, width=0.2, door=(1.0, 0.5))]
# The big tanks on the wing pylons (x, y, pylon length, depth; tank x
# offset, length, radius) and the bombs under the body (x, y).
TANK = ((-0.4, 2.0, 1.2, 0.14), (-0.1, 4.6, 0.4))
BOMBS = [(0.6, 0.45), (-2.0, 0.45)]
TIP_X = -5.2


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    (x, y, length, depth), (dx, t_len, radius) = TANK
    for side in (1, -1):
        top = shell.under(x, side * y)
        A.pylon(f"pylon_0_{side + 1}", (x, side * y, top), length, depth, m, hull)
        A.store(f"store_tank_{side + 1}", "tank", (x + dx, side * y, top - depth - radius - 0.02), t_len, radius, m,
                hull)
    for k, (x, y) in enumerate(BOMBS):
        A.mirrored(lambda side, s, x=x, y=y, k=k: A.store(f"store_bomb_{k}_{s}", "bomb",
                                                           (x, side * y, shell.under(x, side * y) - 0.23), 2.4, 0.2, m,
                                                           hull, rail=False))
    tip = SPAN[1]
    fin = Probe(hull, ("tail_fin",))
    A.nav_lights(m, hull, tail=(-6.5, 0.0, fin.over(-6.5, 0.0) - 0.1),
                 **{k: (TIP_X, side * (tip - 0.15), shell.over(TIP_X, side * (tip - 0.15)) + 0.01)
                    for k, side in (("left", 1), ("right", -1))})
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH))
    x, y, z = CONE
    A.mirrored(lambda side, k: cyl(f"shock_cone_{k}", 0.22, 0.55, (x, side * y, z), "X", m["paint"], hull, seg=14,
                                   r2=0.02))
    A.blade_antenna("antenna_spine", (-2.0, 0, shell.over(-2.0, 0)), 0.22, m, hull)
    # The Armée de l'air's roundels (aft on the body and on the wings) and
    # the 3-IM code of the photo on the nose.
    A.markings(v, [
        ("insignia", dict(kind="fr", centre=(-5.6, 1.0, 2.06), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="fr", centre=(-2.6, 3.0, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.42,
                          onto=("wing_",))),
        ("text", dict(text="3-IM", height=0.26, centre=(3.1, 1.0, 1.8), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.2, wing_y=2.8, wing_side=-1, tail_yaw=-0.3, seed=20)


run("mirage_2000", "french_air_camo", build, wreck, script="mirage2000.py")
