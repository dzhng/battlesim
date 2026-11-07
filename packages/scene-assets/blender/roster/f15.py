"""F-15E Strike Eagle and F-15EX Eagle II, from assets/references/f_15_eagle/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f15.py -- [--variant=<card>] [--wreck]

What the photos settle: the slab-sided body between two big box intakes
raked forward at the top, the shoulder wing cropped square at the tips, twin
upright fins on the tail booms, stabilators below them, two engines' nozzles
between the booms with the stinger between them; the two-seat canopy (the
EX keeps the E's), conformal fuel tanks along the intakes' flanks below the
wing, tall gear legs. The Strike Eagle wears gunship grey, the EX two-tone
compass grey; the E carries bombs on its CFTs, the EX missiles.

The references' dimensions (USAF and Boeing figures agree):
length 19.43 m, span 13.05 m, height 5.63 m.

Each variant's shell is a generated skin (`aircraft_skin`, pinned in
`skins/`): Hunyuan3D from our own shaded views of the lofted F-15 its
roster script drew before, so it keeps that model's measured outline as one
smooth surface. The E's skin stands 0.1 m high on it and the EX's 0.28 m
(their fins came out short): both are lowered, the fins' tips held, and
their wings brought in to the 13.05 m span. The canopy is fitted to each
skin's hump; the intakes' mouths, the nozzles, the gear, the pylons and
every store are seated on the skin where it is (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

E, EX = "us_f_15_eagle_f_15e_strike_eagle", "us_f_15_eagle_f_15ex_eagle_ii"
SCHEME = {E: "us_gunship_grey", EX: "us_compass_grey"}

# The wing's mean height on the skins, for the insignia laid on it.
WING_Z = 2.45
SKINS = "assets/source/skins/f_15_eagle"
# Each skin's corrections on the lofted model's outline, and the y of its fins.
FIT = {E: dict(lower=(0.1, 4.2), fin_y=1.06), EX: dict(lower=(0.28, 4.3), fin_y=1.2)}
# The two-seat canopy's plan (the framed canopy's outline the lofted model
# drew), its tail fairing onto the spine, and its bows: the windscreen's
# arch and the bow between the seats.
GLASS = framed_plan(6.3, 2.15, 0.56, peak=0.42)
CANOPY = dict(glass=GLASS, tail=[GLASS[-1], (1.7, 0.3), (1.25, 0.03)], bows=(5.5, 3.8))
# The box intakes' mouths within the lips of each skin's raked intake face
# (`aircraft_skin.intake_mouth` corners, y and z), measured from ahead.
MOUTH = {E: ((0.95, 1.45), (1.55, 1.45), (1.55, 2.3), (0.95, 2.3)),
         EX: ((0.85, 1.45), (1.45, 1.45), (1.45, 2.35), (0.85, 2.35))}
# The F110s' nozzles in place of the skin's, which is cut inside their radius.
NOZZLES = [dict(loc=(-8.6, y, 2.0), r_front=0.58, r_exit=0.52, length=0.82, petals=16) for y in (0.64, -0.64)]
NOZZLE_CUT = [((-10.0, -8.65), (y - 0.5, y + 0.5), (1.5, 2.5)) for y in (0.64, -0.64)]
# Wheelbase and track as the photos give them; each leg's top on the skin.
GEAR = [dict(name="nose", x=3.3, y=0.0, radius=0.33, width=0.19, door=(0.9, 0.5), light=True),
        dict(name="main_L", x=-2.3, y=1.38, radius=0.52, width=0.26, door=(1.3, 0.6), rake=0.08),
        dict(name="main_R", x=-2.3, y=-1.38, radius=0.52, width=0.26, door=(1.3, 0.6), rake=0.08)]


def spec(variant):
    fit = FIT[variant["id"]]
    fin_box = [((-10.0, -6.0), tuple(sorted((s * (fit["fin_y"] - 0.5), s * (fit["fin_y"] + 0.6)))), (3.0, 9.0))
               for s in (1, -1)]
    return dict(
        skin=dict(path=f"{SKINS}/{variant['id']}.glb", length=19.43, top=5.63, lower=fit["lower"], span=(2.0, 6.52),
                  canopy=CANOPY, cut=NOZZLE_CUT,
                  parts=[("wing_L", ((-5.2, 3.5), (2.3, 8.0), (2.05, 3.0))),
                         ("wing_R", ((-5.2, 3.5), (-8.0, -2.3), (2.05, 3.0))),
                         ("tail_fin_0", fin_box[0]), ("tail_fin_1", fin_box[1])]),
        nozzles=NOZZLES,
    )


def build(variant, v):
    e = variant["id"] == E
    m = A.jet(v, spec(variant))
    hull = v.hull
    shell = Probe(hull)
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH[variant["id"]]))
    for g in GEAR:
        A.gear(top=shell.under(g["x"], g["y"]) + 0.02, mats=m, parent=hull, **g)
    A.nav_lights(m, hull, tail=(-9.0, 0.0, 2.05),
                 **{k: (-3.2, side * 6.3, shell.over(-3.2, side * 6.3) + 0.01) for k, side in (("left", 1), ("right", -1))})
    # A 610-gallon tank on each wing's pylon, an AIM-9 on the pylon's
    # outboard rail, and one tank on the centreline under the belly.
    for side, k in ((1, "L"), (-1, "R")):
        top = shell.under(-0.3, side * 3.1)
        A.pylon(f"pylon_wing_{k}", (-0.3, side * 3.1, top), 1.3, 0.36, m, hull)
        A.store(f"store_tank_{k}", "tank", (-0.4, side * 3.1, top - 0.36 - 0.46 - 0.02), 5.0, 0.46, m, hull)
        A.store(f"store_aim9_{k}", "missile", (-0.3, side * 3.55, top - 0.2), 2.9, 0.08, m, hull)
    A.store("store_tank_c", "tank", (-0.6, 0.0, shell.under(-0.6, 0.0) - 0.46 + 0.01), 5.0, 0.46, m, hull)
    # The CFTs' stations: the E's GBU-31s in tandem on stub pylons and its
    # targeting pod under the intake; the EX's AIM-120s on their rails.
    for side, k in ((1, "L"), (-1, "R")):
        for j, x in enumerate((1.3, -1.6)):
            top = shell.under(x, side * 2.0)
            if e:
                A.pylon(f"pylon_cft_{k}{j}", (x, side * 2.0, top), 0.9, 0.1, m, hull)
                A.store(f"store_cft_{k}{j}", "bomb", (x, side * 2.0, top - 0.1 - 0.23 - 0.01), 3.0, 0.23, m, hull)
            else:
                A.store(f"store_cft_{k}{j}", "missile", (x + 0.1, side * 2.0, top - 0.16), 3.65, 0.09, m, hull)
    if e:
        A.store("store_pod", "pod", (3.0, 1.0, shell.under(3.0, 1.0) - 0.18 + 0.01), 2.4, 0.18, m, hull)
    # The spine's and the belly's blades.
    A.blade_antenna("antenna_spine", (-1.0, 0, shell.over(-1.0, 0)), 0.25, m, hull)
    A.blade_antenna("antenna_belly", (0.8, 0, shell.under(0.8, 0)), 0.2, m, hull, down=True)
    # Seymour Johnson's SJ in black on the E's gunship grey; Eglin's ET on the EX.
    code, serial, ink = ("SJ", "AF 88 1682", "black") if e else ("ET", "AF 20 0001", "lowvis_dark")
    fin_y = FIT[variant["id"]]["fin_y"]
    A.markings(v, [
        ("insignia", dict(kind="us_lowvis", centre=(0.4, 1.9, 2.05), normal=(0, 1, 0), up=(0, 0, 1), size=0.42,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.4, 4.6, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.6, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.4, -4.6, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.6, onto=("wing_R",), mirror=False)),
        ("text", dict(text=code, height=0.75, centre=(-8.3, fin_y + 0.3, 4.25), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_",), colour=ink)),
        ("text", dict(text=serial, height=0.2, centre=(-8.1, fin_y + 0.3, 3.5), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_",), colour=ink)),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-5.6, wing_y=3.8, wing_side=1, tail_yaw=0.25, seed=15)


run("f_15_eagle", SCHEME, build, wreck, script="f15.py")
