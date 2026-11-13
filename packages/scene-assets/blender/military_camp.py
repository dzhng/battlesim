"""The camp (`fixtures/props/military/camp.json`): the tent, the flagpole and the net
over supplies, for `military.py`."""
import bpy, bmesh, math, random
from mathutils import Euler, Matrix, Vector, noise

from parts import *
import parts
from military_parts import *


# ------------------------------------------------------------------ the tent
@kind
def tent():
    """A general-purpose field tent, 6 m by 5 m and 2.8 m high: olive duck canvas over a
    ridge, low walls under a scalloped valance, slack between its poles, guyed out to
    pegs along both eaves and from each gable. The pins of its two centre poles show
    through the ridge; the front (+X) gable's door stands open, its flaps rolled and tied
    back either side of the dark inside."""
    hx, hy, hz = 3.0, 2.5, 1.4
    duck = textured("tent_canvas", "nylon", colour=(0.105, 0.11, 0.068), chip=0.1, dirt=0.35, rise=0.35, streak=0.2,
                    mottle=0.1, grain=0.04)
    rope = flat_paint("tent_rope", (0.2, 0.18, 0.13), rough=0.9, grime=0.5)
    wood = textured("tent_stake", "pallet_wood", colour=(0.12, 0.09, 0.06), chip=0.2, dirt=0.8, rise=0.3)
    inside = flat_paint("tent_inside", (0.015, 0.016, 0.012), rough=1.0, grime=0.0)
    canvas = body("canvas")
    rig = body("rigging")
    wall_y, eave_y, eave_z, ridge_z = 1.72, 1.88, 1.45, 2 * hz - 0.12
    gable_x, roof_x = 2.5, 2.66
    stations = (-roof_x, -0.85, 0.85, roof_x)

    def sag(x):  # the canvas sags between the eave poles
        for a, b in zip(stations, stations[1:]):
            if a <= x <= b:
                return 0.06 * math.sin(math.pi * (x - a) / (b - a))
        return 0.0

    def eave_at(x):
        return eave_z - sag(x)

    def roof(bm, lod):
        nu, nv = (36, 18, 8, 2)[lod], (10, 6, 3, 1)[lod]
        for s in (-1, 1):
            def at(u, v, s=s):
                x = -roof_x + 2 * roof_x * u
                y = s * eave_y * (1 - v)
                z = eave_at(x) + (ridge_z - eave_at(x)) * v - 0.05 * math.sin(math.pi * v) * (0.6 + 0.4 * sag(x) / 0.06)
                return (x, y, z)
            grid(bm, nu, nv, at, flip=s > 0)
        return True

    mesh_part("roof", roof, duck, canvas, smooth=True)

    def walls(bm, lod):
        nu, nv = (48, 24, 8, 2)[lod], (6, 3, 2, 1)[lod]
        for s in (-1, 1):
            def at(u, v, s=s):
                x = -gable_x + 2 * gable_x * u
                bow = 0.05 * math.sin(math.pi * u) * math.sin(math.pi * v)  # drawn in between the corners
                fold = 0.025 * math.sin(x * 9.0 + s) * (1 - v)
                return (x, s * (wall_y + 0.06 * (1 - v) ** 2 + fold - bow), (eave_at(x) - 0.04) * v)
            grid(bm, nu, nv, at, flip=s > 0)
        return True

    mesh_part("walls", walls, duck, canvas, smooth=True)

    def gables(bm, lod):
        for s in (-1, 1):
            x = s * gable_x
            pts = [(-wall_y - 0.06, 0.0), (wall_y + 0.06, 0.0), (wall_y, eave_z - 0.02), (0.0, ridge_z - 0.04),
                   (-wall_y, eave_z - 0.02)]
            vs = [bm.verts.new((x, y, z)) for y, z in pts]
            bm.faces.new(vs if s > 0 else vs[::-1])  # listed facing +X
        return True

    mesh_part("gables", gables, duck, canvas)

    # the valance: a scalloped skirt hanging off each eave, a little flared
    def valance(bm, lod):
        nu = (36, 18, 8, 2)[lod]
        for s in (-1, 1):
            def at(u, v, back=0.0, s=s):
                x = -roof_x + 2 * roof_x * u
                drop = 0.2 - 0.03 * (1 - math.cos(2 * math.pi * u * 12)) * (1 - v)
                return (x, s * (eave_y + 0.03 * (1 - v) - back), eave_at(x) - drop * (1 - v) - 0.01)
            grid(bm, nu, 1, at, flip=s < 0)
            grid(bm, nu, 1, lambda u, v, s=s: at(u, v, 0.006), flip=s > 0)
        return True

    mesh_part("valance", valance, duck, canvas, lods=(0, 1, 2))
    # the open door: the dark inside, its two flaps folded back flat either side and tied
    dx, dw, dh = gable_x + 0.004, 0.5, 1.95
    prism("doorway", [(-dw, 0.0), (dw, 0.0), (dw * 0.6, dh), (-dw * 0.6, dh)], 0.004, (dx, 0, 0), inside, canvas,
          rot=(0, 0, math.pi / 2))
    for k, s in enumerate((-1, 1)):
        flap = [(s * dw, 0.03), (s * (dw + 0.5), 0.03), (s * dw * 0.6, dh)]
        prism(f"door_flap_{k}", flap if s > 0 else flap[::-1], 0.02, (dx + 0.012, 0, 0), duck, canvas,
              rot=(0, 0, math.pi / 2), lods=(0, 1, 2))
        box(f"door_tie_{k}", (0.03, 0.04, 0.5), (dx + 0.03, s * (dw + 0.18), 0.6), rope, canvas, lods=(0,),
            rot=(s * 0.5, 0, 0))
    # the centre poles' pins through the ridge
    for k, x in enumerate((-1.2, 1.2)):
        cyl(f"pole_washer_{k}", 0.04, 0.02, (x, 0, ridge_z + 0.0), "Z", wood, canvas, seg=8, lods=(0, 1))
        cyl(f"pole_pin_{k}", 0.013, 0.12, (x, 0, ridge_z + 0.06), "Z", wood, canvas, seg=6, lods=(0, 1, 2))
    # guys from each eave station and each gable's peak, out to pegs inside the box
    guys = []
    for x in stations[1:-1] + (-roof_x + 0.05, roof_x - 0.05):
        for s in (-1, 1):
            guys.append(((x, s * (eave_y + 0.004), eave_at(x) + 0.004), (x, s * (hy - 0.06), 0.01)))
    for s in (-1, 1):
        guys.append(((s * (roof_x + 0.004), 0, ridge_z - 0.12), (s * (hx - 0.06), 0, 0.01)))
    for k, (a, b) in enumerate(guys):
        tube(f"guy_{k}", a, b, 0.009, rope, rig, lods=(0, 1), seg=4)
        lean = (Vector(b) - Vector(a)).normalized()
        box(f"peg_{k}", (0.025, 0.025, 0.16), Vector(b) + Vector((0, 0, 0.05)) - lean * 0.01, wood, rig, lods=(0, 1),
            rot=Vector((0, 0, 1)).rotation_difference(Vector((0, 0, 1)) - lean * 0.4).to_euler())
    return [hx, hy, hz]


# ------------------------------------------------------------------ the flagpole
@kind
def flagpole():
    """An 8 m yard flagpole: a tapered aluminium pole in a ground sleeve on a small concrete
    pad, a ball truck on top, the halyard's two runs down to their cleat, and a generic
    olive flag with a sand band, flying out along +X from two snap hooks on the halyard,
    caught mid-ripple in a breeze, its fly drooping and curling. The flag and pad stand
    past the pole's box (the catalog's footprint tolerance): cloth over everyone's heads
    and a slab underfoot, not a body."""
    hx, hy, hz = 0.08, 0.08, 4.0
    alu = textured("pole_alu", "galvanised", colour=(0.45, 0.46, 0.46), chip=0.1, dirt=0.4, rise=0.8)
    cord = flat_paint("halyard", (0.55, 0.53, 0.48), rough=0.8, grime=0.3)
    concrete = textured("pole_pad", "concrete", chip=0.4, dirt=0.9, rise=0.3)
    pole = body("pole")
    flag = body("flag")
    top = 2 * hz - 0.13
    box("pad", (0.46, 0.46, 0.1), (0, 0, 0.05), concrete, pole, bevel=0.01, lods=(0, 1, 2))
    cyl("sleeve", 0.075, 0.12, (0, 0, 0.16), "Z", alu, pole, seg=16, lods=(0, 1, 2))
    cyl("pole", 0.055, top - 0.1, (0, 0, 0.1 + (top - 0.1) / 2), "Z", alu, pole, seg=16, r2=0.035)
    cyl("truck", 0.05, 0.05, (0, 0, top + 0.02), "Z", alu, pole, seg=12, lods=(0, 1, 2))
    cyl("ball", 0.065, 0.11, (0, 0, top + 0.095), "Z", alu, pole, seg=12, r2=0.0, lods=(0, 1, 2))
    hal_x, cleat_z = 0.05, 1.3
    tube("halyard_up", (hal_x, 0.0, cleat_z), (hal_x, 0.0, top + 0.03), 0.006, cord, pole, lods=(0, 1, 2), seg=5)
    tube("halyard_down", (0.03, 0.035, cleat_z), (0.03, 0.035, top + 0.03), 0.006, cord, pole, lods=(0, 1), seg=5)
    box("cleat", (0.04, 0.03, 0.22), (0.05, 0.01, cleat_z), alu, pole, lods=(0, 1, 2))
    # the flag: one cloth whose hoist stands on the halyard; its colour is laid by height
    fw, fh, ftop = 1.5, 0.95, top - 0.08
    x0 = hal_x + 0.014
    rows = (0.0, 0.18, 0.355, 0.365, 0.5, 0.635, 0.645, 0.82, 1.0)  # band edges held by close rows

    def at(u, v):
        x = x0 + fw * u
        wave = 0.22 * u ** 1.2 * math.sin(2 * math.pi * (1.4 * u - 0.2 * v) + 0.4 * math.sin(3.0 * v))
        wave += 0.03 * u * math.sin(2 * math.pi * (3.1 * u + 0.5 * v))
        curl = 0.08 * max(0.0, (u - 0.82) / 0.18) ** 2 * (1 - v)  # the fly's lower corner turns over
        z = ftop - fh + v * fh - 0.25 * u * u * (0.6 + 0.4 * (1 - v))
        return (x, wave + curl, z)

    def where(p):  # a world point on the flag back to its (u, v)
        u = max(0.0, min(1.0, (p.x - x0) / fw))
        return (p.z - (ftop - fh) + 0.25 * u * u * (0.6 + 0.4 * (1 - 0.5))) / fh

    @vertex_paint("flag_cloth", rough=0.75)
    def cloth(p, n, edge):
        v = where(p)
        return (0.3, 0.26, 0.16) if 0.36 <= v <= 0.64 else (0.075, 0.095, 0.05)

    def build(bm, lod):
        nu = (24, 12, 6, 2)[lod]
        vs = rows if lod < 2 else (0.0, 0.36, 0.64, 1.0)
        for side, off in ((0, -0.002), (1, 0.002)):
            grid_rows = [[bm.verts.new(Vector(at(i / nu, v)) + Vector((0, off, 0))) for v in vs] for i in range(nu + 1)]
            for i in range(nu):
                for j in range(len(vs) - 1):
                    q = (grid_rows[i][j], grid_rows[i + 1][j], grid_rows[i + 1][j + 1], grid_rows[i][j + 1])  # faces -Y
                    bm.faces.new(q[::-1] if side else q)
        return True

    mesh_part("flag", build, bpy.data.materials["flag_cloth"], flag, smooth=True)
    for k, z in enumerate((ftop - 0.03, ftop - fh + 0.03)):
        box(f"snap_{k}", (0.04, 0.016, 0.07), (hal_x + 0.01, 0, z), alu, flag, lods=(0, 1))
    return [hx, hy, hz]
