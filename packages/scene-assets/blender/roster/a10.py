"""A-10C Thunderbolt II, from assets/references/a_10_warthog/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/a10.py -- [--wreck]

What the photos settle: the blunt nose with the GAU-8's seven-barrel muzzle
under it and the nose gear offset to the right, the high bubble canopy on an
armoured tub, the thick straight low wing with the main gear half-retracted
into pods under it and the tips drooped, two engines in round nacelles on
pylons high on the rear fuselage, the tailplane with twin fins at its ends;
eleven pylons. Two-tone grey.

The references' dimensions (USAF fact sheet figures agree):
length 16.26 m, span 17.53 m, height 4.47 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D from our own shaded views of the guide (`guides/a10.py`, the
lofted A-10 measured against a three-view: the wing, gear pods, tailplane
and fins 0.25-0.6 m aft or outboard of where it had them). It left out
the twin fins, so they are built here, and it stands with its engine nacelles' tops where the lofted model's were
(`top`). The canopy is fitted to its hump; the gun, the engines' fan faces
and exhausts, the gear, the pylons and the stores sit on the skin (`Probe`).
"""
import math
import os
import sys


sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402


# The low wing's height, for the insignia laid on it.
WING_Z = 1.6
# The twin fins at the tailplane's ends, as the lofted model drew them.
FIN = dict(root_x=-6.08, root_z=1.5, height=2.97, root_chord=2.0, tip_chord=1.45, sweep_m=0.45)
# The canopy's plan (the lofted model's framed outline), its tail onto the spine.
GLASS = framed_plan(6.1, 3.3, 0.56, peak=0.5, tail=0.6)
# The engines' centres (y, z) on their pylons over the rear fuselage.
ENGINE_Y, ENGINE_Z, FAN_R = 1.5, 3.28, 0.52
# The frame's 16.46 m runs from the gun's muzzle, which stands this far
# proud of the nose, to the tail: the skin is the rest, its tail held.
GUN_PROUD = 0.44
SPEC = dict(
    skin=dict(path="assets/source/skins/a_10_warthog/us_a_10_warthog_a_10c_retained_as_an_iconic_exception_if_needed.glb",
              length=16.46 - GUN_PROUD, x=-GUN_PROUD / 2, top=ENGINE_Z + 0.68, span=(2.0, 8.77),
              canopy=dict(glass=GLASS, tail=[GLASS[-1], (2.8, 0.3), (2.4, 0.03)], bows=(5.6,)),
              parts=[("wing_L", ((-2.5, 3.5), (1.6, 9.5), (1.2, 2.4))),
                     ("wing_R", ((-2.5, 3.5), (-9.5, -1.6), (1.2, 2.4)))]),
    fins=[dict(FIN, y=2.86), dict(FIN, y=-2.86)],
)
# The nose leg offset to the right, the mains in their pods; each leg's top on the skin.
GEAR = [dict(name="nose", x=5.0, y=-0.46, radius=0.33, width=0.2, door=(0.8, 0.4), light=True),
        dict(name="main_L", x=-0.3, y=2.1, radius=0.5, width=0.3),
        dict(name="main_R", x=-0.3, y=-2.1, radius=0.5, width=0.3)]
# The wing pylons (x, y, length, depth) on the skin's chord, a little aft
# of the lofted wing's, and what hangs under each.
STATIONS = [((0.3, 3.4, 1.2, 0.26), ("missile", -0.1, 2.5, 0.15)), ((0.2, 4.6, 1.1, 0.24), ("bomb", -0.1, 2.3, 0.2)),
            ((0.0, 5.8, 1.0, 0.22), ("pod", -0.1, 2.4, 0.15))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(STATIONS):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull)
    A.nav_lights(m, hull, **{k: (-0.2, side * 8.6, shell.over(-0.2, side * 8.6) + 0.01)
                             for k, side in (("left", 1), ("right", -1))})
    # The GAU-8 under the nose: its muzzle fairing and seven barrels.
    nose_x = shell.along(0.06, 1.85, True)
    cyl("gun_fairing", 0.24, 0.5, (nose_x - 0.2, 0.06, 1.85), "X", m["paint"], hull, seg=16, bevel=0.02)
    for k in range(7):
        a = math.tau * k / 7
        cyl(f"gun_barrel_{k}", 0.035, 0.5, (nose_x + 0.2, 0.06 + math.cos(a) * 0.12, 1.85 + math.sin(a) * 0.12), "X",
            m["steel"], hull, seg=8, lods=MID if k % 2 == 0 else NEAR)
    cyl("gun_muzzle", 0.17, 0.04, (nose_x + 0.44, 0.06, 1.85), "X", m["black"], hull, seg=14, lods=MID)
    # The engines are the skin's nacelles: each fan's face and exhaust, dark, just inside their lips.
    for side, s_ in ((1, "L"), (-1, "R")):
        y = side * ENGINE_Y
        cyl(f"nacelle_engine_{s_}_fan", FAN_R, 0.04, (shell.along(y, ENGINE_Z, True) - 0.03, y, ENGINE_Z), "X",
            m["dark"], hull, seg=20, lods=MID)
        lip = shell.along(y, ENGINE_Z + 0.4, False)
        cyl(f"nacelle_engine_{s_}_exhaust", 0.44, 0.05, (lip + 0.06, y, ENGINE_Z), "X", m["black"], hull, seg=16,
            lods=MID)
    box("refuel_door", (0.5, 0.3, 0.03), (7.0, 0, shell.over(7.0, 0)), m["dark"], hull, rot=(0, 0.35, 0), lods=NEAR)
    A.blade_antenna("antenna_spine", (0.8, 0, shell.over(0.8, 0)), 0.2, m, hull)
    # Moody's FT on the fins in black, the low-visibility insignia behind the
    # cockpit and on the wings, the serial on the nose's flank.
    mouth_y, serial_y = shell.flank(6.8, 1, 1.98), shell.flank(4.2, 1, 2.3)
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
        # The 23rd Wing's Flying Tiger shark mouth under the nose (the photo),
        # sized to the skin's nose (its side 1.7 to 2.3 m up at x 6.9).
        ("insignia", dict(kind="shark_mouth", centre=(6.8, mouth_y, 1.98), normal=(0, 1, 0), up=(0, 0, 1), size=0.6,
                          onto=("fuselage",), ahead=(1, 0, 0))),
        ("text", dict(text="0223", height=0.18, centre=(4.2, serial_y, 2.3), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-4.9, wing_y=5.0, wing_side=-1, tail_yaw=0.32, seed=10)


run("a_10_warthog", "us_gunship_grey", build, wreck, script="a10.py")
