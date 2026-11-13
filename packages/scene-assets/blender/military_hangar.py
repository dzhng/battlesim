"""The hangar and its ruin (`fixtures/props/military/works.json`), for `military.py`."""
import bpy, bmesh, math, random
from mathutils import Euler, Matrix, Vector, noise

from parts import *
import parts
from military_parts import *


# ------------------------------------------------------------------ the hangar
# An arch hangar: one barrel vault of corrugated steel sheet, 16 m across and 8 m high, on
# a concrete curb down each long side, with flat end walls. The corrugations run over it
# from curb to curb, fine as a Quonset's, so it reads as one shell from afar.
CURB_W, CURB_H = 0.36, 0.3  # each long side's concrete curb
HALF_SPAN = 7.82  # the vault's half width at its foot, on the curb's middle
RISE = 7.62  # the vault's height above the curb (its top at the box's 8 m less a rib)
SHEET_U_M = 1.2  # a `roof_sheet` tile spans 16 corrugations: 75 mm apart along the hangar
SHEET_V_M = 6.0  # and 6 m round the vault
RECESS = 0.5  # how far each end wall stands in under the vault's edge
OLIVE = (0.098, 0.102, 0.072)  # a faded olive-drab paint, chalked by the sun


def vault(t):
    """The vault's outer line at `t` (0 at the +Y foot, 1 at the -Y foot): (y, z)."""
    a = math.pi * t
    return HALF_SPAN * math.cos(a), CURB_H + RISE * math.sin(a)


def vault_length(t, steps=96):
    """The arc's length from the +Y foot to `t`, metres."""
    s, last = 0.0, vault(0.0)
    for k in range(1, steps + 1):
        p = vault(t * k / steps)
        s += math.hypot(p[0] - last[0], p[1] - last[1])
        last = p
    return s


def hangar_materials():
    return dict(
        sheet=textured("hangar_sheet", "roof_sheet", colour=OLIVE, chip=0.4, dirt=0.35, rise=0.8, streak=0.5,
                       mottle=0.15, grain=0.02),
        wall=textured("hangar_wall", "cladding", colour=(0.092, 0.096, 0.07), chip=0.5, dirt=0.35, rise=0.6,
                      streak=0.6, grain=0.02),
        steel=textured("hangar_steel", "enamel", colour=(0.045, 0.05, 0.035), chip=0.6, dirt=0.7, rise=0.8),
        concrete=textured("hangar_curb", "concrete", chip=0.6, dirt=0.9, rise=0.3, mottle=0.2),
        glass=glass("hangar_glass"),
        skylight=flat_paint("hangar_skylight", (0.16, 0.17, 0.14), rough=0.45, grime=0.5),
        mark=flat_paint("hangar_mark", (0.42, 0.41, 0.36), rough=0.8, grime=0.6),
    )


def vault_sheet(name, m, parent, x0, x1):
    """The vault's corrugated sheet from `x0` to `x1`, its corrugations the texture's, which
    runs round the vault (its own UVs)."""
    arc = vault_length(1.0)

    def build(bm, lod):
        nu, nv = (24, 12, 6, 2)[lod], (56, 36, 20, 10)[lod]
        grid(bm, nu, nv, lambda u, v: (x0 + (x1 - x0) * u, *vault(v)),
             uv=lambda u, v: ((x0 + (x1 - x0) * u) / SHEET_U_M, arc * v / SHEET_V_M), flip=True)  # outward

    return mesh_part(name, build, m["sheet"], parent, smooth=True)


def end_wall_outline(inset, n):
    """The end wall's outline inside the vault, `inset` in from its outer line, as (y, z)
    from the +Y foot over to the -Y foot, `n` steps."""
    pts = []
    for k in range(n + 1):
        y, z = vault(k / n)
        a = math.pi * k / n
        pts.append((y - inset * math.cos(a), max(0.0, z - inset * math.sin(a))))
    pts[0] = (pts[0][0], 0.0)
    pts[-1] = (pts[-1][0], 0.0)
    return pts


def quad(bm, x, facing, ya, yb, lo, ta, tb):
    """Into `bm`: a wall piece at `x` from `ya` to `yb`, from `lo` up to `ta` and `tb`."""
    vs = [bm.verts.new((x, y, z)) for y, z in ((ya, lo), (yb, lo), (yb, tb), (ya, ta))]  # faces +X as listed
    bm.faces.new(vs if facing > 0 else vs[::-1])


def panels(name, m, parent, x, facing, holes):
    """The end wall as vertical strips between the openings' edges, each strip filled from
    the ground (or an opening's head) to the arch: one face per strip piece, so no face
    spans an opening. Every tier keeps the openings (their doors fill them)."""

    def build(bm, lod):
        n = (40, 24, 14, 8)[lod]
        outline = end_wall_outline(0.02, n)

        def top(y):  # the wall's top at `y`: the outline, interpolated
            for (ya, za), (yb, zb) in zip(outline, outline[1:]):
                lo, hi = min(ya, yb), max(ya, yb)
                if lo - 1e-9 <= y <= hi + 1e-9:
                    return za if hi == lo else za + (zb - za) * (y - ya) / (yb - ya)
            return 0.0

        cuts = sorted({round(y, 4) for y, _ in outline} | {round(v, 4) for h in holes for v in (h[0], h[1])})
        for ya, yb in zip(cuts, cuts[1:]):
            if yb - ya < 1e-4:
                continue
            ta, tb = top(ya), top(yb)
            # walk up the strip past each opening it crosses, then to the arch
            cur = 0.0
            for y0, y1, z0, z1 in sorted(h for h in holes if h[0] - 1e-6 <= ya and yb <= h[1] + 1e-6):
                if z0 > cur + 1e-4:
                    quad(bm, x, facing, ya, yb, cur, z0, z0)
                cur = max(cur, z1)
            if max(ta, tb) > cur + 1e-4:
                quad(bm, x, facing, ya, yb, cur, max(cur, ta), max(cur, tb))
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
        return True

    return mesh_part(name, build, m["wall"], parent)


def skylight(name, m, parent, x0, x1, v0, v1):
    """A translucent sheet in the vault from `x0` to `x1` and `v0` to `v1` round it, laid on
    the ribs' crowns."""

    def build(bm, lod):
        grid(bm, (8, 4, 2, 1)[lod], (4, 2, 1, 1)[lod],
             lambda u, v: (x0 + (x1 - x0) * u, *(c + 0.012 * d for c, d in zip(
                 vault(v0 + (v1 - v0) * v), (math.cos(math.pi * (v0 + (v1 - v0) * v)), math.sin(math.pi * (v0 + (v1 - v0) * v)))))),
             flip=True)
        return True

    return mesh_part(name, build, m["skylight"], parent)


def vault_trim(name, m, parent, x, w=0.14):
    """The flashing that caps the vault's edge over an end wall: a flat strip following
    the arch."""

    def build(bm, lod):
        n = (48, 28, 14, 8)[lod]
        rings = []
        for k in range(n + 1):
            t = k / n
            y, z = vault(t)
            ny, nz = math.cos(math.pi * t), math.sin(math.pi * t)
            out, inn = 0.02, -0.06
            rings.append([bm.verts.new((x + sx, y + r * ny, z + r * nz))
                          for sx, r in ((-w / 2, out), (w / 2, out), (w / 2, inn), (-w / 2, inn))])
        for a, b in zip(rings, rings[1:]):
            for i in range(4):
                j = (i + 1) % 4
                bm.faces.new((a[i], a[j], b[j], b[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        return True

    return mesh_part(name, build, m["steel"], parent)


@kind
def hangar():
    """A K-span arch hangar, 24 m by 16 m and 8 m high: the ribbed olive vault on its curbs,
    seam bands over it every 4 m, the end walls of vertical-ribbed sheet. The front (+X)
    is closed by two braced sliding leaves under a track beam, a wicket door in one and
    the hangar's number on the other, a lamp each side and a louvred vent under the apex;
    the back has a door and two windows."""
    hx, hy, hz = 12.0, 8.0, 4.0
    m = hangar_materials()
    shell = body("shell")
    x0, x1 = -hx + 0.04, hx - 0.04
    vault_sheet("vault", m, shell, x0, x1)
    for k, s in enumerate((-1, 1)):
        box(f"curb_{k}", (2 * hx, CURB_W, CURB_H), (0, s * HALF_SPAN, CURB_H / 2), m["concrete"], shell, bevel=0.02)
        box(f"curb_far_{k}", (2 * hx, CURB_W, CURB_H), (0, s * HALF_SPAN, CURB_H / 2), m["concrete"], shell,
            lods=())
    # seam bands where the arch's panels join, every 4 m
    for k, x in enumerate((-8.0, -4.0, 0.0, 4.0, 8.0)):
        vault_trim(f"seam_{k}", m, shell, x, w=0.12)
    # translucent sheets let into the vault down each side
    for k, x in enumerate((-7.0, -1.0, 5.0)):
        for j, (v0, v1) in enumerate(((0.2, 0.29), (0.71, 0.8))):
            skylight(f"skylight_{k}_{j}", m, shell, x, x + 1.0, v0, v1)
    for k, x in enumerate((x0, x1)):
        vault_trim(f"edge_{k}", m, shell, x + (0.03 if k == 0 else -0.03), w=0.08)
    # ---- the front: an opening 9.6 m wide and 5.2 m high, closed by two leaves
    door_w, door_h, xf = 9.6, 5.2, x1 - RECESS
    vent = (-0.9, 0.9, 6.2, 6.9)
    panels("front_wall", m, shell, xf, 1, [(-door_w / 2 - 0.15, door_w / 2 + 0.15, 0.0, door_h + 0.25), vent])
    bar("track_beam", (xf + 0.12, -door_w / 2 - 0.6, door_h + 0.38), (xf + 0.12, door_w / 2 + 0.6, door_h + 0.38),
        0.2, 0.26, m["steel"], shell)
    for k, s in enumerate((-1, 1)):
        y = s * (door_w / 2 + 0.08)
        box(f"jamb_{k}", (0.22, 0.16, door_h + 0.25), (xf + 0.05, y, (door_h + 0.25) / 2), m["steel"], shell)
        # a leaf: vertical-ribbed sheet in a steel frame, braced corner to corner
        cy = s * door_w / 4
        lx = xf + 0.16
        box(f"leaf_{k}", (0.08, door_w / 2 - 0.04, door_h - 0.05), (lx, cy, door_h / 2 + 0.02), m["wall"], shell)
        for j, (a, b) in enumerate((((-1, 0.08), (1, 0.08)), ((-1, door_h - 0.1), (1, door_h - 0.1)),
                                    ((-1, door_h / 2), (1, door_h / 2)))):
            bar(f"leaf_rail_{k}_{j}", (lx + 0.06, cy + a[0] * (door_w / 4 - 0.06), a[1]),
                (lx + 0.06, cy + b[0] * (door_w / 4 - 0.06), b[1]), 0.04, 0.14, m["steel"], shell, lods=(0, 1, 2))
        for j, z0 in enumerate((0.08, door_h / 2)):
            bar(f"leaf_brace_{k}_{j}", (lx + 0.06, cy - s * (door_w / 4 - 0.1), z0 + 0.06),
                (lx + 0.06, cy + s * (door_w / 4 - 0.1), z0 + door_h / 2 - 0.2), 0.04, 0.12, m["steel"], shell,
                lods=(0, 1))
        for j, sy in enumerate((-1, 1)):
            bar(f"leaf_stile_{k}_{j}", (lx + 0.06, cy + sy * (door_w / 4 - 0.06), 0.06),
                (lx + 0.06, cy + sy * (door_w / 4 - 0.06), door_h - 0.06), 0.04, 0.14, m["steel"], shell,
                lods=(0, 1, 2))
        # rollers hanging the leaf from the track
        for j, oy in enumerate((-1.6, 1.6)):
            box(f"hanger_{k}_{j}", (0.06, 0.12, 0.42), (xf + 0.17, cy + oy, door_h + 0.19), m["steel"], shell,
                lods=(0, 1))
        # a lamp each side, over the jambs
        box(f"lamp_arm_{k}", (0.3, 0.05, 0.05), (xf + 0.15, s * (door_w / 2 + 0.7), door_h + 0.95), m["steel"],
            shell, lods=(0, 1))
        box(f"lamp_back_{k}", (0.02, 0.18, 0.18), (xf + 0.01, s * (door_w / 2 + 0.7), door_h + 0.95), m["steel"],
            shell, lods=(0, 1))
        cyl(f"lamp_{k}", 0.14, 0.12, (xf + 0.3, s * (door_w / 2 + 0.7), door_h + 0.88), "Z", m["steel"], shell,
            seg=12, r2=0.06, lods=(0, 1))
    # the wicket door in the -Y leaf, and the hangar's number on the +Y leaf
    wy, lx = -door_w / 4 + 0.6, xf + 0.205
    box("wicket", (0.02, 0.95, 2.05), (lx, wy, 1.12), m["steel"], shell, lods=(0, 1))
    box("wicket_handle", (0.05, 0.04, 0.2), (lx + 0.03, wy + 0.36, 1.1), m["steel"], shell, lods=(0,))
    stencil("number", "3", 1.6, (xf + 0.205, door_w / 4, 3.3), (math.pi / 2, 0, math.pi / 2), m["mark"], shell,
            lods=(0, 1, 2))
    # the vent: a louvred frame under the apex
    box("vent_frame", (0.12, vent[1] - vent[0] + 0.16, vent[3] - vent[2] + 0.16),
        (xf + 0.02, 0, (vent[2] + vent[3]) / 2), m["steel"], shell, lods=(0, 1, 2))
    for j in range(5):
        z = vent[2] + 0.1 + j * 0.13
        box(f"louvre_{j}", (0.14, vent[1] - vent[0], 0.03), (xf + 0.06, 0, z), m["steel"], shell,
            rot=(0, math.radians(35), 0), lods=(0, 1))
    box("vent_back", (0.02, vent[1] - vent[0], vent[3] - vent[2]), (xf - 0.02, 0, (vent[2] + vent[3]) / 2),
        m["steel"], shell, lods=(0, 1, 2, 3))
    # ---- the back: a door and two windows
    xb = x0 + RECESS
    win = [(-5.2, -3.8, 1.5, 2.4), (3.8, 5.2, 1.5, 2.4)]
    pdoor = (-0.5, 0.5, 0.0, 2.1)
    panels("back_wall", m, shell, xb, -1, win + [pdoor])
    for k, (y0, y1, z0, z1) in enumerate(win):
        box(f"window_{k}", (0.03, y1 - y0, z1 - z0), (xb, (y0 + y1) / 2, (z0 + z1) / 2), m["glass"], shell)
        box(f"window_frame_top_{k}", (0.1, y1 - y0 + 0.12, 0.06), (xb - 0.02, (y0 + y1) / 2, z1 + 0.03), m["steel"],
            shell, lods=(0, 1, 2))
        box(f"window_sill_{k}", (0.16, y1 - y0 + 0.16, 0.06), (xb - 0.05, (y0 + y1) / 2, z0 - 0.03), m["steel"],
            shell, lods=(0, 1, 2))
        box(f"window_bar_{k}", (0.04, 0.04, z1 - z0), (xb - 0.02, (y0 + y1) / 2, (z0 + z1) / 2), m["steel"], shell,
            lods=(0, 1))
    box("back_door", (0.05, 1.0, 2.1), (xb, 0, 1.05), m["steel"], shell)
    box("back_door_head", (0.12, 1.2, 0.08), (xb - 0.03, 0, 2.14), m["steel"], shell, lods=(0, 1, 2))
    box("back_door_handle", (0.05, 0.04, 0.2), (xb - 0.04, 0.38, 1.05), m["steel"], shell, lods=(0,))
    box("back_step", (0.5, 1.4, 0.16), (xb - 0.25, 0, 0.08), m["concrete"], shell, lods=(0, 1, 2))
    return [hx, hy, hz]


# ------------------------------------------------------------------ the hangar's ruin
@kind
def hangar_ruin():
    """The hangar brought down, on its plan at the remains' 1.5 m. What survives says which
    hangar it was: the olive vault's feet still stand on their curbs, torn off at uneven
    heights and peeled in or out; bent rib stubs rise from them; the rest of the vault lies
    crushed into folds of the same ribbed sheet, torn open over a burnt bed; the back wall
    is a jagged stub and the front's braced leaves lie thrown over the wreck under their
    fallen track beam. Sheets slid off over the curbs lie flat on the ground beside it (the
    kind's rubble allowance). Soot and blistered paint round where it burnt."""
    hx, hy, hz = 12.0, 8.0, 0.75
    top = 2 * hz - 0.06
    parts.SCORCH.extend([(Vector((3.0, -1.5, 0.4)), 4.5), (Vector((-6.0, 2.5, 0.4)), 3.5), (Vector((8.5, 4.0, 0.4)), 2.5)])
    m = dict(
        sheet=textured("ruin_sheet", "roof_sheet", colour=OLIVE, chip=0.6, dirt=0.15, rise=0.3, streak=0.0,
                       soot=0.25, mottle=0.3, grain=0.02),
        wall=textured("ruin_wall", "cladding", colour=(0.092, 0.096, 0.07), chip=0.7, dirt=0.2, rise=0.4, soot=0.4),
        steel=textured("ruin_steel", "enamel", colour=(0.04, 0.042, 0.032), chip=0.7, dirt=0.3, rise=0.4, soot=0.5),
        concrete=textured("ruin_curb", "concrete", chip=0.8, dirt=0.9, rise=0.3, mottle=0.25, soot=0.3),
        char=textured("ruin_char", "rubble", colour=(0.12, 0.105, 0.088), chip=0.0, dirt=0.3, rise=0.2, ash=0.2),
    )
    ruin = body("ruin")
    arc = vault_length(1.0)
    xs, xe = -hx + 0.3, hx - 0.3
    sheet_uv = lambda x, s: (x / SHEET_U_M, s / SHEET_V_M)
    n01 = lambda *p: 0.5 + 0.5 * noise.noise(Vector(p))
    # the curbs, cracked through in two places each, the broken ends a little out of line
    for k, s in enumerate((-1, 1)):
        breaks = [-hx + 0.25, -3.0 + 2.0 * s, 6.5 - s, hx - 0.25]
        for j, (a, b) in enumerate(zip(breaks, breaks[1:])):
            box(f"curb_{k}_{j}", (b - a - 0.12, CURB_W, CURB_H), ((a + b) / 2, s * (HALF_SPAN - 0.02 * j), CURB_H / 2),
                m["concrete"], ruin, bevel=0.02, rot=(0, 0, 0.012 * (j - 1) * s))
    # the vault's feet: runs of ribbed sheet still on the curbs, torn off at uneven heights,
    # each peeled inward or outward; gaps where whole runs went down with the vault
    r = random.Random(23)
    for k, s in enumerate((-1, 1)):
        x = xs
        j = 0
        while x < xe - 1.0:
            length = r.uniform(2.5, 5.5)
            a, b = x, min(xe, x + length)
            x = b + r.uniform(0.6, 2.2)
            if r.random() < 0.2:
                continue  # this run went down
            lean = r.choice((-1, 1)) * r.uniform(0.2, 0.6)  # + peels inward, - outward
            seed = r.uniform(0, 50)

            def foot(bm, lod, a=a, b=b, s=s, lean=lean, seed=seed):
                nu, nv = max(2, int((b - a) * (8, 4, 1.5, 0.6)[lod])), (5, 3, 2, 1)[lod]

                def at(u, v, back=0.0):
                    xx = a + (b - a) * u
                    reach = 0.012 + 0.042 * n01(xx * 0.45, s * 3.0, seed) * (0.6 + 0.4 * n01(xx * 2.2, seed, 1.0))
                    y, z = vault(reach * v if s > 0 else 1.0 - reach * v)
                    return (xx, y - s * (lean * v * v + back), min(top, z))

                grid(bm, nu, nv, at, flip=s > 0)
                grid(bm, nu, nv, lambda u, v: at(u, v, 0.03), flip=s < 0)
                return True

            mesh_part(f"foot_{k}_{j}", foot, m["sheet"], ruin)
            j += 1

    # the collapsed sheet: folds running along the hangar, torn open where the noise is high
    ylim = HALF_SPAN - 0.14

    def crease(t):  # a buckle: straight flanks up to a sharp ridge
        return 1.0 - abs((t % 1.0) * 2.0 - 1.0)

    def height(x, y):
        s = y / ylim
        fold = 0.42 * crease(1.3 * s + 0.25 * math.sin(x / 3.1) + 0.5) ** 1.5
        cross = 0.25 * crease(x / 3.7 + y / 9.0) ** 2 * n01(x / 6, y / 6, 8.0)
        lump = 0.18 * noise.noise(Vector((x / 4.0, y / 4.0, 1.7)))
        z = 0.12 + 0.6 * s ** 6 + fold + cross + lump
        edge = min(1.0, (xe - abs(x)) / 1.2)  # down onto the ground at the ends
        return max(0.06, min(top, z * max(0.15, edge)))

    def heap(bm, lod):
        nu, nv = (120, 60, 24, 10)[lod], (56, 28, 14, 6)[lod]

        def at(u, v):
            x, y = xs + (xe - xs) * u, -ylim + 2 * ylim * v
            return (x, y, height(x, y))

        def keep(i, j):
            x, y = xs + (xe - xs) * (i + 0.5) / nu, -ylim + 2 * ylim * (j + 0.5) / nv
            return noise.noise(Vector((x / 2.6, y / 2.6, 4.2))) < 0.3

        solid_sheet(bm, nu, nv, at, keep, thick=0.03,
                    uv=lambda u, v: sheet_uv(xs + (xe - xs) * u, arc * v))
        return True

    mesh_part("heap", heap, m["sheet"], ruin)
    # the burnt bed the sheet came down on, which shows through its tears, broken concrete on it
    box("char_bed", (2 * (xe - 0.2), 2 * (HALF_SPAN - 0.25), 0.05), (0, 0, 0.025), m["char"], ruin)

    def chunks(bm, lod):
        rr = random.Random(5)
        for k in range(26):
            x, y = rr.uniform(-xe + 0.5, xe - 0.5), rr.uniform(-HALF_SPAN + 0.6, HALF_SPAN - 0.6)
            sz = rr.uniform(0.1, 0.28)
            lump(bm, lod, (x, y, sz * 0.5), (sz * 1.3, sz, sz * 0.8), 0.35, k + 1)
        return True

    mesh_part("chunks", chunks, m["concrete"], ruin, lods=(0, 1, 2))
    # rib stubs: each frame's feet rise from the curbs following the vault, then bend over
    r = random.Random(41)
    for k, x in enumerate((-8.0, -4.0, 0.0, 4.0, 8.0)):
        for j, s in enumerate((-1, 1)):
            if r.random() < 0.25:
                continue
            t1 = r.uniform(0.028, 0.05)
            pts = []
            for t in (0.0, t1 * 0.5, t1):
                y, z = vault(t if s > 0 else 1.0 - t)
                pts.append((x, y - s * 0.08, min(top - 0.05, z)))
            last = pts[-1]
            bend = r.uniform(0.5, 1.0)
            pts.append((x + r.uniform(-0.4, 0.4), last[1] - s * bend, max(0.3, last[2] - r.uniform(0.2, 0.6))))
            for i, (a, b) in enumerate(zip(pts, pts[1:])):
                bar(f"rib_{k}_{j}_{i}", a, b, 0.1, 0.14, m["steel"], ruin, lods=(0, 1, 2))
        # and the frame's crown, broken and lying through the heap
        r2 = random.Random(100 + k)
        lying = []
        for y in (-4.4, -1.2, 2.0, 4.6):
            lx, ly = x + r2.uniform(-0.6, 0.6), y + r2.uniform(-0.5, 0.5)
            lying.append((lx, ly, height(lx, ly) + 0.05))
        for i, (a, b) in enumerate(zip(lying, lying[1:])):
            if r2.random() < 0.3:
                continue
            bar(f"crown_{k}_{i}", a, b, 0.1, 0.14, m["steel"], ruin, lods=(0, 1, 2))

    # the end walls' feet: a jagged stub of each, leaning out where it tore
    def stub(x, facing, spans, seed):
        def build(bm, lod):
            for y0, y1 in spans:
                nu, nv = max(2, int((y1 - y0) * (5, 3, 1, 0.5)[lod])), (4, 2, 1, 1)[lod]

                def at(u, v, back=0.0, y0=y0, y1=y1):
                    y = y0 + (y1 - y0) * u
                    h = min(top, 0.4 + 0.95 * n01(y * 0.5, seed, 2.0) + 0.12 * noise.noise(Vector((y * 3.1, seed, 0.0))))
                    z = h * v
                    return (x + facing * (0.12 * v * v * h - back), y, z)

                grid(bm, nu, nv, at, flip=facing < 0)
                grid(bm, nu, nv, lambda u, v: at(u, v, 0.05), flip=facing > 0)
            return True

        return build

    span = HALF_SPAN - 0.25
    mesh_part("back_stub", stub(-hx + RECESS, -1, [(-span, span)], 3.0), m["wall"], ruin)
    xf = hx - RECESS
    mesh_part("front_stub", stub(xf, 1, [(-span, -5.0), (5.0, span)], 9.0), m["wall"], ruin)
    # the front: its leaves thrown down over the wreck, the track beam fallen across them
    for k, (cy, tilt, yaw, z) in enumerate(((-2.6, 0.24, 0.12, 0.7), (2.5, 0.1, -0.2, 0.45))):
        rot = Euler((0.04 * (k * 2 - 1), tilt, yaw))
        centre = Vector((xf - 2.6, cy, z))
        box(f"leaf_{k}", (5.0, 4.6, 0.08), centre, m["wall"], ruin, rot=rot)
        for j, oy in enumerate((-1.6, 0.0, 1.6)):  # its rails, on its face
            box(f"leaf_rail_{k}_{j}", (5.0, 0.06, 0.12), centre + rot.to_matrix() @ Vector((0, oy, 0.09)), m["steel"],
                ruin, rot=rot, lods=(0, 1))
    bar("track_beam", (xf + 0.1, -5.3, 0.12), (xf - 1.0, 5.0, top - 0.15), 0.2, 0.26, m["steel"], ruin)
    for k, s in enumerate((-1, 1)):
        bar(f"jamb_{k}", (xf, s * 4.9, 0.05), (xf - 1.8, s * 4.3, 1.1), 0.22, 0.16, m["steel"], ruin, lods=(0, 1, 2))
    # sheets that slid off over the curbs, lying on the ground beside them
    r = random.Random(17)
    for k in range(10):
        s = r.choice((-1, 1))
        x = r.uniform(-hx + 1.5, hx - 1.5)
        out = r.uniform(0.6, 1.2)
        a = Vector((x, s * (HALF_SPAN - 0.05), CURB_H + 0.02))
        b = Vector((x + r.uniform(-0.6, 0.6), s * (HALF_SPAN + out), 0.03))
        d = b - a
        rot = Euler((math.atan2(-d.z, abs(d.y)) * s, 0, math.atan2(d.x, abs(d.y)) * -s))
        box(f"slid_{k}", (r.uniform(1.2, 2.2), d.length + 0.2, 0.03), (a + b) / 2, m["sheet"], ruin, rot=rot,
            lods=(0, 1, 2))
    return [hx, hy, hz]
