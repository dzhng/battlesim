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


def charred(name, seed=11.0, ember=0.3):
    """Burnt timber. `ember` is how far its edges go to rust-brown: little on a plain beam, which is all edge."""
    def fn(p, n, edge):
        c = lerp3((0.015, 0.014, 0.013), (0.09, 0.08, 0.07), 0.5 + 0.5 * fbm(p, 2.0, 3, seed))
        if edge > 0.3:
            c = lerp3(c, (0.2, 0.08, 0.03), ember * edge)  # embers' glow gone to rust-brown
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


def window(name, at, facing, w, h, frame_mat, glass_mat, shutter_mat=None, parent=None, sill_mat=None, bevel=0.015,
           glass_lods=(0, 1, 2), lights=2):
    """A window on a wall face: `at` is the centre on the wall's outer plane, `facing` its outward normal (axis-aligned).
    `lights` panes side by side. A kit's window, drawn hundreds of times, passes `bevel` 0 and keeps its glass in every tier."""
    fx, fy = facing
    rot = (0, 0, math.atan2(fy, fx))
    # local frame: +x out of the wall, y along the wall
    def L(dx, dy, dz):
        c, s = fx, fy
        return (at[0] + dx * c - dy * s, at[1] + dx * s + dy * c, at[2] + dz)
    box(name + "_glass", (0.04, w, h), L(0.0, 0, 0), glass_mat, parent, rot=rot, lods=glass_lods)
    box(name + "_head", (0.12, w + 0.2, 0.12), L(0.04, 0, h / 2 + 0.06), frame_mat, parent, rot=rot, bevel=bevel, lods=(0, 1))
    box(name + "_sill", (0.16, w + 0.24, 0.07), L(0.06, 0, -h / 2 - 0.035), sill_mat or frame_mat, parent, rot=rot,
        bevel=bevel, lods=(0, 1))
    for s in (-1, 1):
        box(f"{name}_jamb_{'ab'[s > 0]}", (0.08, 0.1, h), L(0.03, s * (w / 2 + 0.05), 0), frame_mat, parent, rot=rot,
            lods=(0,))
    for k in range(1, lights):
        box(name + ("_mullion" if lights == 2 else f"_mullion_{k}"), (0.05, 0.05, h), L(0.03, -w / 2 + k * w / lights, 0),
            frame_mat, parent, rot=rot, lods=(0,))
    box(name + "_transom", (0.05, w, 0.05), L(0.03, 0, h * 0.18), frame_mat, parent, rot=rot, lods=(0,))
    if shutter_mat is not None:
        for s in (-1, 1):
            box(f"{name}_shutter_{'ab'[s > 0]}", (0.05, w / 2 + 0.02, h + 0.04), L(0.05, s * (w * 0.75 + 0.14), 0),
                shutter_mat, parent, rot=rot, bevel=bevel and 0.01, lods=(0, 1))


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


def rubble(name, centre, radius, height, count, mat, seed, parent=None, lods=(0, 1, 2, 3), seg=(28, 14, 7, 5),
           rings=(8, 4, 2, 1)):
    """A heap of broken masonry: irregular chunks under a low mound. `seg` and `rings` are
    the mound's sides and rings at each tier: fewer for a heap drawn by the hundred."""
    mound_seg, mound_rings = seg, rings
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
        seg = mound_seg[lod]
        rings = mound_rings[lod]
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


# ---------------------------------------------------------------- ruins
def ragged_wall(name, a0, a1, fixed, along_x, lo, hi, seed, mat, parent=None, thickness=0.45, gaps=(), windows=0.0,
                openings=(), lods=TIERS, groups=(1, 2, 4, 8), jagged=1.0, run=(0.3, 0.9), buckle=0.0, raked=False):
    """A broken masonry wall from `a0` to `a1` along x (or y), `thickness` thick about the
    line `fixed`: runs of irregular width whose tops walk up and down in brick courses
    between `lo` and `hi`, with sudden breaks, loose top blocks, and holes through it:
    `gaps` (centre, width) knocked down to the rubble, `windows` a period of openings
    broken to their sills, `openings` (centre, width, sill) each broken to its own sill,
    with the wall cut at its jambs. A coarser tier merges `groups[tier]` runs into one;
    `jagged` scales how far the top walks. `run` is how wide a run is (wider for a wall
    of panels or sheets than of bricks), and `buckle` how far each stands out of line.
    `raked` slopes each run's top from its neighbour's height to its own where the step
    between them is small, so the wall's top is a broken line and not a row of battlements."""
    wr = random.Random(seed * 7919)
    course = 0.075
    cuts = sorted(c for o in openings for c in (o[0] - o[1] / 2, o[0] + o[1] / 2) if a0 + 0.05 < c < a1 - 0.05)
    runs, a, top = [], a0, (lo + hi) / 2
    while a < a1 - 0.05:
        w = min(wr.uniform(*run), a1 - a)
        ahead = [c - a for c in cuts if c - a > 1e-6]
        if ahead and ahead[0] < w + 0.15:  # a jamb: the run ends at it
            w = ahead[0]
        # a random walk pulled back towards the upper third, with sudden breaks
        top += wr.gauss(0, 0.28 * jagged) + 0.25 * (lo + 0.8 * (hi - lo) - top)
        if wr.random() < 0.08:
            top = lo + wr.uniform(0, 0.4)  # a break down towards the plinth
        top = max(lo, min(hi, top))
        mid = a + w / 2
        t = top
        for g, gw in gaps:
            if abs(mid - g) < gw / 2:
                t = min(t, 0.12 + 0.1 * wr.random())
        if windows > 0 and 0.3 < ((mid - a0) / windows) % 1.0 < 0.62:
            t = min(t, 0.95 + 0.08 * wr.random())
        for at, width, sill in openings:
            if abs(mid - at) < width / 2:
                t = min(t, sill + 0.08 * wr.random())
        t = max(course, round(t / course) * course)
        chip = (wr.uniform(0.1, 0.25), wr.uniform(0.3, 0.7), wr.choice((-1, 1))) if wr.random() < 0.45 and 0.5 < t < hi - 0.25 else None
        runs.append((a, a + w, t, chip, wr.uniform(-buckle, buckle) if buckle else 0.0))
        a += w

    def build(bm, lod):
        group = groups[lod]
        before = None  # the run before's height
        for i in range(0, len(runs), group):
            part = runs[i:i + group]
            s0, s1 = part[0][0], part[-1][1]
            t = sum(r[2] * (r[1] - r[0]) for r in part) / (s1 - s0)
            spans = [(s0, s1, t, None, part[0][4])] if group > 1 else part
            for r0, r1, tt, chip, out in spans:
                c = ((r0 + r1) / 2, fixed + out) if along_x else (fixed + out, (r0 + r1) / 2)
                size = (r1 - r0 + 0.01, thickness, tt) if along_x else (thickness, r1 - r0 + 0.01, tt)
                made = bmesh.ops.create_cube(bm, size=1.0, matrix=Matrix.Translation((c[0], c[1], tt / 2)) @ Matrix.Diagonal((*size, 1)))
                if raked and before is not None and 0.0 < abs(before - tt) <= 0.8:
                    for v in made["verts"]:  # its top's near end takes the neighbour's height
                        if v.co.z > tt / 2 and (v.co.x if along_x else v.co.y) < (r0 + r1) / 2:
                            v.co.z = before
                before = tt
                if chip and lod == 0:
                    ch, cw, side = chip
                    cs = (cw * (r1 - r0), thickness * 0.55, ch) if along_x else (thickness * 0.55, cw * (r1 - r0), ch)
                    off = (0, side * thickness * 0.2) if along_x else (side * thickness * 0.2, 0)
                    bmesh.ops.create_cube(bm, size=1.0, matrix=Matrix.Translation((c[0] + off[0], c[1] + off[1], tt + ch / 2)) @
                                          Matrix.Diagonal((*cs, 1)))

    mesh_part(name, build, mat, parent, lods)


def fill_height(x, y, rect, top, seed, low=0.3):
    """How high `rubble_fill`'s heap over `rect` stands at (x, y): what lies on it is placed by this.
    It is a mound: lowest against the walls, where they held the fall off, and lumpy all over."""
    x0, x1, y0, y1 = rect
    p = Vector((x, y, 0.0))
    inward = min(x - x0, x1 - x, y - y0, y1 - y)
    crown = 0.3 + 0.7 * smoothstep(0.0, min(3.0, 0.45 * min(x1 - x0, y1 - y0)), inward)
    n = 0.5 + 0.5 * fbm(p, 0.25, 2, 3.7 + seed)
    lump = 0.5 + 0.5 * fbm(p, 0.8, 2, 11.3 + seed)
    return top * (low + (1.0 - low) * min(1.0, max(0.0, crown * (0.45 + 0.5 * n + 0.4 * lump))))


FILL_RIM_M = 0.7  # the heap's outermost cells: the shade of the walls round it stays in them at every tier


def rubble_fill(name, x0, x1, y0, y1, top, mat, seed, parent=None, cells=(1.3, 2.6, 8.0), lods=(0, 1, 2), low=0.3, spill=0.0):
    """The mass of a collapsed building inside its walls: one lumpy mound over the
    rectangle, `low` of `top` at its rim and up to `top` in the middle. From its rim it
    falls to the ground `spill` metres further out (one number, or one for each of its
    west, east, south and north sides): a talus banked through the walls' gaps and against
    their feet. `cells` is the longest cell at each tier: the surface
    is the same heap, sampled coarser, always with a narrow band of cells at its rim.
    Its UVs are laid from above, so the recipe's fragments keep their size."""
    tile = textures.tile_of(TEXTURED[mat.name]) if mat.name in TEXTURED else 1.0

    def height(x, y):
        return fill_height(x, y, (x0, x1, y0, y1), top, seed, low)

    def stations(a, b, cell):
        """Where the grid's lines stand from `a` to `b`: the two ends, a rim band inside each, and even cells between."""
        if b - a < 2 * FILL_RIM_M + 0.6:
            n = max(1, math.ceil((b - a) / cell - 1e-6))
            return [a + (b - a) * i / n for i in range(n + 1)]
        n = max(1, math.ceil((b - a - 2 * FILL_RIM_M) / cell - 1e-6))
        return [a] + [a + FILL_RIM_M + (b - a - 2 * FILL_RIM_M) * i / n for i in range(n + 1)] + [b]

    def build(bm, lod):
        xs, ys = stations(x0, x1, cells[lod]), stations(y0, y1, cells[lod])
        nx, ny = len(xs) - 1, len(ys) - 1
        uv = bm.loops.layers.uv.new("UVMap")
        grid = [[bm.verts.new((x, y, height(x, y))) for x in xs] for y in ys]
        faces = [bm.faces.new((grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i])) for j in range(ny) for i in range(nx)]
        # the rim's talus, down to the ground a little further out
        rim = ([grid[0][i] for i in range(nx + 1)] + [grid[j][nx] for j in range(1, ny + 1)]
               + [grid[ny][i] for i in range(nx - 1, -1, -1)] + [grid[j][0] for j in range(ny - 1, 0, -1)])
        west, east, south, north = spill if isinstance(spill, tuple) else (spill,) * 4
        foot = [bm.verts.new((v.co.x + east * (v.co.x >= x1 - 1e-6) - west * (v.co.x <= x0 + 1e-6),
                              v.co.y + north * (v.co.y >= y1 - 1e-6) - south * (v.co.y <= y0 + 1e-6), 0.0)) for v in rim]
        for k in range(len(rim)):
            n = (k + 1) % len(rim)
            faces.append(bm.faces.new((rim[n], rim[k], foot[k], foot[n])))
        for f in faces:
            for loop in f.loops:
                loop[uv].uv = ((loop.vert.co.x + 0.3 * loop.vert.co.z) / tile, (loop.vert.co.y + 0.3 * loop.vert.co.z) / tile)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    return mesh_part(name, build, mat, parent, lods)


def scorched(mat, top, seed=0.0, floor=0.0):
    """`mat` after the fire, as a material of its own (`<name>_burnt`): its paint with soot
    over it, rising with the smoke from the building's own colour at the foot to black
    by `top` metres, where the wall broke, further up some stretches than others.
    `floor` is the least soot anywhere on it (a fallen roof is sooted all over). A
    textured material keeps its recipe, its tint mask and its wear."""
    name = mat.name + "_burnt"
    if name in bpy.data.materials:
        return bpy.data.materials[name]
    base = PAINTS[mat.name]
    twin = mat.copy()
    twin.name = name

    def soot(c, p):
        rise = smoothstep(0.3 * top, 0.95 * top, p.z)
        patch = 0.5 + 0.5 * fbm(p, 1.4, 2, 23.0 + seed)
        k = min(0.96, max(floor * (0.6 + 0.8 * patch), rise * (0.45 + 0.75 * patch)))
        return lerp3(c, (0.012, 0.011, 0.01), k)

    if mat.name in TEXTURED:
        TEXTURED[name] = TEXTURED[mat.name]
        PAINTS[name] = lambda p, n, edge: (lambda c, wear: (soot(c, p), wear))(*base(p, n, edge))
    else:
        PAINTS[name] = lambda p, n, edge: soot(base(p, n, edge), p)
    return twin


def weathered_roof(mat, seed=0.0, patch=0.16, moss=0.2):
    """Lay a roof's own weather over its material's paint: tone that drifts in soft patches
    some metres across, and moss or soot greying the dampest of them. The fields are wider
    than two of the finest tier's cells, so no tier shows the mesh's grid; the recipe
    carries what is smaller than a tile, and the roof what is larger."""
    base = PAINTS[mat.name]

    def fn(p, n, edge):
        c, wear = base(p, n, edge)
        if n.z < 0.2:
            return c, wear
        drift = fbm(p, 0.16, 2, 71.0 + seed)
        damp = smoothstep(0.1, 0.6, fbm(p, 0.11, 2, 83.0 + seed))
        c = tuple(x * (1.0 + patch * drift) for x in c)
        return lerp3(c, tuple(0.5 * (x + y) for x, y in zip(c, (0.07, 0.075, 0.055))), moss * damp), max(wear, 0.5 * moss * damp)

    PAINTS[mat.name] = fn
    return mat


# ---------------------------------------------------------------- town houses
# Textured builders for buildings drawn by the hundred: a wall is one face and
# the look comes from its material's recipe, so a house stays a few hundred triangles.
def _face(bm, points, toward):
    """A face through `points` whose normal leans `toward`."""
    f = bm.faces.new([bm.verts.new(p) for p in points])
    f.normal_update()
    if f.normal.dot(Vector(toward)) < 0:
        f.normal_flip()
    return f


def _ring(points):
    """`points` without the repeats a degenerate edge leaves (a gable has no hip, a pyramid no ridge)."""
    out = []
    for p in points:
        if not out or (Vector(p) - Vector(out[-1])).length > 1e-6:
            out.append(p)
    if len(out) > 1 and (Vector(out[0]) - Vector(out[-1])).length < 1e-6:
        out.pop()
    return out


class RoofShape:
    """A pitched roof over a rectangle: the walls reach `eave` and the ridge runs
    `along` x or y at `ridge`. `hips` says, for the ridge's low and high end, how
    much of that end is hipped: 0 a gable, 1 a full hip, between them a half-hip
    clipping the gable's top. Every slope has the one pitch. Eaves overhang by
    `over`, gable verges by `verge` (one per end; 0 where a neighbour's roof carries on)."""

    def __init__(self, x0, x1, y0, y1, eave, ridge, along="x", hips=(0.0, 0.0), over=0.35, verge=(0.25, 0.25)):
        self.swap = along == "y"
        self.a0, self.a1, self.b0, self.b1 = (y0, y1, x0, x1) if self.swap else (x0, x1, y0, y1)
        self.eave, self.ridge, self.hips = eave, ridge, hips
        self.bc, self.half = (self.b0 + self.b1) / 2, (self.b1 - self.b0) / 2
        self.tan = (ridge - eave) / self.half
        self.reach = self.half + over  # from the ridge line out to the eave's edge
        self.edge_z = eave - over * self.tan
        self.ends = [self.a0 - (over if hips[0] >= 1 else verge[0]), self.a1 + (over if hips[1] >= 1 else verge[1])]
        self.inset = [hips[0] * self.reach, hips[1] * self.reach]
        if self.ends[1] - self.inset[1] < self.ends[0] + self.inset[0] - 1e-9:
            raise SystemExit("a roof's hips meet before its ridge: lower `hips` or lengthen it")

    def world(self, a, b, z):
        return (b, a, z) if self.swap else (a, b, z)

    def on_slope(self, a, side, t):
        """A point on the low (-1) or high (+1) side's slope: `t` 0 at the eave's edge, 1 at the ridge."""
        return self.world(a, self.bc + side * self.reach * (1 - t), self.edge_z + (self.ridge - self.edge_z) * t)

    def planes(self):
        """The top surface: the two slopes, and an end slope under each hip."""
        A, d, h = self.ends, self.inset, self.hips
        out = [_ring([self.on_slope(A[0], s, 0), self.on_slope(A[1], s, 0), self.on_slope(A[1], s, 1 - h[1]),
                      self.on_slope(A[1] - d[1], s, 1), self.on_slope(A[0] + d[0], s, 1), self.on_slope(A[0], s, 1 - h[0])])
               for s in (-1, 1)]
        for k, inward in ((0, 1), (1, -1)):
            if h[k] > 0:
                out.append([self.on_slope(A[k], -1, 1 - h[k]), self.on_slope(A[k], 1, 1 - h[k]),
                            self.on_slope(A[k] + inward * d[k], 1, 1)])
        return out

    def crests(self):
        """The ridge and each hip, as segments: where the capping tiles run."""
        A, d, h = self.ends, self.inset, self.hips
        tops = [self.on_slope(A[0] + d[0], 1, 1), self.on_slope(A[1] - d[1], 1, 1)]
        out = [tuple(tops)] if (Vector(tops[0]) - Vector(tops[1])).length > 1e-6 else []
        for k in (0, 1):
            if h[k] > 0:
                out += [(self.on_slope(A[k], s, 1 - h[k]), tops[k]) for s in (-1, 1)]
        return out

    def end_wall_top(self, k, drop):
        """End wall `k`'s upper outline as (b, z), from its high-b corner to its low-b one:
        a gable's triangle, a half-hip's clipped one, or a hip's level top."""
        top = self.eave - drop
        cut = (self.edge_z + (self.ridge - self.edge_z) * (1 - self.hips[k])
               + self.tan * abs(self.ends[k] - (self.a0, self.a1)[k]))
        if cut <= self.eave + 1e-6:
            return [(self.b1, top), (self.b0, top)]
        if cut >= self.ridge:
            return [(self.b1, top), (self.bc, self.ridge - drop), (self.b0, top)]
        w = (self.ridge - cut) / self.tan
        return [(self.b1, top), (self.bc + w, cut - drop), (self.bc - w, cut - drop), (self.b0, top)]


def house_walls(name, shape, mat, parent=None, sides=("a0", "a1", "b0", "b1"), base=0.0, lods=TIERS, bands=None):
    """The outer walls under `shape`, one face each, from `base` to the roof's underside:
    `a0` and `a1` are the end walls (gabled as the roof says), `b0` and `b1` the walls
    under the eaves. Leave out a side that stands against a neighbour. `bands` (foot,
    head) cuts each wall in three up its height: its foot, its body and the head under
    the eaves. The ground's mud and shade then stay in the foot and the eaves' shade in
    the head at every tier, so the wall is one colour near and far."""
    drop = 0.03

    def build(bm, lod):
        top = shape.eave - drop
        levels = [base, top] if not bands else [base, base + bands[0], top - bands[1], top]
        for key, b, out in (("b0", shape.b0, -1), ("b1", shape.b1, 1)):
            for z0, z1 in zip(levels, levels[1:]) if key in sides else ():
                _face(bm, [shape.world(shape.a0, b, z0), shape.world(shape.a1, b, z0), shape.world(shape.a1, b, z1),
                           shape.world(shape.a0, b, z1)], shape.world(0, out, 0))
        for k, (key, a, out) in enumerate((("a0", shape.a0, -1), ("a1", shape.a1, 1))):
            if key in sides:
                for z0, z1 in zip(levels[:-2], levels[1:-1]):
                    _face(bm, [shape.world(a, b, z) for b, z in ((shape.b0, z0), (shape.b1, z0), (shape.b1, z1), (shape.b0, z1))],
                          shape.world(out, 0, 0))
                outline = [(shape.b0, levels[-2]), (shape.b1, levels[-2])] + shape.end_wall_top(k, drop)
                _face(bm, [shape.world(a, b, z) for b, z in outline], shape.world(out, 0, 0))

    return mesh_part(name, build, mat, parent, lods)


def pitched_roof(name, shape, mat, trim, parent=None, thickness=0.14, fascia_ends=(True, True), lods=TIERS):
    """`shape`'s roof in `mat`, with its own UVs (u along the eave, v up the slope, in
    metres over the recipe's tile) so the courses lie level on every slope. The fascia
    and bargeboards, the soffit and the capping tiles drop out with distance.
    `fascia_ends` drops an end's boards where the next roof carries straight on."""
    tile = textures.tile_of(TEXTURED[mat.name]) if mat.name in TEXTURED else 1.0
    planes = shape.planes()
    up = Vector((0, 0, 1))

    def tiles_(bm, lod):
        uv = bm.loops.layers.uv.new("UVMap")
        for poly in planes:
            f = _face(bm, poly, up)
            u = up.cross(f.normal).normalized()
            v = f.normal.cross(u)
            for loop in f.loops:
                loop[uv].uv = (loop.vert.co.dot(u) / tile, loop.vert.co.dot(v) / tile)

    mesh_part(name + "_tiles", tiles_, mat, parent, lods)

    count = {}
    for poly in planes:
        for i, p in enumerate(poly):
            key = tuple(sorted((tuple(round(c, 5) for c in p), tuple(round(c, 5) for c in poly[(i + 1) % len(poly)]))))
            count[key] = count.get(key, 0) + 1
    rim = [key for key, n in sorted(count.items()) if n == 1]
    centre = Vector(shape.world((shape.a0 + shape.a1) / 2, shape.bc, 0))

    def end_of(p):
        a = p[1] if shape.swap else p[0]
        return next((k for k in (0, 1) if abs(a - shape.ends[k]) < 1e-4), None)

    def fascia(bm, lod):
        for p, q in rim:
            if end_of(p) is not None and end_of(p) == end_of(q) and not fascia_ends[end_of(p)]:
                continue
            p, q = Vector(p), Vector(q)
            out = Vector((q.y - p.y, p.x - q.x, 0))
            if out.dot((p + q) / 2 - centre) < 0:
                out = -out
            _face(bm, [p, q, q - up * thickness, p - up * thickness], out)

    mesh_part(name + "_fascia", fascia, trim, parent, lods=tuple(t for t in lods if t < 3))

    def soffit(bm, lod):
        for poly in planes:
            _face(bm, [(x, y, z - thickness) for x, y, z in poly], -up)

    mesh_part(name + "_soffit", soffit, trim, parent, lods=tuple(t for t in lods if t < 2))

    def caps(bm, lod):
        uv = bm.loops.layers.uv.new("UVMap")
        for p, q in shape.crests():
            p, q = Vector(p), Vector(q)
            run = (q - p).normalized()
            across = run.cross(up).normalized()
            for side in (-1, 1):
                f = _face(bm, [p + side * 0.15 * across - 0.1 * up, q + side * 0.15 * across - 0.1 * up, q + 0.07 * up,
                               p + 0.07 * up], side * across + up)
                for loop in f.loops:
                    loop[uv].uv = (loop.vert.co.dot(run) / tile, side * (loop.vert.co - p).dot(across) / tile)

    if shape.crests():
        mesh_part(name + "_caps", caps, mat, parent, lods=tuple(t for t in lods if t < 2))


WALL_BANDS_M = (1.1, 0.6)  # a house wall's foot and head, cut off its body (`house_walls`)


def house_shell(name, shape, wall, roof, trim, parent=None, sides=("a0", "a1", "b0", "b1"), fascia_ends=(True, True),
                plinth=None, plinth_mat=None, plinth_m=0.4):
    """Walls, roof and plinth of one box of a building: `<name>_walls`, `<name>_roof` and
    `<name>_plinth`. `roof` None leaves the box to a neighbour's roof; `plinth` is the
    plinth's (x0, x1, y0, y1), `plinth_m` high."""
    house_walls(name + "_walls", shape, wall, parent, sides, bands=WALL_BANDS_M)
    if roof is not None and shape.ends[1] - shape.ends[0] > 1e-6:
        pitched_roof(name + "_roof", shape, roof, trim, parent, fascia_ends=fascia_ends)
    if plinth:
        x0, x1, y0, y1 = plinth
        box(name + "_plinth", (x1 - x0 + 0.08, y1 - y0 + 0.08, plinth_m), ((x0 + x1) / 2, (y0 + y1) / 2, plinth_m / 2), plinth_mat,
            parent, lods=(0, 1, 2))


def chimney(name, size, height, mat, cap_mat, parent=None, pots=1, pot_mat=None):
    """A chimney stack standing on the origin: a capped stack with clay pots. The cap
    is in every tier: from above it is the chimney's colour."""
    box(name + "_stack", (size[0], size[1], height), (0, 0, height / 2), mat, parent)
    box(name + "_cap", (size[0] + 0.12, size[1] + 0.12, 0.07), (0, 0, height + 0.035), cap_mat, parent)
    for k in range(pots):
        x = (k - (pots - 1) / 2) * 0.32
        cyl(f"{name}_pot_{k}", 0.085, 0.2, (x, 0, height + 0.17), "Z", pot_mat or cap_mat, parent, seg=10, r2=0.07, lods=(0, 1))


def panel_door(name, w, h, leaf_mat, frame_mat, parent=None, glass_mat=None, light=0.0):
    """A panelled front door facing -Y, its threshold's centre on the wall plane at the
    origin; `light` metres of glazed fanlight over it."""
    box(name + "_leaf", (w, 0.05, h), (0, -0.025, h / 2), leaf_mat, parent)
    top = h + light + (0.06 if light else 0.0)
    box(name + "_head", (w + 0.24, 0.1, 0.1), (0, -0.05, top + 0.05), frame_mat, parent, lods=(0, 1))
    for s in (-1, 1):
        box(f"{name}_jamb_{'ab'[s > 0]}", (0.1, 0.1, top), (s * (w / 2 + 0.05), -0.05, top / 2), frame_mat, parent, lods=(0, 1))
        for k, (z0, z1) in enumerate(((0.2, 0.95), (1.1, h - 0.18))):
            box(f"{name}_panel_{'ab'[s > 0]}_{k}", (w * 0.33, 0.02, z1 - z0), (s * w * 0.22, -0.06, (z0 + z1) / 2), leaf_mat,
                parent, lods=(0,))
    if light:
        box(name + "_bar", (w, 0.08, 0.06), (0, -0.04, h + 0.03), frame_mat, parent, lods=(0, 1))
        box(name + "_light", (w, 0.04, light), (0, -0.02, h + 0.06 + light / 2), glass_mat, parent, lods=(0, 1, 2))


# ---------------------------------------------------------------- farm buildings
class LeanToShape:
    """One slope over a rectangle, for `pitched_roof`: the wall on the side named by
    `rises` (north, south, east or west) reaches `high`, the wall across from it `low`.
    The slope overhangs the low wall by `over`, the high wall by `over_top` and each
    end by `verge`; `top` is its highest point."""

    def __init__(self, x0, x1, y0, y1, low, high, rises="north", over=0.3, over_top=0.2, verge=(0.2, 0.2)):
        self.swap = rises in ("east", "west")
        self.a0, self.a1, self.b0, self.b1 = (y0, y1, x0, x1) if self.swap else (x0, x1, y0, y1)
        up = 1 if rises in ("north", "east") else -1
        self.bc, self.eave = (self.b0 + self.b1) / 2, low
        self.tan = (high - low) / (self.b1 - self.b0)
        self.ends = [self.a0 - verge[0], self.a1 + verge[1]]
        low_b, high_b = (self.b0, self.b1) if up > 0 else (self.b1, self.b0)
        self.foot = (low_b - up * over, low - over * self.tan)
        self.head = (high_b + up * over_top, high + over_top * self.tan)
        self.edge_z, self.top = self.foot[1], self.head[1]

    def world(self, a, b, z):
        return (b, a, z) if self.swap else (a, b, z)

    def planes(self):
        return [[self.world(self.ends[0], *self.foot), self.world(self.ends[1], *self.foot),
                 self.world(self.ends[1], *self.head), self.world(self.ends[0], *self.head)]]

    def crests(self):
        return []


def timber_frame(name, shape, mat, parent=None, posts=None, openings=None, foot=0.6, rail=2.5, width=0.2, proud=0.04,
                 lods=TIERS):
    """Exposed framing over the walls under `shape` (a `RoofShape`): a sill at `foot`, a
    rail at `rail`, a plate under the eaves, corner posts and `posts`, braced at each
    corner; a gable's posts run up to the roof under a collar. `posts` and `openings`
    are per wall (`a0`, `a1`, `b0`, `b1`): where the posts stand along it, and each
    opening as (from, to, head) along it: the sill stops at one, and so does the rail
    when the head is over it. A timber is a board `proud` of the wall, with edges on
    the finest tier; the two coarse tiers drop the braces, which are a pixel wide there and crawl."""
    posts, openings = posts or {}, openings or {}
    plate = shape.eave - 0.03 - width / 2

    def members(key):
        gable = key[0] == "a"
        s0, s1 = (shape.b0, shape.b1) if gable else (shape.a0, shape.a1)
        outline = sorted(shape.end_wall_top(int(key[1]), 0.0)) if gable else []

        def roof(s):  # the roof's underside over a gable's post
            for (sa, za), (sb, zb) in zip(outline, outline[1:]):
                if sa <= s <= sb and sb > sa:
                    return za + (zb - za) * (s - sa) / (sb - sa) - 0.2
            return plate

        holes = sorted(openings.get(key, ()))
        stand = sorted({s0 + width / 2, s1 - width / 2, *posts.get(key, ())})
        out = [((s, foot), (s, max(plate, roof(s)))) for s in stand]
        at = s0
        for lo, hi, _ in holes + [(s1, s1, 0.0)]:  # the sill, between the openings
            if lo - at > 1e-6:
                out.append(((at, foot), (lo, foot)))
            at = hi
        out.append(((s0, plate), (s1, plate)))
        for k, (a, b) in enumerate(zip(stand, stand[1:])):
            mid = (a + b) / 2
            opening = [head for lo, hi, head in holes if lo < mid < hi]
            if not any(head > rail for head in opening):
                out.append(((a, rail), (b, rail)))
            end = -1 if k == 0 else 1 if k == len(stand) - 2 else 0
            if end and not opening:
                corner, reach = (a, min(b - a, 1.5)) if end < 0 else (b, -min(b - a, 1.5))
                out += [((corner + reach, foot), (corner, rail)), ((corner, rail), (corner + reach, plate))]
        if gable and shape.ridge - shape.eave > 1.5:
            z = shape.eave + 0.55 * (shape.ridge - shape.eave)
            reach = (roof(shape.bc) + 0.2 - z) / shape.tan - 0.25
            if reach > 0.3:
                out.append(((shape.bc - reach, z), (shape.bc + reach, z)))
        return out

    walls = {"b0": (shape.b0, -1), "b1": (shape.b1, 1), "a0": (shape.a0, -1), "a1": (shape.a1, 1)}

    def build(bm, lod):
        for key, (at, out) in sorted(walls.items()):
            def point(s, z, lift):
                return Vector(shape.world(at + out * lift, s, z) if key[0] == "a" else shape.world(s, at + out * lift, z))

            normal = Vector(shape.world(out, 0, 0) if key[0] == "a" else shape.world(0, out, 0))
            for (ps, pz), (qs, qz) in members(key):
                if lod >= 2 and abs(qs - ps) > 1e-6 and abs(qz - pz) > 1e-6:
                    continue
                run = Vector((qs - ps, qz - pz)).normalized()
                ds, dz = -run.y * width / 2, run.x * width / 2
                ring = [(ps - ds, pz - dz), (qs - ds, qz - dz), (qs + ds, qz + dz), (ps + ds, pz + dz)]
                _face(bm, [point(s, z, proud) for s, z in ring], normal)
                if lod == 0:
                    for (sa, za), (sb, zb) in zip(ring, ring[1:] + ring[:1]):
                        away = point((sa + sb) / 2, (za + zb) / 2, 0) - point((ps + qs) / 2, (pz + qz) / 2, 0)
                        _face(bm, [point(sa, za, 0), point(sb, zb, 0), point(sb, zb, proud), point(sa, za, proud)], away)

    return mesh_part(name, build, mat, parent, lods)


def wall_panels(name, panels, mat, parent=None, proud=0.03, lods=TIERS):
    """Flat panels on walls, one quad each: what a far tier keeps of a window or a door.
    A panel is (x, y, z, yaw, w, h): the centre of its foot on the wall plane, turned
    about +Z as a wall fitting is (it faces -Y before the turn)."""

    def build(bm, lod):
        for x, y, z, yaw, w, h in panels:
            c, s = math.cos(yaw), math.sin(yaw)
            _face(bm, [(x + dx * c + proud * s, y + dx * s - proud * c, z + dz)
                       for dx, dz in ((-w / 2, 0), (w / 2, 0), (w / 2, h), (-w / 2, h))], (s, -c, 0))

    return mesh_part(name, build, mat, parent, lods)


def barn_doors(name, w, h, leaf_mat, frame_mat, parent=None, hoist=0.0):
    """A barn's double doors facing -Y, the threshold's centre on the wall plane at the
    origin: two ledged and braced plank leaves between posts under a heavy lintel.
    `hoist` metres of beam stand out over a loft's doors."""
    half = w / 2
    ledges = (0.12 * h, 0.5 * h, 0.88 * h)
    for s in (-1, 1):
        tag = "ab"[s > 0]
        box(f"{name}_leaf_{tag}", (half - 0.02, 0.06, h), (s * half / 2, -0.03, h / 2), leaf_mat, parent)
        box(f"{name}_post_{tag}", (0.18, 0.12, h), (s * (half + 0.09), -0.06, h / 2), frame_mat, parent, lods=(0, 1))
        for k, z in enumerate(ledges):
            box(f"{name}_ledge_{tag}_{k}", (half - 0.12, 0.03, 0.14), (s * half / 2, -0.075, z), leaf_mat, parent, lods=(0, 1))
        for k, (z0, z1) in enumerate(zip(ledges, ledges[1:])):
            run, rise = half - 0.2, z1 - z0 - 0.14
            box(f"{name}_brace_{tag}_{k}", (math.hypot(run, rise), 0.03, 0.12), (s * half / 2, -0.075, (z0 + z1) / 2), leaf_mat,
                parent, rot=(0, s * math.atan2(rise, run), 0), lods=(0,))
    box(name + "_lintel", (w + 0.6, 0.16, 0.26), (0, -0.08, h + 0.13), frame_mat, parent, lods=(0, 1, 2))
    box(name + "_gap", (0.04, 0.02, h), (0, -0.07, h / 2), bpy_dark(), parent, lods=(0, 1))
    if hoist:
        box(name + "_hoist", (0.16, hoist, 0.18), (0, -hoist / 2, h + 0.5), frame_mat, parent, lods=(0, 1, 2))


def stable_door(name, w, h, leaf_mat, frame_mat, parent=None):
    """A split stable door facing -Y, the threshold's centre on the wall plane at the
    origin: the lower leaf shut, the upper swung in and the stall dark behind it."""
    low = h * 0.55
    box(name + "_leaf", (w, 0.05, low), (0, -0.025, low / 2), leaf_mat, parent)
    box(name + "_dark", (w, 0.03, h - low), (0, -0.015, (h + low) / 2), bpy_dark(), parent)
    box(name + "_head", (w + 0.36, 0.12, 0.16), (0, -0.06, h + 0.08), frame_mat, parent, lods=(0, 1, 2))
    for s in (-1, 1):
        box(f"{name}_jamb_{'ab'[s > 0]}", (0.12, 0.1, h), (s * (w / 2 + 0.06), -0.05, h / 2), frame_mat, parent, lods=(0, 1))
    for k, z in enumerate((0.25, low - 0.2)):
        box(f"{name}_ledge_{k}", (w - 0.08, 0.03, 0.12), (0, -0.065, z), leaf_mat, parent, lods=(0,))


def farm_cart(name, wood, iron, parent=None):
    """A two-wheeled farm cart standing on the origin, its shafts resting on the ground toward -Y."""
    box(name + "_bed", (1.3, 2.6, 0.1), (0, 0.3, 0.9), wood, parent)
    for s in (-1, 1):
        tag = "ab"[s > 0]
        box(f"{name}_side_{tag}", (0.05, 2.6, 0.42), (s * 0.65, 0.3, 1.16), wood, parent, lods=(0, 1, 2))
        box(f"{name}_end_{tag}", (1.3, 0.05, 0.42), (0, 0.3 + s * 1.3, 1.16), wood, parent, lods=(0, 1))
        cyl(f"{name}_wheel_{tag}", 0.6, 0.09, (s * 0.8, 0.4, 0.63), "X", wood, parent, seg=16, lods=(0, 1, 2))
        cyl(f"{name}_tyre_{tag}", 0.62, 0.05, (s * 0.8, 0.4, 0.63), "X", iron, parent, seg=16, lods=(0,), caps=False)
        cyl(f"{name}_hub_{tag}", 0.12, 0.2, (s * 0.8, 0.4, 0.63), "X", iron, parent, seg=8, lods=(0,))
        box(f"{name}_shaft_{tag}", (0.08, 2.1, 0.08), (s * 0.5, -1.95, 0.47), wood, parent, rot=(0.4, 0, 0), lods=(0, 1))
    cyl(name + "_axle", 0.05, 1.6, (0, 0.4, 0.63), "X", iron, parent, seg=6, lods=(0,))


def bale_stack(name, mat, parent=None, across=3, high=3, seed=3):
    """Straw bales stacked one deep against a wall facing -Y (the wall plane at y = 0),
    centred on the origin: `high` courses, the top one a bale short."""
    rng = random.Random(seed)
    w, d, h = 0.95, 0.48, 0.4
    for k in range(high):
        count = across - (k == high - 1 and high > 1)
        for i in range(count):
            x = (i - (count - 1) / 2) * w + rng.uniform(-0.04, 0.04)
            box(f"{name}_{k}_{i}", (w - 0.04, d - 0.03, h - 0.02), (x, -d / 2 - 0.03, (k + 0.5) * h), mat, parent,
                rot=(0, 0, rng.uniform(-0.05, 0.05)), lods=(0, 1))
    box(name + "_mass", (across * w, d, (high - 0.5) * h), (0, -d / 2 - 0.03, (high - 0.5) * h / 2), mat, parent, lods=(2, 3))


def woodpile(name, w, h, bark, cut, parent=None, seed=5):
    """Split logs stacked against a wall facing -Y (the wall plane at y = 0), their cut ends outward."""
    rng = random.Random(seed)
    box(name + "_stack", (w, 0.42, h), (0, -0.23, h / 2), bark, parent)
    pitch = 0.26
    for k in range(int(h / pitch)):
        for i in range(int(w / pitch)):
            x = -w / 2 + (i + 0.5) * (w / int(w / pitch)) + rng.uniform(-0.03, 0.03)
            cyl(f"{name}_log_{k}_{i}", rng.uniform(0.09, 0.125), 0.06, (x, -0.455, 0.15 + k * pitch + rng.uniform(-0.02, 0.02)), "Y",
                cut, parent, seg=6, lods=(0,))
    for s in (-1, 1):
        box(f"{name}_stake_{'ab'[s > 0]}", (0.08, 0.08, h + 0.25), (s * (w / 2 + 0.05), -0.3, (h + 0.25) / 2), bark, parent, lods=(0, 1))


def stone_trough(name, stone_mat, water_mat, parent=None, w=1.7, d=0.5, h=0.5):
    """A stone water trough against a wall facing -Y (the wall plane at y = 0)."""
    box(name + "_block", (w, d, h - 0.08), (0, -d / 2 - 0.03, (h - 0.08) / 2), stone_mat, parent)
    box(name + "_water", (w - 0.16, d - 0.16, 0.02), (0, -d / 2 - 0.03, h - 0.07), water_mat, parent, lods=(0, 1))
    for s in (-1, 1):
        box(f"{name}_rim_x{'ab'[s > 0]}", (0.09, d, 0.08), (s * (w / 2 - 0.045), -d / 2 - 0.03, h - 0.04), stone_mat, parent, lods=(0, 1))
        box(f"{name}_rim_y{'ab'[s > 0]}", (w - 0.18, 0.09, 0.08), (0, -d / 2 - 0.03 + s * (d / 2 - 0.045), h - 0.04), stone_mat, parent,
            lods=(0, 1))


def water_butt(name, wood, iron, water_mat, parent=None, r=0.27, h=0.95):
    """A rain barrel standing on the origin."""
    cyl(name + "_staves", r, h, (0, 0, h / 2), "Z", wood, parent, seg=14)
    cyl(name + "_water", r - 0.03, 0.02, (0, 0, h + 0.005), "Z", water_mat, parent, seg=14, lods=(0, 1))
    for k, z in enumerate((0.2, h - 0.2)):
        cyl(f"{name}_hoop_{k}", r + 0.012, 0.05, (0, 0, z), "Z", iron, parent, seg=14, lods=(0,), caps=False)
