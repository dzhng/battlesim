"""The earthworks (`fixtures/props/military/works.json`): the revetment and HESCO, for
`military.py`."""
import bpy, bmesh, math, random
from mathutils import Euler, Matrix, Vector, noise

from parts import *
import parts
from military_parts import *


# ------------------------------------------------------------------ the revetment
@kind
def revetment():
    """A bin revetment, the kind that walls aircraft and vehicle bays: faces of galvanised
    corrugated steel held 3 m apart in a frame of posts a metre apart, a heavy sill and a
    top rail, tied across, the ends closed and framed the same way, filled with earth to
    just under the rim and grassing over, the bay number stencilled on the +X end. Rust
    weeps from the fixings and dust films the foot."""
    hx, hy, hz = 8.0, 1.5, 1.6
    face_m = textured("bin_face", "cladding", colour=(0.27, 0.27, 0.255), chip=0.4, dirt=0.8, rise=0.9, streak=0.7,
                      mottle=0.2, grain=0.02)
    post_m = textured("bin_post", "enamel", colour=(0.07, 0.065, 0.055), chip=0.7, dirt=0.8, rise=0.8)
    soil = textured("bin_soil", "soil", colour=(0.13, 0.105, 0.075), chip=0.0, dirt=0.0, streak=0.0, mottle=0.25,
                    lichen=0.45, grain=0.04)
    mark = flat_paint("bin_mark", (0.36, 0.33, 0.26), rough=0.85, grime=0.7)
    bin_ = body("bin")
    face_h, post_w, wall = 2 * hz - 0.14, 0.12, hy - 0.16  # the sheet's top; a post's flange; the faces' half spacing
    ex = hx - 0.16  # the ends' sheet
    for k, s in enumerate((-1, 1)):
        box(f"face_{k}", (2 * ex + 0.06, 0.04, face_h), (0, s * wall, face_h / 2), face_m, bin_)
        box(f"end_{k}", (0.04, 2 * wall, face_h), (s * ex, 0, face_h / 2), face_m, bin_)

    def frame(k, run, at):
        """One face's frame: posts every metre along `run` metres, a heavy sill and a top
        rail; `at(t, out)` places a point `t` along the face, `out` off it."""
        n = round(run)
        for i in range(n + 1):
            box(f"post_{k}_{i}", (post_w, post_w, face_h + 0.06), at(-run / 2 + run * i / n, post_w / 2 + 0.02) + Vector((0, 0, (face_h + 0.06) / 2)),
                post_m, bin_, rot=(0, 0, 0), lods=(0, 1, 2))
        for name, z, h, d in (("sill", 0.14, 0.28, 0.1), ("rail", face_h - 0.06, 0.1, 0.07)):
            a, b = at(-run / 2, d / 2 + 0.02), at(run / 2, d / 2 + 0.02)
            bar(f"{name}_{k}", a + Vector((0, 0, z)), b + Vector((0, 0, z)), d, h, post_m, bin_, lods=(0, 1, 2))

    for k, s in enumerate((-1, 1)):
        frame(f"side{k}", 2 * ex, lambda t, out, s=s: Vector((t, s * (wall + out), 0)))
        frame(f"end{k}", 2 * wall, lambda t, out, s=s: Vector((s * (ex + out), t, 0)))
    for i in range(1, 16):
        x = -ex + 2 * ex * i / 16
        bar(f"tie_{i}", (x, -wall, face_h - 0.1), (x, wall, face_h - 0.1), 0.025, 0.025, post_m, bin_, lods=(0, 1))
    # the fill: earth heaped inside the faces to their rim, lumpy

    def fill(bm, lod):
        nu, nv = (64, 32, 12, 4)[lod], (12, 8, 4, 2)[lod]

        def at(u, v):
            x, y = -ex + 0.02 + (2 * ex - 0.04) * u, -wall + 0.02 + (2 * wall - 0.04) * v
            crown = 0.16 * math.sin(math.pi * v) ** 0.6 * min(1.0, (ex - abs(x)) / 0.4 + 0.3)
            lumps = 0.05 * noise.noise(Vector((x * 0.9, y * 1.3, 3.0))) + 0.02 * noise.noise(Vector((x * 4, y * 4, 7.0)))
            return (x, y, face_h - 0.15 + max(0.0, crown + lumps * math.sin(math.pi * v)))

        grid(bm, nu, nv, at)
        return True

    mesh_part("fill", fill, soil, bin_, smooth=True)
    stencil("bay_number", "R4", 0.55, (ex + 0.025, 0, face_h * 0.62), (math.pi / 2, 0, math.pi / 2), mark, bin_,
            lods=(0, 1, 2))
    return [hx, hy, hz]


# ------------------------------------------------------------------ HESCO
CELL_H = 1.36  # the cells' wire, from the ground


@kind
def hesco():
    """A run of HESCO: six earth-filled cells, each a galvanised weld-mesh basket lined with
    sand-coloured geotextile, 1.07 m long and 1.1 m deep. Each cell's liner pillows out
    between the coil joints that hinge the cells together, which sit in the creases; the
    liner's top folds over the wire rim, and the earth inside lies a little below it, heaped
    differently in each cell. One module: the battle repeats it along a placed wall, so its
    ends are half joints that meet the next module's."""
    hx, hy, hz = 3.2, 0.55, 0.7
    liner = textured("hesco_liner", "hesco_liner", chip=0.0, dirt=0.9, rise=0.5, streak=0.4, mottle=0.12,
                     grain=0.03)
    fill = textured("hesco_fill", "soil", colour=(0.19, 0.16, 0.115), chip=0.0, dirt=0.0, streak=0.0, lichen=0.1)
    wire = textured("hesco_wire", "galvanised", chip=0.2, dirt=0.6, rise=0.4)
    run = body("cells")
    cells, wall = 6, hy - 0.07
    length = 2 * hx / cells

    def bulge(x, z):
        """How far the liner stands out from the wire's line: pillowed mid-cell, creased in
        at the joints, flattening toward the ground and the rim."""
        f = ((x + hx) / length) % 1.0
        across = 0.07 * math.sin(math.pi * f) ** 0.7 - 0.018
        return across * math.sin(math.pi * min(1.0, z / CELL_H)) ** 0.5

    def rim(x):  # the basket's top: it dips between the joints under the fill's weight
        return CELL_H - 0.01 - 0.035 * math.sin(math.pi * (((x + hx) / length) % 1.0))

    def sides(bm, lod):
        nu, nv = (cells * 10, cells * 4, cells * 2, cells)[lod], (10, 6, 3, 1)[lod]
        for s in (-1, 1):
            grid(bm, nu, nv, lambda u, v, s=s: (-hx + 2 * hx * u, s * (wall + bulge(-hx + 2 * hx * u, CELL_H * v)),
                                                 (rim(-hx + 2 * hx * u) - 0.01) * v), flip=s > 0)
        return True

    mesh_part("liner_sides", sides, liner, run)

    def ends(bm, lod):
        for s in (-1, 1):
            x = s * (hx - 0.005)
            vs = [bm.verts.new((x, y, z)) for y, z in ((-wall, 0), (wall, 0), (wall, CELL_H - 0.02), (-wall, CELL_H - 0.02))]
            bm.faces.new(vs if s > 0 else vs[::-1])  # listed facing +X
        return True

    mesh_part("liner_ends", ends, liner, run)
    # each cell's rim wire and the liner folded in over it, dipping between the joints; a
    # divider of liner and wire across the top at each joint
    for k in range(cells):
        for j, s in enumerate((-1, 1)):
            for h, (u0, u1) in enumerate(((0.0, 0.5), (0.5, 1.0))):
                xa, xb = -hx + (k + u0) * length, -hx + (k + u1) * length
                a, b = Vector((xa, s * wall, rim(xa))), Vector((xb, s * wall, rim(xb - 1e-6)))
                bar(f"rim_{k}_{j}_{h}", a, b, 0.014, 0.014, wire, run, lods=(0, 1, 2))
                inner = Vector((0, -s * 0.03, -0.035))
                bar(f"lip_{k}_{j}_{h}", a + inner, b + inner, 0.012, 0.07, liner, run, lods=(0, 1, 2))
    for k in range(1, cells):
        x = -hx + k * length
        box(f"divider_{k}", (0.014, 2 * wall - 0.02, 0.12), (x, 0, CELL_H - 0.08), liner, run, lods=(0, 1, 2))

    def earth(bm, lod):
        nu, nv = (cells * 8, cells * 3, cells, cells)[lod], (6, 3, 2, 1)[lod]

        def at(u, v):
            x = -hx + 2 * hx * u
            cell = min(cells - 1, int((x + hx) / length))
            heap = (0.05 + 0.04 * random.Random(cell).random()) * math.sin(math.pi * v) * math.sin(math.pi * ((x + hx) / length % 1.0))
            return (x, -wall + 0.04 + 2 * (wall - 0.04) * v, CELL_H - 0.2 + heap + 0.02 * noise.noise(Vector((x * 5, v * 6, 3.0))))

        grid(bm, nu, nv, at)
        return True

    mesh_part("fill", earth, fill, run)

    # the coil joints in the creases (half joints at the module's ends): a stack of wire turns
    def coils(bm, lod):
        turns = (14, 7, 0, 0)[lod]
        for k in range(cells + 1):
            x = -hx + k * length + (0.014 if k == 0 else -0.014 if k == cells else 0.0)
            for s in (-1, 1):
                y = s * (wall - 0.012)
                tall = CELL_H - 0.02
                bmesh.ops.create_cone(bm, cap_ends=True, segments=6, radius1=0.012, radius2=0.012, depth=tall,
                                      matrix=Matrix.Translation((x, y, tall / 2)))
                for t in range(turns):
                    bmesh.ops.create_cone(bm, cap_ends=False, segments=8, radius1=0.026, radius2=0.026, depth=0.012,
                                          matrix=Matrix.Translation((x, y, 0.05 + t * (tall - 0.1) / max(1, turns - 1))))
        return True

    mesh_part("coils", coils, wire, run, lods=(0, 1))
    return [hx, hy, hz]
