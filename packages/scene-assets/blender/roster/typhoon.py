"""Eurofighter Typhoon (Tranche 3), from assets/references/eurofighter_typhoon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/typhoon.py -- [--wreck]

What the photos settle: the long pointed nose, canards just behind and below
the cockpit, the big rectangular chin intake under the forward body with its
variable lip, the cranked delta wing with no tailplane, one swept fin, two
EJ200 nozzles side by side; wing-tip defensive-aids pods, tanks and missiles
under the wing, missiles semi-recessed under the body. Light grey.

The references' dimensions (Eurofighter figures agree): length
15.96 m, span 10.95 m, height 5.28 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`), made
from shaded views of the lofted model this script drew before it, which it
matches to a few centimetres but for a span 0.3 m short (`SPAN`). Its canopy
is fitted to its hump; the chin intake's dark mouth, the nozzles, gear,
stores, lights and markings are built on it, seated where it is (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/eurofighter_typhoon/europe_eurofighter_typhoon_tranche_3.glb"
WING_Z = 1.9
# The skin's span to the frame's half span (its tips' pods outboard).
SPAN = (2.0, 5.32)
# The canopy's plan, the lofted model's framed canopy's outline, its
# opaque tail onto the spine; its heights are the skin's hump's.
CANOPY_GLASS = framed_plan(x_front=5.5, x_back=2.0, half_width=0.5, peak=0.42, tail=0.75)
CANOPY_TAIL = [CANOPY_GLASS[-1], (1.6, 0.28), (1.2, 0.03)]
# The chin intake's dark mouth inside its lips on the skin's face
# (`aircraft_skin.intake_mouth` corners, y and z: one mouth across the
# centreline).
MOUTH = ((-0.52, 1.42), (0.52, 1.42), (0.52, 1.88), (-0.52, 1.88))
# The two EJ200 nozzles in place of the skin's, the skin cut inside them.
NOZZLES = [dict(loc=(-6.4, y, 1.8), r_front=0.45, r_exit=0.42, length=0.45, petals=12) for y in (0.44, -0.44)]
# One box over both: between the pipes the skin's own tail would be left in shreds.
NOZZLE_CUT = [((-7.4, -6.5), (-0.85, 0.85), (1.39, 2.21))]
SPEC = dict(
    # The skin: its wings out from the body, its canards ahead of them
    # under the cockpit, its fin above the spine.
    skin=dict(path=SKIN, length=15.96, top=5.28, span=SPAN, cut=NOZZLE_CUT,
              canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(4.9,)),
              parts=[("wing_L", ((-6.5, 1.5), (1.0, 6.0), (1.4, 2.4))),
                     ("wing_R", ((-6.5, 1.5), (-6.0, -1.0), (1.4, 2.4))),
                     ("wing_canard_L", ((3.8, 6.0), (0.75, 2.5), (2.1, 2.8))),
                     ("wing_canard_R", ((3.8, 6.0), (-2.5, -0.75), (2.1, 2.8))),
                     ("tail_fin_0", ((-8.5, -3.5), (-0.3, 0.3), (3.05, 9.0)))]),
    nozzles=NOZZLES,
)
# Wheelbase and track as the photos give them; each leg's top on the skin.
GEAR = [dict(name="nose", x=2.7, y=0.0, radius=0.26, width=0.16, door=(0.7, 0.4), light=True),
        dict(name="main_L", x=-1.5, y=1.2, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
        dict(name="main_R", x=-1.5, y=-1.2, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)]
# The wing pylons (x, y, length, depth) from the skin's underside and what
# hangs under each (kind, x offset, length, radius): tanks inboard, AIM-120s
# outboard; the tips' defensive-aids pods; the semi-recessed missiles under
# the body.
PYLONS = [((-1.7, 2.0, 1.1, 0.3), ("tank", -0.1, 4.0, 0.36)), ((-2.5, 3.3, 1.0, 0.24), ("missile", 0.0, 3.65, 0.09))]
# The tip's chord is measured at `TIP_X`; its pod reaches forward of it.
TIP_X, POD_X, TIP_POD = -5.1, -4.4, (1.6, 0.12)
BODY_MISSILES = [(0.4, 0.55), (-2.6, 0.55)]
# The RAF's low-visibility roundels and fin flash, as the side photo.
MARKINGS = [
    ("insignia", dict(kind="uk_lowvis", centre=(1.4, 1.2, 2.6), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("fuselage",))),
    ("insignia", dict(kind="uk_flash", centre=(-5.9, 0.3, 3.45), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("tail_fin_0",))),
    ("insignia", dict(kind="uk_lowvis", centre=(-3.6, 3.4, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.45,
                      onto=("wing_",))),
    ("text", dict(text="ZK353", height=0.2, centre=(-4.2, 1.2, 2.3), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("fuselage",))),
]


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
        mid = (shell.under(TIP_X, side * (tip - 0.1)) + shell.over(TIP_X, side * (tip - 0.1))) / 2
        A.store(f"store_tip_{s}", "pod", (POD_X, side * (tip + 0.08), mid), *TIP_POD, m, hull)
    for k, (x, y) in enumerate(BODY_MISSILES):
        A.mirrored(lambda side, s, x=x, y=y, k=k: A.store(f"store_body_{k}_{s}", "missile",
                                                           (x, side * y, shell.under(x, side * y) - 0.07), 3.65, 0.09,
                                                           m, hull, rail=False))
    fin = Probe(hull, ("tail_fin",))
    A.nav_lights(m, hull, tail=(-7.5, 0.0, fin.over(-7.5, 0.0) - 0.1),
                 **{k: (TIP_X, side * (tip - 0.25), shell.over(TIP_X, side * (tip - 0.25)) + 0.01)
                    for k, side in (("left", 1), ("right", -1))})
    intake_mouth("intake_mouth", shell, m["black"], hull, 1, MOUTH)
    A.blade_antenna("antenna_spine", (-1.2, 0, shell.over(-1.2, 0)), 0.22, m, hull)
    A.markings(v, MARKINGS)


def wreck(variant, v):
    import bpy
    A.crash(v, tail_x=-3.6, wing_y=3.2, wing_side=1, tail_yaw=0.28, seed=3)
    # The crash lays its litter by the airframe's span; the unit type's
    # frame is a little wider than the wreck's box allows, so the far
    # plate lies a little nearer (it sat 7 cm past the box).
    for o in bpy.data.objects:
        if o.name.startswith("litter_1_") and o.parent == v.root:
            o.location.y *= 0.95


run("eurofighter_typhoon", "nato_air_grey", build, wreck, script="typhoon.py")
