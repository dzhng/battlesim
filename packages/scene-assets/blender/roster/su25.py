"""Su-25SM3, from assets/references/su_25_frogfoot/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/su25.py -- [--wreck]

What the photos settle: a slim pointed nose and the armoured cockpit with
its small canopy, two engines in nacelles hugging the fuselage at the
roots of a high straight wing, round-cornered intakes ahead of them, a
tall fin, tailplane with dihedral, short nozzles ending at the trailing
edge; the nose wheel offset to the left, rough-field tyres; eight pylons
with rocket pods and bombs, the SM3's L-370 jamming pods on the wing tips.
Green and brown camouflage.

The references' dimensions (Sukhoi figures): length 15.53 m,
span 14.36 m, height 4.8 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the airframe alone from our own renders of the lofted
Su-25 this script drew before, both sides, top and bottom, centred on the
hit box. It matches the lofted body to a few centimetres; its wings reach 0.45 m too far, brought
in to the frame's span (`span`), and its fin came out a rectangle, so the
lofted swept fin is built in its place. The canopy is fitted to its small
hump; the intakes' dark mouths, the short nozzles in place of the
nacelles' flat ends, the gear and the stations are measured on the skin
(the tables below) or seated on it (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/su_25_frogfoot/eastern_su_25_frogfoot_su_25sm3.glb"
WING_Z = 2.25
# The skin's frame: its length to the probe's tip, centred on the hit box
# (the stations below are on it), its wings in to the frame's span.
LENGTH, SPAN = 16.22, (2.0, 7.0)
# The canopy over the skin's small hump: the windscreen's foot at 5.0 m,
# the glass's peak a third of the way back, running on over the trough
# behind the hump to 3.6 (the lofted canopy's length); its armoured
# windscreen's frame.
CANOPY = dict(glass=framed_plan(x_front=5.0, x_back=3.6, half_width=0.42, peak=0.35),
              tail=[(3.35, 0.18), (3.1, 0.03)], bows=(4.7,))
CANOPY["tail"] = [CANOPY["glass"][-1]] + CANOPY["tail"]
# Each intake's dark mouth on its raked face (`intake_mouth` corners, y and
# z); the nacelles' flat ends (x, centre y, z, radius), where the short
# nozzles go.
MOUTH = ((0.9, 1.55), (1.45, 1.55), (1.45, 2.3), (0.9, 2.3))
ENGINE = (-4.79, 1.12, 1.94, 0.4)
EX, EY, EZ, ER = ENGINE
SPEC = dict(
    skin=dict(path=SKIN, length=LENGTH, top=4.8, span=SPAN, canopy=CANOPY,
              # The skin's fin (a rectangle) pressed into the spine under
              # the built one; the nacelles' ends inside the nozzles go.
              # The trough behind its hump is raised within the glass's
              # width, so the canopy fitted to it is one bubble back to 3.6.
              press=[((-8.8, -3.9), (-0.4, 0.4), (2.6, 9.0)), (((3.55, 4.05), (-0.42, 0.42), (2.3, 3.0)), 3.0)],
              cut=[((EX - 0.8, EX + 0.12), tuple(sorted((s * (EY - ER), s * (EY + ER)))), (EZ - ER, EZ + ER))
                   for s in (1, -1)],
              parts=[("wing_L", ((-3.2, 3.2), (1.9, 8.0), (1.6, 2.9))),
                     ("wing_R", ((-3.2, 3.2), (-8.0, -1.9), (1.6, 2.9)))]),
    wing_hinge=((0.75, 0, 1), 0.1),
    fins=[dict(root_x=-4.49, root_z=2.3, height=2.5, root_chord=2.6, tip_chord=0.9, sweep_m=1.7)],
    nozzles=[dict(loc=(EX + 0.1, s * EY, EZ), r_front=ER + 0.02, r_exit=ER - 0.02, length=0.6, petals=0)
             for s in (1, -1)],
)
# Gear (x, y, radius, width, extras), each leg's top under the skin: the nose
# wheel offset to the left.
GEAR = [("nose", 4.71, 0.25, 0.33, 0.2, dict(door=(0.8, 0.4), light=True)),
        ("main_L", -0.09, 1.25, 0.42, 0.26, {}), ("main_R", -0.09, -1.25, 0.42, 0.26, {})]
# The wing's stations (y, length, depth) and what hangs there, each pylon a
# little ahead of the middle of the skin's chord there.
STATIONS = [((2.4, 1.0, 0.3), ("bomb", 2.4, 0.2)), ((3.3, 1.0, 0.28), ("pod", 2.7, 0.26)),
            ((4.2, 0.9, 0.26), ("pod", 2.7, 0.26)), ((5.1, 0.9, 0.24), ("missile", 2.9, 0.1))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    for k, ((y, length, depth), (kind, s_len, radius)) in enumerate(STATIONS):
        x = sum(shell.chord(y, 4.0, -3.9)) / 2 + 0.15
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x - 0.1, side * y, top - depth - radius - 0.02), s_len, radius,
                    m, hull)
    # The SM3's jamming pods on the wing tips, at the tips' mid-thickness;
    # the navigation lights inboard of them.
    tip = SPAN[1] - 0.05
    tx = sum(shell.chord(tip - 0.1, 4.0, -3.9)) / 2
    for side, k in ((1, "L"), (-1, "R")):
        mid = (shell.under(tx, side * (tip - 0.1)) + shell.over(tx, side * (tip - 0.1))) / 2
        A.store(f"store_tip_{k}", "pod", (tx, side * (tip + 0.12), mid), 2.0, 0.2, m, hull)
    nx = sum(shell.chord(tip - 0.4, 4.0, -3.9)) / 2
    A.nav_lights(m, hull, left=(nx, tip - 0.4, shell.over(nx, tip - 0.4) + 0.01),
                 right=(nx, -tip + 0.4, shell.over(nx, -tip + 0.4) + 0.01),
                 tail=(-LENGTH / 2 + 0.4, 0.0, shell.over(-LENGTH / 2 + 0.4, 0.0) + 0.02))
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, MOUTH))
    # The photo's markings: the red star, VKS ROSSII and the serial on the
    # fin, red stars under the wings, the red 10 on the nose.
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-6.29, 0.4, 4.0), normal=(0, 1, 0), up=(0, 0, 1), size=0.36,
                          onto=("tail_fin_0",))),
        ("text", dict(text="ВКС РОССИИ", height=0.16, centre=(-5.99, 0.4, 3.45), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="white")),
        ("text", dict(text="RF-95120", height=0.12, centre=(-6.19, 0.4, 3.7), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="white")),
        *[("insignia", dict(kind="ru_star", centre=(-0.29, y, WING_Z - 0.5), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.6, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)],
        ("text", dict(text="10", height=0.5, centre=(4.51, 1.0, 2.05), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="red")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.6, wing_y=4.6, wing_side=-1, tail_yaw=-0.32, seed=25, litter_reach=0.95)


run("su_25_frogfoot", "russian_helicopter_camo", build, wreck, script="su25.py")
