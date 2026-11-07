"""Su-27SM3, Su-30SM2, Su-35S and Su-34, from assets/references/sukhoi_flanker_family/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/flanker.py -- [--variant=<card>] [--wreck]

What the photos settle: one airframe family. A long drooping nose and a
bubble canopy (the Su-30's two-seat), the wide flat centre body blending into
the wing through long leading-edge extensions, two engine nacelles slung
apart under it with a tunnel between, their rectangular intakes raked under
the extensions, two upright fins on the nacelles, stabilators, ventral fins,
and the long tail stinger between the nozzles; tall gear with twin nose
wheels. The Su-30SM2 adds canards; the Su-35S drops the spine air brake and
carries wing-tip jamming pods; the Su-34 has the side-by-side cockpit under a
broad flattened nose, canards, tandem main wheels and a longer stinger. Flanker
blue-greys.

The references' dimensions (Sukhoi figures): Su-27SM3 21.9 x 14.7
x 5.92 m; Su-30SM2 21.935 x 14.7 x 6.36 m; Su-35S 21.9 x 15.3 x 5.9 m;
Su-34 23.34 x 14.7 x 6.09 m.

Each airframe's shell is a generated skin (`aircraft_skin`, pinned in
`skins/`): Hunyuan3D's model of it alone from our own renders of the lofted
airframe this script drew before, both sides, top and bottom. Their fins
came out short, standing the bodies high with the fins' tips at the frame's
height: each is lowered (`lower`) back onto the lofted body's line, and the
two-seaters' and the Su-35's wings brought in or out to the frame's span
(`span`). The canopy is fitted to each skin's own hump from the plan
measured on it (the Su-34's wide side-by-side glass); the intakes' dark
mouths, the nozzles in place of the nacelles' flat ends, the gear, the
stations and the sensors are measured on the skins (the tables below) or
seated on them (`Probe`). The canards and the ventral fins the skins left
out are built.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_canopy import framed_plan  # noqa: E402
from aircraft_skin import Probe, intake_mouth  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

SU27, SU30, SU35, SU34 = ("eastern_sukhoi_flanker_family_su_27sm3", "eastern_sukhoi_flanker_family_su_30sm2",
                          "eastern_sukhoi_flanker_family_su_35s", "eastern_sukhoi_flanker_family_su_34")
DIMENSIONS = {SU27: (21.9, 14.7, 5.92), SU30: (21.935, 14.7, 6.36), SU35: (21.9, 15.3, 5.9), SU34: (23.34, 14.7, 6.09)}
SKINS = "assets/source/skins/sukhoi_flanker_family"

WING_Z = 2.55
# The tail booms outboard of the nacelles (over the main wheels), where the
# fins and the ventral fins stand.
BOOM_Y, FIN_Z = 2.3, 2.5
# Each skin, measured on it once lowered (`skin_register`, the hump's top
# along the centreline, casts from ahead and behind): its length to the
# probe's tip, `lower` and `span`; the canopy's plan (the framed canopy's
# outline from the windscreen's foot to where the glass meets the spine,
# highest at `peak` of the way back) and its opaque tail onto the spine, its
# bows; each intake's dark mouth (`intake_mouth` corners); the nacelles'
# flat ends (x, centre y, z, radius), where the nozzles go; the wing tip;
# the infrared sensor's ball (x, y) ahead of the windscreen, to starboard
# on the skin's own, and the lumps the skin drew there (the old ball,
# mirrored by its symmetry: x, |y| and the nose's height there, to which
# they are pressed).
FIT = {
    SU27: dict(length=21.8, lower=(0.62, 4.5), span=None,
               glass=dict(x_front=6.05, x_back=3.25, half_width=0.58, peak=0.38),
               tail=[(2.85, 0.3), (2.45, 0.04)], bows=(5.5,),
               mouth=((0.86, 1.2), (1.56, 1.2), (1.56, 1.9), (0.86, 1.9)), engine=(-8.76, 1.2, 1.62, 0.42), tip=7.1,
               irst=(6.8, -0.25), lumps=((6.6, 7.0), (0.1, 0.4), 3.0)),
    SU30: dict(length=21.84, lower=(0.95, 4.85), span=(3.0, 7.3),
               glass=dict(x_front=6.05, x_back=2.55, half_width=0.6, peak=0.44),
               tail=[(2.1, 0.25), (1.7, 0.04)], bows=(5.5, 4.0),
               mouth=((0.92, 1.3), (1.62, 1.3), (1.62, 2.1), (0.92, 2.1)), engine=(-9.35, 1.27, 1.7, 0.44), tip=7.25,
               irst=(6.55, -0.17), lumps=((6.35, 6.75), (0.0, 0.32), 2.99)),
    SU35: dict(length=21.8, lower=(0.41, 4.3), span=(3.0, 7.6),
               glass=dict(x_front=6.05, x_back=3.4, half_width=0.56, peak=0.35),
               tail=[(3.0, 0.25), (2.6, 0.04)], bows=(5.5,),
               mouth=((0.8, 1.3), (1.62, 1.3), (1.62, 1.95), (0.8, 1.95)), engine=(-9.26, 1.2, 1.62, 0.42), tip=7.55,
               irst=(6.95, -0.12), lumps=((6.8, 7.1), (0.0, 0.2), 2.97)),
    SU34: dict(length=23.24, lower=(0.97, 4.75), span=(3.0, 7.3),
               glass=dict(x_front=7.05, x_back=4.35, half_width=0.92, peak=0.37),
               tail=[(3.95, 0.35), (3.6, 0.05)], bows=(6.5,),
               mouth=((1.15, 1.32), (1.7, 1.32), (1.7, 2.05), (1.15, 2.05)), engine=(-8.4, 1.35, 1.78, 0.4), tip=7.2,
               irst=(7.4, -0.45), lumps=((7.2, 7.6), (0.0, 0.7), 2.83)),
}
# The skins' wings out from the booms, ahead of the stabilators, and their
# fins above the booms, split off by name.
PARTS = [("wing_L", ((-6.3, 3.0), (2.9, 9.0), (1.5, 3.2))), ("wing_R", ((-6.3, 3.0), (-9.0, -2.9), (1.5, 3.2))),
         ("tail_fin_0", ((-11.5, -3.5), (1.6, 3.4), (2.95, 9.0))),
         ("tail_fin_1", ((-11.5, -3.5), (-3.4, -1.6), (2.95, 9.0)))]
# The wing's stations (x, y, length, depth) and their missiles (length, radius).
PYLONS = [((-0.6, 3.4, 1.3, 0.34), (3.6, 0.12)), ((-1.5, 4.8, 1.1, 0.28), (3.0, 0.09))]


def _canopy(fit):
    glass = framed_plan(**fit["glass"])
    return dict(glass=glass, tail=[glass[-1]] + fit["tail"], bows=fit["bows"])


def _edge(shell, x):
    """The skin's half width at `x`, from above."""
    ys = []
    for k in range(60):
        try:
            shell.over(x, k * 0.05)
            ys.append(k * 0.05)
        except ValueError:
            pass
    return max(ys)


def _lumps(xs, ys, z):
    """`press` boxes flattening a lump each side of the centreline (one
    across it when `ys` starts there) down to `z`."""
    if ys[0] == 0:
        return [(xs, (-ys[1], ys[1]), (z, 9.0))]
    return [(xs, ys, (z, 9.0)), (xs, (-ys[1], -ys[0]), (z, 9.0))]


def spec(variant):
    vid = variant["id"]
    fit = FIT[vid]
    x, y, z, r = fit["engine"]
    s = dict(
        skin=dict(path=f"{SKINS}/{vid}.glb", length=fit["length"], top=DIMENSIONS[vid][2], lower=fit["lower"],
                  span=fit["span"], canopy=_canopy(fit), parts=PARTS,
                  press=_lumps(*fit["lumps"]),
                  # The nacelles' flat ends go inside the nozzles.
                  cut=[((x - 1.0, x + 0.12), tuple(sorted((sd * (y - r), sd * (y + r)))), (z - r, z + r))
                       for sd in (1, -1)]),
        wing_hinge=(0.12, (0.78, 0, 1)),
        nozzles=[dict(loc=(x + 0.4, sd * y, z), r_front=r + 0.03, r_exit=r, length=0.95, petals=14) for sd in (1, -1)],
    )
    return s


def build(variant, v):
    s = spec(variant)
    m = A.jet(v, s)
    hull = v.hull
    vid = variant["id"]
    su34 = vid == SU34
    fit = FIT[vid]
    shell = Probe(hull)
    length = fit["length"]
    # The two-seaters' canards on the extensions' sides, level with where
    # their roots meet the skin.
    if vid in (SU30, SU34):
        root = _edge(shell, 4.8) - 0.06
        z = shell.over(4.8, root) - 0.06
        A.paired_surface("wing_canard", [(5.6, root, z, 1.7, 0.1), (4.7, root + 2.2, z, 0.8, 0.04)], m["paint"],
                         hull)
    # The ventral fins under the booms, canted out, their roots up into the skin.
    for k, side in ((2, 1), (3, -1)):
        A.fin(f"tail_fin_{k}", root_x=-4.9, root_z=shell.under(-5.7, side * BOOM_Y) + 0.04, height=0.8,
              root_chord=1.6, tip_chord=0.9, sweep_m=0.6, y=side * BOOM_Y, cant=side * 2.95, rudder=False, mats=m,
              parent=hull)
    # The gear, each leg's top under the skin: the twin nose wheels, the
    # mains (the Su-34's in tandem).
    A.gear(name="nose", x=7.0, y=0.0, top=shell.under(7.0, 0.0) + 0.02, radius=0.34, width=0.18, wheels=2,
           spread=0.3, door=(1.0, 0.6), mats=m, parent=hull)
    A.mirrored(lambda side, k: A.gear(name=f"main_{k}", x=-0.9, y=side * 2.2, top=shell.under(-0.9, side * 2.2) + 0.02,
                                      radius=0.52, width=0.28, door=(1.4, 0.7), rake=0.1,
                                      tandem=1.0 if su34 else 0.0, mats=m, parent=hull))
    # The wing's stations, each pylon from the wing's underside; the
    # missiles under the nacelles; the Su-34's bomb on the centreline.
    for k, ((px, py, plen, depth), (mlen, mr)) in enumerate(PYLONS):
        for side in (1, -1):
            top = shell.under(px, side * py)
            A.pylon(f"pylon_{k}_{side + 1}", (px, side * py, top), plen, depth, m, hull)
            A.store(f"store_{k}_{side + 1}", "missile", (px - 0.2, side * py, top - depth - mr - 0.02), mlen, mr, m,
                    hull)
    _, ey, _, _ = fit["engine"]
    A.mirrored(lambda side, k: A.store(f"store_belly_{k}", "missile",
                                       (0.0, side * ey, shell.under(0.0, side * ey) - 0.2), 3.6, 0.12, m, hull,
                                       rail=True))
    if su34:
        A.store("store_bomb", "bomb", (1.0, 0.0, shell.under(1.0, 0.0) - 0.36), 3.0, 0.25, m, hull)
    # The wing tips' rails and what they carry: the Su-35's jamming pods,
    # the others' missiles; each at the tip's mid-thickness.
    tip = fit["tip"]
    tx = sum(shell.chord(tip - 0.15, 2.0, -7.9)) / 2
    for side, k in ((1, "L"), (-1, "R")):
        mid = (shell.under(tx, side * (tip - 0.15)) + shell.over(tx, side * (tip - 0.15))) / 2
        box(f"wing_tip_rail_{k}", (2.4, 0.08, 0.1), (tx, side * tip, mid), m["dark"], hull)
        if vid == SU35:
            A.store(f"store_tip_{k}", "pod", (tx, side * (tip + 0.05), mid - 0.1), 2.2, 0.12, m, hull)
        else:
            A.store(f"store_tip_{k}", "missile", (tx + 0.1, side * (tip + 0.05), mid - 0.1), 2.9, 0.09, m, hull)
    nx = sum(shell.chord(tip - 0.35, 2.0, -7.9)) / 2
    A.nav_lights(m, hull, left=(nx, tip - 0.35, shell.over(nx, tip - 0.35) + 0.01),
                 right=(nx, -tip + 0.35, shell.over(nx, -tip + 0.35) + 0.01),
                 tail=(-length / 2 + 0.6, 0.0, shell.over(-length / 2 + 0.6, 0.0) + 0.02))
    mouth = fit["mouth"]
    A.mirrored(lambda side, k: intake_mouth(f"intake_mouth_{k}", shell, m["black"], hull, side, mouth))
    if vid != SU35:
        box("spine_brake", (2.0, 0.56, 0.05), (0.6, 0, shell.over(0.6, 0) + 0.005), m["paint"], hull, bevel=0.02,
            lods=MID)
    # The infrared search sensor ahead of the windscreen.
    ix, iy = fit["irst"]
    A.sensor_ball("irst", (ix, iy, shell.over(ix, iy) + 0.06), 0.15, m, hull)
    A.blade_antenna("antenna_spine", (-1.5, 0, shell.over(-1.5, 0)), 0.25, m, hull)
    # The VKS's red star on the fins (out and in) and under the wings; the
    # red bort number on the nose (52 and 01 in the side photos).
    fin_x = -4.4 - 2.4 * 0.72 - 1.0
    fin_z = FIN_Z + (DIMENSIONS[vid][2] - FIN_Z) * 0.72
    rows = [("insignia", dict(kind="ru_star", centre=(fin_x, y, fin_z), normal=(0, n, 0), up=(0, 0, 1), size=0.36,
                              onto=("tail_fin_0", "tail_fin_1"), mirror=False))
            for y, n in ((BOOM_Y + 0.25, 1), (BOOM_Y - 0.25, -1), (-BOOM_Y - 0.25, -1), (-BOOM_Y + 0.25, 1))]
    rows += [("insignia", dict(kind="ru_star", centre=(-1.6, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                               size=0.7, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)]
    bort = {SU27: "52", SU30: "27", SU35: "04", SU34: "01"}[vid]
    rows.append(("text", dict(text=bort, height=0.62, centre=((5.2 if su34 else 7.6), 1.4, 2.45), normal=(0, 1, 0),
                              up=(0, 0, 1), onto=("fuselage",), colour="red")))
    A.markings(v, rows)


def wreck(variant, v):
    # The span is the widest of the roster's: the torn panels lie a little
    # nearer, inside the wreck's footprint.
    A.crash(v, tail_x=-5.2, wing_y=4.6, wing_side=-1, tail_yaw=-0.25, seed=27, litter_reach=0.9)


run("sukhoi_flanker_family", "russian_air_blue", build, wreck, script="flanker.py")
