"""Court and garden bodies, each authored to one simulation box (the presets'
`street_props.bodies`): the garden catalog's rows in `fixtures/props/city/gardens.json`.
Shared by every region. Generic: no brand, logo or sign.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/courts.py <kind> <out.glb>

kind (box half extents, metres):
  garden_shed   a timber apex shed: shiplap walls, a ledged door, a window, a felt roof; [1.2, 0.9, 1.1]
  hedge         a clipped privet hedge, a 6 m module; [3.0, 0.4, 0.9]
  garden_fence  close-board timber panels between posts on a gravel board, a 5.5 m module; [2.75, 0.05, 0.9]
  washing_line  two T-posts, two lines, sheets and towels pegged out; [2.0, 0.1, 0.9]
  garden_table  a slatted timber patio table with four chairs; [1.0, 0.7, 0.4]

A piece is capped at 1 MiB raw (specs/courtyards/README.md), so its recipes are
embedded at `TEXTURE_PX`. The battle fits each placed box from the authored one.
Origin at the box's centre on the ground, +X along its first half extent.
"""
import bpy, bmesh, sys, os, math, json, random
from mathutils import Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *

ARGS = [a for a in script_args() if not a.startswith("--")]
KIND, OUT = ARGS[0], ARGS[1]
reset()
root = empty(KIND)
rng = random.Random(13)
KINDS = {}
# Half the street set's: up to three recipes and the geometry stay under the 1 MiB cap.
TEXTURE_PX = 128


def kind(fn):
    KINDS[fn.__name__] = fn
    return fn


def tube(name, a, b, r, mat, lods=TIERS, seg=8):
    """A round bar from `a` to `b`; four-sided on the far tiers."""
    a, b = Vector(a), Vector(b)
    d = b - a
    rot = Vector((0, 0, 1)).rotation_difference(d.normalized()).to_euler()
    return cyl(name, r, d.length, (a + b) / 2, "Z", mat, root, seg=seg, rot=rot, lods=lods, min_seg=4)


def timber(name, colour, chip=0.3, dirt=0.6, rise=0.4, mottle=0.35, streak=0.3):
    return textured(name, "pallet_wood", colour=colour, chip=chip, dirt=dirt, rise=rise, mottle=mottle, streak=streak)


def placed(origin, yaw):
    """A point `(x, y, z)` of a part's own frame, turned by `yaw` and moved to `origin`."""
    c, s = math.cos(yaw), math.sin(yaw)
    return lambda x, y, z: (origin[0] + x * c - y * s, origin[1] + x * s + y * c, origin[2] + z)


# ------------------------------------------------------------------ the garden
@kind
def garden_shed():
    """A 2.2 m by 1.6 m apex shed under a felt roof that overhangs it to the box: shiplap
    walls (level boards, a shadow line under each lap) on timber bearers, corner battens, a ledged door and a four-pane window on the
    long face at +Y, barge boards on the gables."""
    hx, hy, hz = 1.2, 0.9, 1.1
    wall = timber("shed_wall", (0.15, 0.085, 0.045), chip=0.4, dirt=0.5, rise=0.4, streak=0.5, mottle=0.25)
    trim = timber("shed_trim", (0.1, 0.06, 0.035), dirt=0.4)
    lap = flat_paint("shed_lap", (0.03, 0.017, 0.009), rough=0.9, grime=0.0)
    felt = textured("shed_felt", "flat_roof", colour=(0.045, 0.045, 0.045), chip=0.0, dirt=0.0, streak=0.0,
                    lichen=0.4)
    glass = flat_paint("shed_glass", (0.05, 0.06, 0.065), rough=0.1, grime=0.3)
    iron = flat_paint("shed_iron", (0.025, 0.025, 0.025), rough=0.5, grime=0.2)
    wx, wy, base = 1.08, 0.76, 0.08  # the walls' half extents, and their sill above the ground
    ridge, fall = 2 * hz - 0.05, 0.6  # the roof's underside at the ridge, and its fall per metre
    eaves = ridge - fall * wy
    for k, y in enumerate((-wy + 0.06, 0.0, wy - 0.06)):
        box(f"bearer_{k}", (2 * wx, 0.07, base), (0, y, base / 2), trim, root, lods=(0, 1, 2))
    # the walls and gables as one solid; its boards run level, as shiplap does
    prism("body", [(-wy, base), (wy, base), (wy, eaves), (0.0, ridge), (-wy, eaves)], 2 * wx, (0, 0, 0), wall, root,
          rot=(0, 0, math.pi / 2))
    # the shadow under each board's lap, every 12.5 cm, across the long faces and up the gables
    for k in range(1, int((ridge - base) / 0.125)):
        z = base + 0.125 * k
        if z < eaves - 0.03:
            for f, s in enumerate((-1, 1)):
                box(f"lap_y{f}_{k}", (2 * wx, 0.006, 0.012), (0, s * (wy + 0.003), z), lap, root, lods=(0,))
        half = min(wy, (ridge - z) / fall) - 0.03  # the gable's width at this height
        if half > 0.05:
            for f, s in enumerate((-1, 1)):
                box(f"lap_x{f}_{k}", (0.006, 2 * half, 0.012), (s * (wx + 0.003), 0, z), lap, root, lods=(0,))
    for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
        box(f"batten_{k}", (0.05, 0.05, eaves - base), (sx * (wx + 0.005), sy * (wy + 0.005), (eaves + base) / 2), trim,
            root, lods=(0, 1, 2))
    # the roof: two felted slopes of board out to the box, a ridge strip, barge boards and fascias
    edge = ridge - fall * hy
    for k, s in enumerate((-1, 1)):
        slope = [(0.0, ridge), (s * hy, edge), (s * hy, edge + 0.05), (0.0, ridge + 0.05)]
        prism(f"roof_{k}", slope if s > 0 else slope[::-1], 2 * hx, (0, 0, 0), felt, root, rot=(0, 0, math.pi / 2))
        box(f"fascia_{k}", (2 * hx, 0.02, 0.1), (0, s * (hy - 0.01), edge - 0.02), trim, root, lods=(0, 1, 2))
        for j, sx in enumerate((-1, 1)):
            run = math.hypot(hy, fall * hy) - 0.06  # stopping short of the eaves, inside the box
            box(f"barge_{k}_{j}", (0.025, run, 0.12), (sx * (hx - 0.0125), s * (hy - 0.06) / 2, (ridge + edge) / 2 - 0.02), trim,
                root, rot=(s * -math.atan(fall), 0, 0), lods=(0, 1, 2))
    box("ridge_cap", (2 * hx, 0.16, 0.03), (0, 0, ridge + 0.06), felt, root, lods=(0, 1, 2))
    # the door: vertical boards, three ledges and a brace showing through, a hasp and two hinges
    face, dx, dw, dh = wy + 0.015, -0.4, 0.72, 1.5
    box("door", (dw, 0.03, dh), (dx, face, base + 0.04 + dh / 2), trim, root, bevel=0.008)
    for k in range(1, 6):
        box(f"door_joint_{k}", (0.008, 0.006, dh - 0.02), (dx - dw / 2 + k * dw / 6, face + 0.016, base + 0.04 + dh / 2),
            iron, root, lods=(0,))
    box("door_frame", (dw + 0.1, 0.02, dh + 0.06), (dx, face - 0.01, base + 0.04 + dh / 2), trim, root, lods=(0, 1, 2))
    for j, z in enumerate((0.3, 1.3)):
        box(f"hinge_{j}", (0.3, 0.008, 0.035), (dx - dw / 2 + 0.15, face + 0.019, base + z), iron, root, lods=(0, 1))
    box("hasp", (0.12, 0.012, 0.04), (dx + dw / 2 - 0.04, face + 0.02, base + 0.85), iron, root, lods=(0, 1))
    # the window: four panes in a frame under the eaves, a sill under it
    wcx, wz, ww, wh = 0.55, 1.12, 0.62, 0.46
    box("window_frame", (ww + 0.08, 0.03, wh + 0.08), (wcx, face, wz), trim, root, bevel=0.006)
    for k, (ox, oz) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
        box(f"pane_{k}", (ww / 2 - 0.04, 0.01, wh / 2 - 0.04), (wcx + ox * ww / 4, face + 0.012, wz + oz * wh / 4), glass,
            root, lods=(0, 1, 2))
    box("window_far", (ww, 0.01, wh), (wcx, face + 0.012, wz), glass, root, lods=(3,))
    box("window_sill", (ww + 0.14, 0.06, 0.03), (wcx, face + 0.02, wz - wh / 2 - 0.05), trim, root, lods=(0, 1, 2))
    return [hx, hy, hz]


def _periodic(f, x, length):
    """`f(x)` blended with `f(x - length)` across the module, so its two ends agree and
    repeated modules meet without a step."""
    w = (x + length / 2) / length
    return (1 - w) * f(x) + w * f(x - length)


@kind
def hedge():
    """A clipped privet hedge 1.8 m high: a slight batter, the top rounded off by the
    trimmer, the faces lumpy with leaf and with the shallow scallops of its passes. The
    relief repeats across the module, so a run of hedges is one hedge; its ends are cut
    square where they show at a run's end."""
    hx, hy, hz = 3.0, 0.4, 0.9
    leaf = textured("hedge_leaf", "privet", chip=0.0, dirt=0.2, rise=0.35, streak=0.0, mottle=0.12, grain=0.0)
    top, foot, crown, r = 2 * hz - 0.03, hy - 0.01, hy - 0.07, 0.2  # height; half widths at foot and top; corner

    def section(ds):
        """The hedge's cross-section from its left foot over the top to its right foot, as
        (y, z, outward normal) points `ds` apart at most."""
        bend = (0.0, 1.0) if ds is None else [i / 6 for i in range(7)]
        corner = [(-(crown - r) - r * math.cos(t * math.pi / 2), top - r + r * math.sin(t * math.pi / 2)) for t in bend]
        line = [(-foot, 0.0)] + corner + [(-y, z) for y, z in reversed(corner)] + [(foot, 0.0)]
        out = []
        for (y0, z0), (y1, z1) in zip(line, line[1:]):
            n = 1 if ds is None else max(1, math.ceil(math.hypot(y1 - y0, z1 - z0) / ds))
            out.extend((y0 + (y1 - y0) * i / n, z0 + (z1 - z0) * i / n) for i in range(n))
        out.append(line[-1])
        pts = []
        for i, (y, z) in enumerate(out):  # the normal across the neighbouring points
            a, b = out[max(0, i - 1)], out[min(len(out) - 1, i + 1)]
            n = Vector((b[1] - a[1], -(b[0] - a[0]))).normalized()  # (dy, dz) turned outward
            pts.append((y, z, -n.x, -n.y))
        return pts

    def build(bm, lod):
        dx, ds, leafy, scallop = ((0.15, 0.12, 0.022, 0.03), (0.4, 0.25, 0.0, 0.03), (1.5, 0.5, 0.0, 0.0),
                                  (2 * hx, None, 0.0, 0.0))[lod]
        ring = section(ds)
        n = max(1, round(2 * hx / dx))
        rows = []
        for i in range(n + 1):
            x = -hx + 2 * hx * i / n
            row = []
            for y, z, ny, nz in ring:
                def relief(xx):
                    p = Vector((xx, y, z))
                    return leafy * noise.noise(p * 9.0) + scallop * noise.noise(p * 1.4 + Vector((5.0, 0, 0)))

                d = _periodic(relief, x, 2 * hx) if (leafy or scallop) else 0.0
                d *= min(1.0, z / 0.15)  # the foot stands on the ground
                row.append(bm.verts.new((x, y + ny * d, max(0.0, z + nz * d))))
            rows.append(row)
        for a, b in zip(rows, rows[1:]):
            for j in range(len(ring) - 1):
                bm.faces.new((a[j], a[j + 1], b[j + 1], b[j])).smooth = True
        # the cut ends, flat and on their own vertices: where modules meet, the faces
        # either side shade as one surface, not darkened toward an end that is not there
        for row, turn in ((rows[0], 1), (rows[-1], -1)):
            bm.faces.new([bm.verts.new(v.co) for v in row][::turn])
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    mesh_part("hedge", build, leaf, root)
    return [hx, hy, hz]


@kind
def garden_fence():
    """Close-board fencing: three 1.83 m bays between square posts, each a gravel board,
    feather-edge boards overlapping on two back rails, and a capping rail. Everything
    stands inside the 10 cm the body is thick. The end posts straddle the module's ends,
    so repeated modules share them."""
    hx, hy, hz = 2.75, 0.05, 0.9
    board_m = timber("fence_board", (0.21, 0.135, 0.08), dirt=0.3, rise=0.3, mottle=0.3)
    post_m = timber("fence_post", (0.14, 0.095, 0.06), dirt=0.3, rise=0.3)
    bays, top = 3, 2 * hz
    bay = 2 * hx / bays
    gravel, cap = 0.15, 0.04  # the gravel board's height; the capping rail's
    for k in range(bays + 1):
        x = -hx + k * bay
        box(f"post_{k}", (0.09, 0.09, top - 0.02), (x, 0, (top - 0.02) / 2), post_m, root, bevel=0.008)
        box(f"post_cap_{k}", (0.1, 0.1, 0.02), (x, 0, top - 0.01), post_m, root, lods=(0, 1))
    for k in range(bays):
        cx, span = -hx + (k + 0.5) * bay, bay - 0.09
        box(f"gravel_{k}", (span, 0.025, gravel), (cx, 0.0, gravel / 2), post_m, root, lods=(0, 1, 2))
        for j, z in enumerate((0.45, 1.45)):
            box(f"rail_{k}_{j}", (span, 0.035, 0.08), (cx, -0.027, z), post_m, root, lods=(0, 1, 2))
        box(f"capping_{k}", (span, 0.07, cap), (cx, 0.0, top - cap / 2 - 0.02), post_m, root, lods=(0, 1, 2))
        h = top - gravel - cap - 0.02
        x0, j = cx - span / 2, 0
        while x0 < cx + span / 2 - 0.02:  # feather-edge boards, each lapping the last
            w = min(0.1 + rng.uniform(-0.006, 0.006), cx + span / 2 - x0)
            box(f"board_{k}_{j}", (w, 0.016, h), (x0 + w / 2, 0.008 + 0.005 * (j % 2), gravel + h / 2), board_m, root,
                rot=(0, rng.uniform(-0.006, 0.006), 0), lods=(0, 1))
            x0 += w - 0.015
            j += 1
        box(f"boards_{k}", (span, 0.02, h), (cx, 0.012, gravel + h / 2), board_m, root, lods=(2,))
    box("boards_far", (2 * hx, 0.02, top - 0.04), (0, 0.0, top / 2 - 0.02), board_m, root, lods=(3,))
    return [hx, hy, hz]


@kind
def washing_line():
    """A straight line down a long narrow garden: two galvanised T-posts 4 m apart,
    two lines sagging between them, a bedsheet, a duvet cover, towels and a pillowcase
    pegged out and hanging still."""
    hx, hy, hz = 2.0, 0.1, 0.9
    zinc = textured("line_post", "galvanised", chip=0.4, dirt=0.6, rise=0.5)
    cord = flat_paint("line_cord", (0.5, 0.5, 0.48), rough=0.7, grime=0.0)
    top, px, arm = 2 * hz, hx - 0.04, hy - 0.02
    sag = 0.07
    line_z = lambda x: top - 0.06 - sag * (1 - (x / px) ** 2)  # the cord's height along it
    for k, s in enumerate((-1, 1)):
        tube(f"post_{k}", (s * px, 0, 0.0), (s * px, 0, top - 0.02), 0.025, zinc)
        tube(f"arm_{k}", (s * px, -arm, top - 0.04), (s * px, arm, top - 0.04), 0.016, zinc, lods=(0, 1, 2))
        cyl(f"cap_{k}", 0.03, 0.03, (s * px, 0, top - 0.015), "Z", zinc, root, seg=8, lods=(0, 1))
    for j, y in enumerate((-arm + 0.01, arm - 0.01)):
        for i in range(8):  # the sagging cord, in eight straight runs
            xa, xb = -px + 2 * px * i / 8, -px + 2 * px * (i + 1) / 8
            tube(f"cord_{j}_{i}", (xa, y, line_z(xa)), (xb, y, line_z(xb)), 0.006, cord, lods=(0, 1), seg=4)

    def hang(name, line, x0, x1, drop, colour, seed):
        """A cloth pegged along the line from x0 to x1 and hanging `drop`: a thin solid
        with soft vertical folds, its hem swinging a little, and a peg at each end."""
        y = (-arm + 0.01, arm - 0.01)[line]
        cloth = textured(name, "nylon", colour=colour, chip=0.0, dirt=0.0, streak=0.0, mottle=0.08)
        peg = flat_paint(f"{name}_peg", tuple(0.6 * c for c in colour), rough=0.5, grime=0.0)

        def build(bm, lod):
            cols, rows = ((14, 6), (6, 3), (2, 1), (1, 1))[lod]
            fold = (0.035, 0.025, 0.0, 0.0)[lod]
            for side in (-1, 1):
                grid = []
                for r in range(rows + 1):
                    t = r / rows
                    row = []
                    for c in range(cols + 1):
                        x = x0 + (x1 - x0) * c / cols
                        wave = fold * math.sin(c / cols * math.pi * (x1 - x0) / 0.22 + seed) * (0.4 + 0.6 * t)
                        z = line_z(x) - 0.01 - drop * t * (1 + 0.05 * math.sin(math.pi * c / cols))  # sags between pegs
                        row.append(bm.verts.new((x, y + wave + side * 0.004, z)))
                    grid.append(row)
                for r in range(rows):
                    for c in range(cols):
                        q = (grid[r][c], grid[r][c + 1], grid[r + 1][c + 1], grid[r + 1][c])
                        bm.faces.new(q if side > 0 else q[::-1])  # each face looks out of its own side

        mesh_part(name, build, cloth, root, smooth=True)
        for k, x in enumerate((x0 + 0.04, x1 - 0.04)):
            box(f"{name}_peg_{k}", (0.012, 0.022, 0.07), (x, y, line_z(x) - 0.015), peg, root, lods=(0,))

    hang("sheet", 1, -1.75, -0.3, 1.0, (0.74, 0.74, 0.72), 0.0)
    hang("duvet", 0, -0.55, 0.95, 0.95, (0.36, 0.48, 0.66), 1.3)
    hang("towel_a", 1, 0.0, 0.55, 0.75, (0.6, 0.18, 0.12), 2.1)
    hang("towel_b", 1, 0.62, 1.12, 0.7, (0.55, 0.52, 0.32), 0.7)
    hang("pillowcase", 0, 1.15, 1.65, 0.65, (0.72, 0.7, 0.66), 2.9)
    return [hx, hy, hz]


@kind
def garden_table():
    """A slatted hardwood patio table, 1.2 m by 0.8 m, with four slatted chairs drawn up,
    one to each side and set a little askew; the chairs at the ends reach the box."""
    hx, hy, hz = 1.0, 0.7, 0.4
    wood = timber("teak", (0.15, 0.105, 0.07), chip=0.25, dirt=0.5, rise=0.3, mottle=0.4, streak=0.2)
    tx, ty, tz = 0.6, 0.4, 0.73
    for k in range(7):
        box(f"top_slat_{k}", (2 * tx, 0.1, 0.025), (0, -ty + 0.055 + k * (2 * ty - 0.11) / 6, tz - 0.0125), wood, root,
            bevel=0.004, lods=(0, 1))
    box("top_far", (2 * tx, 2 * ty, 0.025), (0, 0, tz - 0.0125), wood, root, lods=(2, 3))
    for k, s in enumerate((-1, 1)):
        box(f"apron_x_{k}", (2 * tx - 0.12, 0.025, 0.07), (0, s * (ty - 0.06), tz - 0.06), wood, root, lods=(0, 1, 2))
        box(f"apron_y_{k}", (0.025, 2 * ty - 0.12, 0.07), (s * (tx - 0.06), 0, tz - 0.06), wood, root, lods=(0, 1, 2))
    for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
        box(f"table_leg_{k}", (0.05, 0.05, tz - 0.025), (sx * (tx - 0.07), sy * (ty - 0.07), (tz - 0.025) / 2), wood,
            root)

    def chair(name, origin, yaw):
        """A slatted side chair facing along its own +y, its back at -y."""
        at = placed(origin, yaw)
        seat, sw, sd = 0.44, 0.44, 0.42
        for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
            tall = (seat + 0.38) if sy < 0 else seat
            box(f"{name}_leg_{k}", (0.04, 0.04, tall), at(sx * (sw / 2 - 0.02), sy * (sd / 2 - 0.02), tall / 2), wood, root,
                rot=(0, 0, yaw), lods=(0, 1, 2))
        for k in range(4):
            box(f"{name}_slat_{k}", (sw, 0.085, 0.02), at(0, -sd / 2 + 0.055 + k * 0.103, seat), wood, root,
                rot=(0, 0, yaw), lods=(0, 1))
        box(f"{name}_seat_far", (sw, sd, 0.02), at(0, 0, seat), wood, root, rot=(0, 0, yaw), lods=(2, 3))
        for k, z in enumerate((seat + 0.16, seat + 0.27, seat + 0.36)):
            box(f"{name}_back_{k}", (sw - 0.04, 0.02, 0.07), at(0, -sd / 2 + 0.01, z), wood, root, rot=(0, 0, yaw),
                lods=(0, 1))
        box(f"{name}_back_far", (sw, 0.02, 0.3), at(0, -sd / 2 + 0.01, seat + 0.25), wood, root, rot=(0, 0, yaw),
            lods=(2, 3))
        box(f"{name}_rail", (sw - 0.04, 0.03, 0.05), at(0, 0, seat - 0.05), wood, root, rot=(0, 0, yaw), lods=(0, 1, 2))

    # one to each long side, pushed in under the edge; one at each end, pulled out
    for name, origin, yaw in (("chair_s", (-0.12, -0.49, 0), 0.0), ("chair_n", (0.15, 0.49, 0), math.pi),
                              ("chair_w", (-0.79, 0.04, 0), -math.pi / 2), ("chair_e", (0.79, -0.05, 0), math.pi / 2)):
        chair(name, origin, yaw + rng.uniform(-0.12, 0.12))
    return [hx, hy, hz]


box_half = KINDS[KIND]()
rest_on_ground()
bpy.context.view_layer.update()
finish(ao_distance=1.0, ao_strength=0.6, ao_rays=10, paint_scale=1.3)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT, texture_px=TEXTURE_PX)
