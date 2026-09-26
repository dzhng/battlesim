"""Village masonry: plaster, brick, stone, roof tiles and timber paints, and the
wall, roof and rubble builders the houses and village props share."""
import bmesh, math, random
from mathutils import Vector, Matrix
from parts import *


def _plaster(base, seed, burnt_=False):
    def fn(p, n, edge):
        # a plinth of rough stone, plaster above with patches fallen off to brick
        if p.z < 0.55:
            c = (0.17, 0.16, 0.14)
            c = tuple(x * (1 + 0.25 * fbm(p, 3.0, 2, seed + 1)) for x in c)
        else:
            c = tuple(x * (1 + 0.06 * fbm(p, 1.3, 3, seed)) for x in base)
            spall = fbm(p, 0.45, 3, seed + 4.0) + 0.15 * fbm(p, 5.0, 2, seed + 2)
            if spall > 0.5:
                c = lerp3(c, (0.2, 0.1, 0.065), smoothstep(0.5, 0.56, spall) * 0.85)
        c = streaks(c, p, n, 0.5, (0.12, 0.11, 0.09))
        if edge > 0.3:
            c = lerp3(c, tuple(x * 0.8 for x in c), 0.4)
        if burnt_:
            # soot rises with the fire: blackest at the broken tops and round the openings
            soot = smoothstep(-0.2, 1.1, p.z) * (0.75 + 0.25 * (0.5 + 0.5 * fbm(p, 0.8, 3, seed + 9)))
            c = lerp3(c, (0.012, 0.011, 0.01), min(0.97, soot))
        return grime_rise(c, p, 0.9, top=1.1, colour=(0.12, 0.1, 0.075))

    return fn


def plaster(name, base=(0.52, 0.47, 0.38), seed=1.0, rough=0.92, burnt_=False):
    return paint(name, rough)(_plaster(base, seed, burnt_))


def brick(name, seed=3.0, burnt_=False):
    def fn(p, n, edge):
        course = 0.075
        row = math.floor(p.z / course)
        off = 0.12 if row % 2 else 0.0
        u = (p.x + p.y + off) / 0.25
        mortar = (p.z / course - row) < 0.14 or (u - math.floor(u)) < 0.06
        c = (0.028, 0.026, 0.022) if mortar else (0.21, 0.085, 0.055)
        k = 1 + 0.3 * fbm(Vector((math.floor(u), row, 0)), 0.9, 1, seed)
        c = tuple(x * k for x in c)
        if burnt_:
            c = lerp3(c, (0.02, 0.018, 0.016), 0.35 + 0.4 * max(0, fbm(p, 0.8, 3, seed + 5)))
        return grime_rise(c, p, 0.8, top=1.0, colour=(0.1, 0.08, 0.06))

    return paint(name, 0.9)(fn)


def stone(name, base=(0.25, 0.24, 0.21), seed=5.0):
    def fn(p, n, edge):
        c = tuple(x * (1 + 0.3 * fbm(p, 2.5, 3, seed)) for x in base)
        if n.z > 0.7:
            c = lerp3(c, (0.12, 0.14, 0.07), max(0.0, fbm(p, 1.8, 2, seed + 3)) * 0.8)  # moss on top
        return grime_rise(c, p, 0.7, top=0.6, colour=(0.1, 0.09, 0.07))

    return paint(name, 0.95)(fn)


def tiles(name, base=(0.3, 0.11, 0.06), seed=7.0):
    """Clay tiles: per-tile tone, lichen and soot near the ridge."""

    def fn(p, n, edge):
        tile = Vector((math.floor(p.x / 0.3), math.floor(p.y / 0.3), math.floor(p.z / 0.3)))
        k = 1 + 0.28 * fbm(tile, 0.7, 1, seed)
        c = tuple(x * k for x in base)
        lichen = fbm(p, 0.7, 3, seed + 2)
        c = lerp3(c, (0.18, 0.16, 0.1), smoothstep(0.1, 0.5, lichen) * 0.55)
        c = lerp3(c, (0.05, 0.04, 0.035), smoothstep(0.3, 0.6, fbm(p, 0.35, 2, seed + 8)) * 0.5)
        return c

    return paint(name, 0.8)(fn)


def timber(name, base=(0.13, 0.085, 0.05), seed=9.0):
    def fn(p, n, edge):
        g = fbm(Vector((p.x * 0.2, p.y * 0.2, p.z * 3.0)), 3.0, 2, seed)
        c = tuple(x * (1 + 0.25 * g) for x in base)
        c = lerp3(c, (0.2, 0.19, 0.17), smoothstep(0.2, 0.6, fbm(p, 2.0, 2, seed + 1)) * 0.35)  # weathered grey
        return grime_rise(c, p, 0.6, top=0.8)

    return paint(name, 0.85)(fn)


def charred(name, seed=11.0):
    def fn(p, n, edge):
        c = lerp3((0.015, 0.014, 0.013), (0.09, 0.08, 0.07), 0.5 + 0.5 * fbm(p, 2.0, 3, seed))
        if edge > 0.3:
            c = lerp3(c, (0.2, 0.08, 0.03), 0.3 * edge)  # embers' glow gone to rust-brown
        return c

    return paint(name, 0.95)(fn)


def rubble_paint(name, seed=13.0):
    def fn(p, n, edge):
        f = 0.5 + 0.5 * fbm(p, 1.6, 3, seed)
        c = lerp3((0.12, 0.115, 0.105), (0.11, 0.05, 0.035), smoothstep(0.5, 0.7, f))  # grey dust and ash, broken brick
        c = lerp3(c, (0.03, 0.028, 0.025), 0.25 + smoothstep(0.45, 0.8, fbm(p, 0.8, 2, seed + 3)) * 0.6)  # soot and ash
        return tuple(x * (0.8 + 0.4 * (0.5 + 0.5 * fbm(p, 9.0, 1, seed + 6))) for x in c)

    return paint(name, 1.0)(fn)


# ---------------------------------------------------------------- builders
def gable_roof(name, x0, x1, y0, y1, eave_z, ridge_z, mat, parent=None, overhang=0.35, along="x", courses=True,
               thickness=0.14, lods=TIERS):
    """A gable roof over the rectangle, ridge along `along`. On the finest tiers the
    tiles are laid as overlapping courses, so the roof has real relief."""
    if along == "y":
        # build along x then rotate: swap
        return _gable(name, y0, y1, x0, x1, eave_z, ridge_z, mat, parent, overhang, courses, thickness, lods, True)
    return _gable(name, x0, x1, y0, y1, eave_z, ridge_z, mat, parent, overhang, courses, thickness, lods, False)


def _gable(name, a0, a1, b0, b1, eave, ridge, mat, parent, over, courses, th, lods, swap):
    bc = (b0 + b1) / 2
    half = (b1 - b0) / 2 + over
    rise = ridge - eave
    slope = math.atan2(rise, (b1 - b0) / 2)
    eave_z = eave - over * math.tan(slope)
    run = half / math.cos(slope)

    def P(a, b, z):
        return (b, a, z) if swap else (a, b, z)

    def build(bm, lod):
        A0, A1 = a0 - over * 0.6, a1 + over * 0.6
        n = max(1, int(run / 0.34)) if (courses and lod == 0) else 1
        for side in (-1, 1):
            for i in range(n):
                t0, t1 = i / n, (i + 1) / n + (0.25 / n if n > 1 else 0)
                t1 = min(t1, 1.0)
                # points along the slope from eave (t=0) to ridge (t=1)
                def at(t, lift=0.0):
                    b = bc + side * half * (1 - t)
                    z = eave_z + (ridge - eave_z) * t + lift
                    return b, z
                lip = 0.035 if n > 1 else 0.0
                bA, zA = at(t0, -lip)
                bB, zB = at(t1)
                vs = []
                for a in (A0, A1):
                    for (bb, zz) in ((bA, zA), (bB, zB)):
                        vs.append(bm.verts.new(P(a, bb, zz)))
                        vs.append(bm.verts.new(P(a, bb, zz - th)))
                # vs: a0: [A top, A bot, B top, B bot], a1: same
                v = vs
                faces = [(0, 2, 6, 4), (1, 5, 7, 3), (0, 4, 5, 1), (2, 3, 7, 6), (0, 1, 3, 2), (4, 6, 7, 5)]
                for f in faces:
                    try:
                        bm.faces.new([v[k] for k in f])
                    except ValueError:
                        pass
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        if swap:
            pass

    mesh_part(name, build, mat, parent, lods)
    # ridge cap
    cap_len = (a1 - a0) + over * 1.2
    ac = (a0 + a1) / 2
    if swap:
        box(name + "_ridge", (0.3, cap_len, 0.16), (bc, ac, ridge + 0.02), mat, parent, bevel=0.03, lods=(0, 1, 2),
            rot=(0, math.radians(45), 0))
    else:
        box(name + "_ridge", (cap_len, 0.3, 0.16), (ac, bc, ridge + 0.02), mat, parent, bevel=0.03, lods=(0, 1, 2),
            rot=(math.radians(45), 0, 0))
    # barge boards up each verge, covering the courses' ends, and a fascia along each eave
    trim = bpy_trim()
    A0, A1 = a0 - over * 0.6, a1 + over * 0.6
    for side in (-1, 1):
        bmid = bc + side * half / 2
        zmid = (eave_z + ridge) / 2 - th * 0.5
        for k, a in enumerate((A0 - 0.03, A1 + 0.03)):
            if swap:
                box(f"{name}_barge_{k}_{'ab'[side > 0]}", (run + 0.1, 0.05, 0.24), (bmid, a, zmid), trim, parent,
                    rot=(0, side * slope, 0), lods=(0, 1))
            else:
                box(f"{name}_barge_{k}_{'ab'[side > 0]}", (0.05, run + 0.1, 0.24), (a, bmid, zmid), trim, parent,
                    rot=(-side * slope, 0, 0), lods=(0, 1))
        be = bc + side * (half + 0.01)
        if swap:
            box(f"{name}_fascia_{'ab'[side > 0]}", (0.05, A1 - A0, 0.2), (be, (A0 + A1) / 2, eave_z - th * 0.6), trim, parent,
                lods=(0, 1))
        else:
            box(f"{name}_fascia_{'ab'[side > 0]}", (A1 - A0, 0.05, 0.2), ((A0 + A1) / 2, be, eave_z - th * 0.6), trim, parent,
                lods=(0, 1))


_TRIM = []


def bpy_trim():
    if not _TRIM:
        _TRIM.append(timber("roof_trim", (0.09, 0.06, 0.04), seed=17.0))
    return _TRIM[0]


def wing(name, x0, x1, y0, y1, eave, ridge, wall_mat, roof_mat, parent=None, along="x", gables=(True, True)):
    """A plastered wing: walls to the eaves, triangular gable ends, a tiled roof."""
    box(name + "_walls", (x1 - x0, y1 - y0, eave), ((x0 + x1) / 2, (y0 + y1) / 2, eave / 2), wall_mat, parent, bevel=0.04)
    # gable ends: prisms
    if along == "x":
        prof = [(y0, eave), (y1, eave), ((y0 + y1) / 2, ridge - 0.05)]
        for k, x in enumerate((x0, x1)):
            if gables[k]:
                prism(f"{name}_gable_{k}", [(py, pz) for py, pz in prof], 0.4, ((x + (0.2 if k == 0 else -0.2)), 0, 0),
                      wall_mat, parent, rot=(0, 0, math.radians(90)))
    else:
        prof = [(x0, eave), (x1, eave), ((x0 + x1) / 2, ridge - 0.05)]
        for k, y in enumerate((y0, y1)):
            if gables[k]:
                prism(f"{name}_gable_{k}", prof, 0.4, (0, y + (0.2 if k == 0 else -0.2), 0), wall_mat, parent)
    gable_roof(name + "_roof", x0, x1, y0, y1, eave, ridge, roof_mat, parent, along=along)


def window(name, at, facing, w, h, frame_mat, glass_mat, shutter_mat=None, parent=None, sill_mat=None):
    """A window on a wall face: `at` is the centre on the wall's outer plane, `facing` its outward normal (axis-aligned)."""
    fx, fy = facing
    rot = (0, 0, math.atan2(fy, fx))
    # local frame: +x out of the wall, y along the wall
    def L(dx, dy, dz):
        c, s = fx, fy
        return (at[0] + dx * c - dy * s, at[1] + dx * s + dy * c, at[2] + dz)
    box(name + "_glass", (0.04, w, h), L(0.0, 0, 0), glass_mat, parent, rot=rot, lods=(0, 1, 2))
    box(name + "_head", (0.12, w + 0.2, 0.12), L(0.04, 0, h / 2 + 0.06), frame_mat, parent, rot=rot, bevel=0.015, lods=(0, 1))
    box(name + "_sill", (0.16, w + 0.24, 0.07), L(0.06, 0, -h / 2 - 0.035), sill_mat or frame_mat, parent, rot=rot,
        bevel=0.015, lods=(0, 1))
    for s in (-1, 1):
        box(f"{name}_jamb_{'ab'[s > 0]}", (0.08, 0.1, h), L(0.03, s * (w / 2 + 0.05), 0), frame_mat, parent, rot=rot,
            lods=(0,))
    box(name + "_mullion", (0.05, 0.05, h), L(0.03, 0, 0), frame_mat, parent, rot=rot, lods=(0,))
    box(name + "_transom", (0.05, w, 0.05), L(0.03, 0, h * 0.18), frame_mat, parent, rot=rot, lods=(0,))
    if shutter_mat is not None:
        for s in (-1, 1):
            box(f"{name}_shutter_{'ab'[s > 0]}", (0.05, w / 2 + 0.02, h + 0.04), L(0.05, s * (w * 0.75 + 0.14), 0),
                shutter_mat, parent, rot=rot, bevel=0.01, lods=(0, 1))


def plank_door(name, at, facing, w, h, mat, parent=None, planks=True):
    fx, fy = facing
    rot = (0, 0, math.atan2(fy, fx))

    def L(dx, dy, dz):
        return (at[0] + dx * fx - dy * fy, at[1] + dx * fy + dy * fx, at[2] + dz)

    box(name, (0.06, w, h), L(0.02, 0, h / 2), mat, parent, rot=rot, bevel=0.01, lods=(0, 1, 2))
    if planks:
        n = max(2, int(w / 0.22))
        for i in range(1, n):
            y = -w / 2 + i * w / n
            box(f"{name}_gap_{i}", (0.02, 0.025, h - 0.1), L(0.055, y, h / 2), bpy_dark(), parent, rot=rot, lods=(0,))
        for z in (0.35, h - 0.35):
            box(f"{name}_brace_{z:.2f}", (0.04, w - 0.1, 0.12), L(0.07, 0, z), mat, parent, rot=rot, lods=(0, 1))


_DARK = []


def bpy_dark():
    if not _DARK:
        _DARK.append(flat_paint("gap_dark", (0.02, 0.018, 0.015), rough=1.0, grime=0.0))
    return _DARK[0]


def rubble(name, centre, radius, height, count, mat, seed, parent=None, lods=(0, 1, 2, 3)):
    """A heap of broken masonry: irregular chunks under a low mound."""
    rng = random.Random(seed)
    chunks = []
    for i in range(count):
        a = rng.random() * 2 * math.pi
        r = radius * math.sqrt(rng.random())
        h = height * (1 - (r / radius) ** 2) * (0.5 + 0.5 * rng.random())
        size = 0.15 + rng.random() * 0.45
        chunks.append((centre[0] + math.cos(a) * r, centre[1] + math.sin(a) * r, max(0.05, h - size * 0.3), size,
                       (rng.random() * 3, rng.random() * 3, rng.random() * 3), rng.random()))

    def mound(bm, lod):
        seg = (28, 14, 7, 5)[lod]
        rings = (8, 4, 2, 1)[lod]
        rr = random.Random(seed + 1)
        top = bm.verts.new((centre[0], centre[1], height * 0.8))
        prev = None
        first_ring = None
        rings_v = []
        for k in range(1, rings + 1):
            t = k / rings
            ring = []
            for j in range(seg):
                a = j * 2 * math.pi / seg
                wob = 1 + 0.18 * math.sin(a * 3 + seed) + 0.12 * (rr.random() - 0.5)
                rad = radius * t * wob
                # lumpy: broken masonry, not a smooth dome
                lump = 0.22 * height * (rr.random() - 0.35) if (t < 1 and lod < 2) else 0.0
                z = max(0.0, height * 0.8 * (1 - t ** 1.6) + lump)
                ring.append(bm.verts.new((centre[0] + math.cos(a) * rad, centre[1] + math.sin(a) * rad, z)))
            rings_v.append(ring)
        for j in range(seg):
            bm.faces.new((top, rings_v[0][j], rings_v[0][(j + 1) % seg]))
        for k in range(len(rings_v) - 1):
            a, b = rings_v[k], rings_v[k + 1]
            for j in range(seg):
                bm.faces.new((a[j], b[j], b[(j + 1) % seg], a[(j + 1) % seg]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        for f in bm.faces:
            f.normal_update()
            if f.normal.z < 0:
                f.normal_flip()

    mesh_part(name + "_mound", mound, mat, parent, lods)

    def pieces(bm, lod):
        keep = {0: 1.0, 1: 0.5, 2: 0.2}.get(lod, 0)
        made = False
        for i, (x, y, z, s, r, k) in enumerate(chunks):
            if k > keep:
                continue
            # a brick-like block: broken masonry and whole bricks among the dust
            sx, sy, sz = (s, s * (0.5 + k * 0.5), s * 0.45) if i % 3 else (0.24, 0.12, 0.07)
            m = Matrix.Translation((x, y, z)) @ Matrix.Rotation(r[0], 4, "X") @ Matrix.Rotation(r[1], 4, "Y") @ \
                Matrix.Rotation(r[2], 4, "Z") @ Matrix.Diagonal((sx, sy, sz, 1))
            res = bmesh.ops.create_cube(bm, size=1.0, matrix=m)
            made = True
        return made

    mesh_part(name + "_chunks", pieces, mat, parent, lods=(0, 1, 2))
