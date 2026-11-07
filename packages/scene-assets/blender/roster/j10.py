"""J-10C, from assets/references/j_10_vigorous_dragon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/j10.py -- [--wreck]

What the photos settle: a single-engine canard delta. The pointed nose and
bubble canopy, the chin intake under the cockpit (the C's diverterless bump,
where the A had a splitter plate), canards just behind the cockpit, the delta
wing, one tall fin, two ventral fins canted outward under the tail, one
nozzle. Light grey.

The references' dimensions (published figures): length 16.9 m,
span 9.75 m, height 5.43 m.

Its shell is a generated skin (`aircraft_skin`, pinned in `skins/`):
Hunyuan3D's model of the airframe alone from our own renders of its guide
(`guides/j10.py`), the lofted J-10C measured from a three-view (both sides,
top and bottom). The guide is 16.35 m nose to tail; the frame's 16.82 m
runs on to the pitot's tip, so the skin stands with its tail at the
frame's and everything measured on the guide sits `GUIDE_X` aft of it. It
stood 0.09 m high (`lower`) and its wings 0.27 m long a side (`span`); the
canopy is fitted to its hump from the guide's plan. The chin intake's dark
mouth, the nozzle (the skin's own cut away inside it), the pitot, the gear,
stores and antennas are measured on it or seated on it (`Probe`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from vehicle_export import run  # noqa: E402

SKIN = "assets/source/skins/j_10_vigorous_dragon/eastern_j_10_vigorous_dragon_j_10c.glb"
# The guide's frame to this one: its x less this (its nose 0.475 m aft of
# the frame's, the pitot's length).
GUIDE_X = -0.475
WING_Z = 1.62
# The canopy's plan, the guide's: the windscreen's foot at 6.1, the glass
# ending on the spine at 3.4, its opaque tail faired in by 2.4; a bow at 5.5.
CANOPY_GLASS = framed_plan(x_front=6.1 + GUIDE_X, x_back=3.4 + GUIDE_X, half_width=0.46, peak=0.57, tail=0.75)
CANOPY_TAIL = [CANOPY_GLASS[-1], (3.0 + GUIDE_X, 0.2), (2.7 + GUIDE_X, 0.1), (2.4 + GUIDE_X, 0.02)]
# The chin intake's dark mouth on the skin's face (1.2 m wide, 1.1 to 1.55 m up), inside its lips.
MOUTH = ((-0.55, 1.13), (0.55, 1.13), (0.55, 1.51), (-0.55, 1.51))
# The nozzle in place of the skin's open pipe (cut inside its radius).
NOZZLE = dict(loc=(-6.6 + GUIDE_X, 0.0, 2.05), r_front=0.62, r_exit=0.55, length=1.3, petals=14)
NOZZLE_CUT = ((-8.6, -6.7 + GUIDE_X), (-0.6, 0.6), (1.45, 2.65))
# The ventral fins under the tail, canted out (thin: the skin's are too coarse).
VENTRAL = dict(root_x=-4.8 + GUIDE_X, height=0.55, root_chord=1.6, tip_chord=0.9, sweep_m=0.5, rudder=False)
SPEC = dict(
    # The skin's delta and canards (`wing_canard_*`, kept from the wreck's
    # fold) and its fin split off by name.
    skin=dict(path=SKIN, length=16.35, top=5.43, x=(8.45 - 7.9) / 2 + GUIDE_X, lower=(0.09, 3.4), span=(1.0, 4.87),
              cut=[NOZZLE_CUT], canopy=dict(glass=CANOPY_GLASS, tail=CANOPY_TAIL, bows=(5.5 + GUIDE_X,)),
              parts=[("wing_canard_L", ((1.3 + GUIDE_X, 3.9 + GUIDE_X), (0.75, 2.5), (1.9, 2.45))),
                     ("wing_canard_R", ((1.3 + GUIDE_X, 3.9 + GUIDE_X), (-2.5, -0.75), (1.9, 2.45))),
                     ("wing_L", ((-8.6, 1.3 + GUIDE_X), (0.95, 6.0), (1.25, 1.95))),
                     ("wing_R", ((-8.6, 1.3 + GUIDE_X), (-6.0, -0.95), (1.25, 1.95))),
                     ("tail_fin_0", ((-8.6, -2.4 + GUIDE_X), (-0.35, 0.35), (2.62, 9.0)))]),
    nozzles=[NOZZLE],
    # The pitot from the skin's nose to the frame's.
    nose_probe=((8.45 + GUIDE_X, 0.0, 1.8), 0.5),
)
# Gear (x, y, radius, width, extras), each leg's top seated under the skin.
GEAR = [("nose", 4.2 + GUIDE_X, 0.0, 0.26, 0.15, dict(door=(0.7, 0.4), light=True)),
        ("main_L", -1.2 + GUIDE_X, 1.3, 0.38, 0.2, dict(door=(1.0, 0.55), rake=0.1)),
        ("main_R", -1.2 + GUIDE_X, -1.3, 0.38, 0.2, dict(door=(1.0, 0.55), rake=0.1))]
# The wing pylons (x, y, length, depth) and what hangs under each: the tank
# inboard, a PL-15 outboard; a PL-10 on the outer rail.
PYLONS = [((-1.0 + GUIDE_X, 2.2, 1.1, 0.3), ("tank", -0.1, 4.0, 0.36)),
          ((-1.8 + GUIDE_X, 3.3, 0.9, 0.24), ("missile", 0.0, 3.7, 0.1)),
          ((-3.0 + GUIDE_X, 4.3, 0.7, 0.12), ("missile", 0.0, 2.9, 0.08))]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    shell = Probe(hull)
    for k, side in ((1, 1), (2, -1)):
        x = VENTRAL["root_x"] - VENTRAL["root_chord"] / 2
        A.fin(f"tail_fin_{k}", root_z=shell.under(x, side * 0.6) + 0.04, y=side * 0.6, cant=side * 2.7, mats=m,
              parent=hull, **VENTRAL)
    for name, x, y, radius, width, extra in GEAR:
        A.gear(name=name, x=x, y=y, top=shell.under(x, y) + 0.02, radius=radius, width=width, mats=m, parent=hull,
               **extra)
    for k, ((x, y, length, depth), (kind, dx, s_len, radius)) in enumerate(PYLONS):
        for side in (1, -1):
            top = shell.under(x, side * y)
            A.pylon(f"pylon_{k}_{side + 1}", (x, side * y, top), length, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", kind, (x + dx, side * y, top - depth - radius - 0.02), s_len, radius, m,
                    hull, rail=kind == "missile")
    intake_mouth("intake_mouth", shell, m["black"], hull, 1, MOUTH)
    # The wing-tip lights mid-chord on the delta's narrow tips.
    chord = shell.chord(4.6, 0.0, -8.45)
    tip_x = (max(chord) + min(chord)) / 2
    fin = Probe(hull, ("tail_fin_0",))
    A.nav_lights(m, hull, tail=(-7.4 + GUIDE_X, 0.0, fin.over(-7.35 + GUIDE_X, 0.0) - 0.35),
                 **{k: (tip_x, side * 4.6, shell.over(tip_x, side * 4.6) + 0.01) for k, side in (("left", 1), ("right", -1))})
    A.blade_antenna("antenna_spine", (0.2, 0, shell.over(0.2, 0)), 0.22, m, hull)
    # The PLA's red star and bars aft on the body and on the wings, the
    # photo's red 05 on the nose and the fin.
    A.markings(v, [
        ("insignia", dict(kind="cn_star", centre=(-3.9 + GUIDE_X, 1.2, 2.15), normal=(0, 1, 0), up=(0, 0, 1), size=0.6,
                          onto=("fuselage",))),
        ("insignia", dict(kind="cn_star", centre=(-1.8 + GUIDE_X, 3.0, WING_Z + 0.3), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_",))),
        ("text", dict(text="05", height=0.42, centre=(6.6 + GUIDE_X, 1.0, 2.1), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="red")),
        ("text", dict(text="05", height=0.4, centre=(-6.6 + GUIDE_X, 0.3, 4.4), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="red")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=10)


run("j_10_vigorous_dragon", "chinese_air_light_grey", build, wreck, script="j10.py")
