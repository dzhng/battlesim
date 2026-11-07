"""F-35A and F-35B Lightning II, from assets/references/f_35_lightning_ii/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f35.py -- [--variant=<card>] [--wreck]

What the photos settle: a short, deep, chined body, the canopy high on it
with one aft frame, caret intakes behind the cockpit, the trapezoid wing and
stabilators, twin fins canted outward, one big engine nozzle; the sensor
window under the nose. The A has the gun's bulge on the left shoulder and a
round nozzle; the B (short take-off, vertical landing) has the lift-fan doors
behind the canopy, the auxiliary inlet doors above the engine and the
swivelling three-bearing nozzle, no gun. Both in the dark grey radar
coating.

The references' dimensions (Lockheed Martin figures agree):
F-35A 15.7 x 10.7 x 4.38 m; F-35B 15.6 x 10.7 x 4.36 m.

Each variant's shell is a generated skin (`aircraft_skin`, pinned in
`skins/`): Hunyuan3D's model of the lofted model it replaces, from its side,
top and bottom renders, so its proportions are the measured ones (the A's
body set 0.08 m down, both spans let out to 10.72 m; the B's body stands a
little deeper than the lofted one). Its canopy is fitted to its hump, and
every part that sits on it is seated on it (`Probe`). The skin carries the
caret intakes and the A's gun fairing; its tail is a box, so the A's round
nozzle and the B's swivel duct are built round it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

FA, FB = "f_35a", "us_f_35_lightning_ii_f_35b"
# The caret intakes' mouths on each skin's raked faces, inside their lips
# (`aircraft_skin.intake_mouth` corners, y and z), measured from ahead.
MOUTH = {FA: ((1.12, 1.55), (1.48, 1.55), (1.58, 2.4), (1.12, 2.4)),
         FB: ((1.12, 1.45), (1.62, 1.45), (1.72, 2.42), (1.12, 2.42))}
SKINS = "assets/source/skins/f_35_lightning_ii"
# Length and height (to the fins' tips) of each, the skin's frame.
FRAME = {FA: (15.7, 4.38), FB: (15.6, 4.36)}

# The wing and stabilators lie on the chine line (the side photo: 2.2 m up).
WING_Z = 2.2
# The fins stand on the tail booms' tops, canted out as the front photo shows.
FIN_Y, FIN_Z, FIN_CANT = 1.32, 2.45, 0.38

# The canopy's plan (x, half width) front to back, and the short opaque
# frame behind it where the hump meets the spine; its heights are the skin's
# hump's (`aircraft_skin.fit_canopy`). The B's is shorter, ahead of its lift
# fan. One frame bow a quarter metre aft of the windscreen's foot.
CANOPY = {
    FA: ([(5.72, 0.06), (5.4, 0.36), (5.0, 0.46), (4.6, 0.5), (4.2, 0.5), (3.8, 0.49), (3.45, 0.46)],
         [(3.45, 0.46), (3.25, 0.34), (3.1, 0.12)]),
    FB: ([(5.85, 0.06), (5.4, 0.42), (5.0, 0.47), (4.6, 0.5), (4.2, 0.52), (3.8, 0.53), (3.5, 0.52)],
         [(3.5, 0.52), (3.35, 0.4), (3.2, 0.15)]),
}
# Each skin on the measured outline: the A's body a little high (down 0.08
# m below its canopy's top, its fins' tips held), both spans a little short.
FIT = {FA: dict(lower=(0.08, 3.2), span=(2.5, 5.36)), FB: dict(span=(2.5, 5.36))}
# The engine's axis on the skin's tail (its section there, 1.45 to 2.55 m
# up). The skin's own nozzle is a box: the built pipe (the A's round nozzle,
# the B's swivel duct) is drawn a little fuller than it, round its end, and
# the box goes only well inside the pipe, where its cut edge can't show.
ENGINE_Z = 2.0
NOZZLE = dict(loc=(-6.2, 0.0, ENGINE_Z), r_front=0.66, r_exit=0.57, length=0.95, petals=14)
SWIVEL = ((-5.9, 0.68, 0.45), (-6.35, 0.66, 0.45), (-6.8, 0.64, 0.45))
TAIL_CUT = {FA: ((-7.6, -6.3), (-0.6, 0.6), (ENGINE_Z - 0.56, ENGINE_Z + 0.56)),
            FB: ((-7.6, -6.4), (-0.63, 0.63), (ENGINE_Z - 0.6, ENGINE_Z + 0.6))}


def spec(variant):
    v = variant["id"]
    length, top = FRAME[v]
    glass, tail = CANOPY[v]
    return dict(
        # The skin: its wings out from the body's flanks, ahead of the
        # stabilators, and each canted fin above the booms, split off by name.
        skin=dict(path=f"{SKINS}/{v}.glb", length=length, top=top, **FIT[v],
                  canopy=dict(glass=glass, tail=tail, bows=(glass[0][0] - 0.25,)), cut=[TAIL_CUT[v]],
                  parts=[("tail_fin_0", ((-7.7, -3.2), (0.9, 2.7), (2.62, 9.0))),
                         ("tail_fin_1", ((-7.7, -3.2), (-2.7, -0.9), (2.62, 9.0))),
                         ("wing_L", ((-4.95, 2.5), (2.1, 6.0), (1.6, 2.7))),
                         ("wing_R", ((-4.95, 2.5), (-6.0, -2.1), (1.6, 2.7)))]),
        nozzles=[] if v == FB else [NOZZLE],
    )


def build(variant, v):
    b = variant["id"] == FB
    m = A.jet(v, spec(variant))
    hull = v.hull
    shell = Probe(hull)
    skin = Probe(hull, ("fuselage",)).over

    # The gear: the nose leg under the cockpit, the mains under the wing
    # roots 4.3 m apart (the front photo), each from the skin above it.
    A.gear(name="nose", x=3.75, y=0.0, top=shell.under(3.75, 0.0) + 0.02, radius=0.3, width=0.17, door=(0.9, 0.45),
           light=True, mats=m, parent=hull)
    A.mirrored(lambda s, k: A.gear(name=f"main_{k}", x=-2.5, y=s * 2.15, top=shell.under(-2.5, s * 2.15) + 0.02,
                                    radius=0.4, width=0.24, door=(1.2, 0.6), rake=0.1, mats=m, parent=hull))
    # The wing-tip navigation lights, on the tips' edges.
    A.nav_lights(m, hull, **{k: (-3.0, s * 5.15, (shell.under(-3.0, s * 5.15) + shell.over(-3.0, s * 5.15)) / 2)
                             for k, s in (("left", 1), ("right", -1))})
    # The electro-optical targeting window under the chin, the distributed
    # aperture windows on the nose's flanks.
    box("eots_window", (0.5, 0.36, 0.14), (6.4, 0, shell.under(6.4, 0) + 0.04), m["glass"], hull, rot=(0, 0.3, 0),
        lods=MID)
    A.mirrored(lambda s, k: box(f"das_window_{k}", (0.18, 0.03, 0.14),
                                (5.2, shell.flank(5.2, s, 2.0) - s * 0.01, 2.0),
                                m["glass"], hull, lods=NEAR))
    # Each caret intake's dark mouth within the lips of the skin's intake face.
    A.mirrored(lambda s, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, s, MOUTH[variant["id"]]))
    A.mirrored(lambda s, k: box(f"bay_seam_{k}", (3.4, 0.03, 0.03), (0.4, s * 0.5, shell.under(0.4, s * 0.5) - 0.005),
                                m["dark"], hull, lods=NEAR))
    # Eielson's AK (the side photo) on the A, the Marines' VK on the B, in
    # the low-visibility grey both wear.
    fin_n, fin_up = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT)), (0, math.sin(FIN_CANT), math.cos(FIN_CANT))

    def on_fin(x, up):
        return (x, FIN_Y + up * fin_up[1] + 0.3, FIN_Z + up * fin_up[2])
    rows = [
        ("insignia", dict(kind="us_lowvis", centre=(-5.0, 1.5, 1.9), normal=(0, 1, 0), up=(0, 0, 1), size=0.3,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.2, 3.8, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.45, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.2, -3.8, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.45, onto=("wing_R",), mirror=False)),
        ("text", dict(text="VK" if b else "AK", height=0.48, centre=on_fin(-5.6, 1.0),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",), colour="lowvis_light")),
        ("text", dict(text="169164" if b else "AF 19 5458", height=0.12,
                      centre=on_fin(-5.4, 0.45), normal=fin_n, up=fin_up,
                      onto=("tail_fin_",), colour="lowvis_light")),
    ]
    if b:
        rows.append(("text", dict(text="MARINES", height=0.24, centre=(-4.4, 1.5, 1.88), normal=(0, 1, 0),
                                  up=(0, 0, 1), onto=("fuselage",), colour="lowvis_light")))
    A.markings(v, rows)

    if b:
        # The lift-fan door behind the canopy, its outline a dark seam on
        # the hump it lies on; the auxiliary inlet doors aft of it, two dark
        # panels either side of the ridge; the three-bearing swivel nozzle,
        # straight aft.
        x0, x1, w = 3.1, 1.4, 0.5
        ring = ([(x0, w * (1 - 2 * t / 6)) for t in range(6)] + [(x0 - (x0 - x1) * t / 12, -w) for t in range(12)]
                + [(x1, -w + 2 * w * t / 6) for t in range(6)] + [(x1 + (x0 - x1) * t / 12, w) for t in range(12)])
        pts = [(x, y, skin(x, y) + 0.008) for x, y in ring]
        for k, (a, c) in enumerate(zip(pts, pts[1:] + pts[:1])):
            A.line(f"lift_fan_door_seam_{k}", a, c, m["dark"], hull, width=0.07, height=0.03, lods=MID)

        def aux(s, k):
            x, y = 0.75, s * 0.2
            roll = math.atan2(skin(x, y + 0.15) - skin(x, y - 0.15), 0.3)
            pitch = math.atan2(skin(x + 0.45, y) - skin(x - 0.45, y), 0.9)
            box(f"aux_inlet_{k}", (0.9, 0.28, 0.03), (x, y, skin(x, y) + 0.01), m["dark"], hull,
                rot=(roll, -pitch, 0), lods=MID)
        A.mirrored(aux)
        for k, (x, r, l) in enumerate(SWIVEL):
            cyl(f"swivel_duct_{k}", r, l, (x - l / 2, 0, ENGINE_Z), "X", m["nozzle"], hull, seg=22, r2=r * 1.03)
            cyl(f"swivel_seam_{k}", r * 1.05, 0.03, (x - l, 0, ENGINE_Z), "X", m["dark"], hull, seg=22, lods=NEAR)
        cyl("swivel_bore", 0.56, 0.04, (-7.23, 0, ENGINE_Z), "X", m["black"], hull, seg=18, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-3.8, wing_y=3.2, wing_side=-1, tail_yaw=-0.25, seed=35)


run("f_35_lightning_ii", "us_gunship_grey", build, wreck, script="f35.py")
