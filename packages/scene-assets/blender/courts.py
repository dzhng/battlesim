"""Court and garden bodies, each authored to one simulation box (the presets'
`street_props.bodies`): the rows of `fixtures/props/city/gardens.json` and `courts.json`.
A `<kind>_<family>` is that region's look of a shared kind, on the shared kind's box.
Generic: no brand, logo or sign.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/courts.py <kind> <out.glb>

kind (box half extents, metres):
  garden_shed   a timber apex shed: shiplap walls, a ledged door, a window, a felt roof; [1.2, 0.9, 1.1]
  hedge         a clipped privet hedge, a 6 m module; [3.0, 0.4, 0.9]
  garden_fence  close-board timber panels between posts on a gravel board, a 5.5 m module; [2.75, 0.05, 0.9]
  washing_line  two T-posts, two lines, sheets and towels pegged out; [2.0, 0.1, 0.9]
  garden_table  a slatted timber patio table with four chairs; [1.0, 0.7, 0.4]
  bike_rack     four Sheffield stands, three bikes parked; [1.5, 0.9, 0.5]
  playground_frame  a timber play tower: slide, ladder, net, monkey bars; [2.0, 1.5, 1.25]
  swing         a two-seat A-frame swing; [1.7, 0.9, 1.1]
China:
  bike_shed     a blue steel-roofed shed, scooters and bikes under it; [5.0, 1.25, 1.2]
  pingpong_table  a cast-concrete table-tennis table; [1.37, 0.76, 0.45]
  outdoor_gym   a double air walker in blue and yellow; [0.8, 0.5, 0.9]
  laundry_poles quilts airing and shirts on hangers between two posts; [3.0, 0.15, 1.0]
  bench_china   lacquered slats on granite ends; the bench's box
  bins_china    four lidded bins in the sorting colours; the bins' box
New York:
  chainlink_fence  tall chain-link between posts, a 3 m module; [1.5, 0.05, 1.8]
  basketball_hoop  a pole, perforated backboard, rim and chain net facing -y; [0.9, 0.6, 1.95]
  basketball_court half a court, its baseline at -x; [7.0, 7.5, 0.02]
  garage_row    three lock-up garages in block, shutters at -y; [4.5, 3.0, 1.3]
  dumpster      a front-load dumpster; [1.0, 0.6, 0.65]
  bench_new_york  the parks' slatted bench on cast-iron ends; the bench's box
  bins_new_york the parks' wire litter basket and rubbish bags; the bins' box

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
    a panel of feather-edge boards on two back rails, and a capping rail. The boards are
    the panel's texture (`feather_edge`) on every tier: as geometry they crawl in moiré.
    Everything stands inside the 10 cm the body is thick. The end posts straddle the
    module's ends, so repeated modules share them."""
    hx, hy, hz = 2.75, 0.05, 0.9
    board_m = textured("fence_board", "feather_edge", colour=(0.3, 0.2, 0.125), chip=0.3, dirt=0.3, rise=0.3,
                       streak=0.3, mottle=0.2)
    post_m = timber("fence_post", (0.2, 0.135, 0.085), dirt=0.3, rise=0.3)
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
        # near, the panel in strips: its occlusion then shades along the boards, not in a
        # streak across one long triangle's diagonal
        for j in range(6):
            box(f"boards_{k}_{j}", (span / 6, 0.022, h), (cx - span / 2 + (j + 0.5) * span / 6, 0.011, gravel + h / 2),
                board_m, root, lods=(0, 1))
        box(f"boards_{k}", (span, 0.022, h), (cx, 0.011, gravel + h / 2), board_m, root, lods=(2,))
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
        tube(f"arm_{k}", (s * px, -arm, top - 0.04), (s * px, arm, top - 0.04), 0.016, zinc, lods=(0, 1))
        cyl(f"cap_{k}", 0.03, 0.03, (s * px, 0, top - 0.015), "Z", zinc, root, seg=8, lods=(0, 1))
    for j, y in enumerate((-arm + 0.01, arm - 0.01)):
        for i in range(8):  # the sagging cord, in eight straight runs; near only, as a far one sparkles
            xa, xb = -px + 2 * px * i / 8, -px + 2 * px * (i + 1) / 8
            tube(f"cord_{j}_{i}", (xa, y, line_z(xa)), (xb, y, line_z(xb)), 0.006, cord, lods=(0,), seg=4)

    lines = (-arm + 0.01, arm - 0.01)
    hang("sheet", lines[1], line_z, -1.75, -0.3, 1.0, (0.74, 0.74, 0.72), 0.0)
    hang("duvet", lines[0], line_z, -0.55, 0.95, 0.95, (0.36, 0.48, 0.66), 1.3)
    hang("towel_a", lines[1], line_z, 0.0, 0.55, 0.75, (0.6, 0.18, 0.12), 2.1)
    hang("towel_b", lines[1], line_z, 0.62, 1.12, 0.7, (0.55, 0.52, 0.32), 0.7)
    hang("pillowcase", lines[0], line_z, 1.15, 1.65, 0.65, (0.72, 0.7, 0.66), 2.9)
    return [hx, hy, hz]


def hang(name, y, line_z, x0, x1, drop, colour, seed, pegs=True):
    """A cloth hung along a line at `y` (its height `line_z(x)`) from x0 to x1 and hanging
    `drop`: a thin solid with soft vertical folds, its hem swinging a little, and a peg at
    each end unless it hangs over a bar."""
    cloth = textured(name, "nylon", colour=colour, chip=0.0, dirt=0.0, streak=0.0, mottle=0.08)

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
    if pegs:
        peg = flat_paint(f"{name}_peg", tuple(0.6 * c for c in colour), rough=0.5, grime=0.0)
        for k, x in enumerate((x0 + 0.04, x1 - 0.04)):
            box(f"{name}_peg_{k}", (0.012, 0.022, 0.07), (x, y, line_z(x) - 0.015), peg, root, lods=(0,))


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


# ------------------------------------------------------------------ the courts, every region
RING = ((20, 6), (14, 4), (10, 3), (8, 3))  # segments round a hoop and round its bar, per tier
COARSE_RING = ((12, 4), (8, 3), (8, 3), (6, 3))  # for a wheel among many


def ring(name, at, centre, radius, r, mat, lods=(0, 1), segs=RING):
    """A hoop of round bar (a wheel's tyre) in the part's own XZ plane round `centre`."""

    def build(bm, lod):
        n, m = segs[lod]
        rings = []
        for i in range(n):
            a = 2 * math.pi * i / n
            ax, az = math.cos(a), math.sin(a)
            row = []
            for j in range(m):
                b = 2 * math.pi * j / m
                d = radius + r * math.cos(b)
                row.append(bm.verts.new(at(centre[0] + ax * d, centre[1] + r * math.sin(b), centre[2] + az * d)))
            rings.append(row)
        for i in range(n):
            a, b = rings[i], rings[(i + 1) % n]
            for j in range(m):
                bm.faces.new((a[j], b[j], b[(j + 1) % m], a[(j + 1) % m]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    mesh_part(name, build, mat, root, lods=lods, smooth=True)


def bicycle(name, origin, yaw, frame, lean=0.0, lods=(0, 1), segs=RING):
    """A town bike parked upright, 1.75 m long along its own x (front wheel at +x) and its
    bars 0.56 m across: two wheels, a diamond frame in `frame`, a black saddle and bars.
    Far off it is its two wheels and its frame as a plate."""
    at0 = placed(origin, yaw)
    at = lambda x, y, z: at0(x, y + z * lean, z)  # leaning on its stand by `lean` per metre up
    tyre = flat_paint(f"{name}_tyre", (0.02, 0.02, 0.02), rough=0.8, grime=0.6)
    dark = flat_paint(f"{name}_dark", (0.025, 0.025, 0.025), rough=0.5, grime=0.0)
    wr, wb = 0.33, 0.54  # wheel radius, half the wheelbase
    for k, x in enumerate((-wb, wb)):
        ring(f"{name}_wheel_{k}", at, (x, 0, wr), wr - 0.025, 0.025, tyre, lods=lods, segs=segs)
        tube(f"{name}_hub_{k}", at(x, -0.04, wr), at(x, 0.04, wr), 0.03, dark, lods=lods[:1], seg=6)
    crank, seat, head = (-0.05, 0, 0.3), (-0.2, 0, 0.82), (0.36, 0, 0.84)
    for k, (a, b) in enumerate(((crank, seat), (crank, head), ((-0.17, 0, 0.74), (0.37, 0, 0.76)),
                               ((-wb, 0, wr), crank), ((-wb, 0, wr), (-0.18, 0, 0.76)), (head, (wb, 0, wr)))):
        tube(f"{name}_tube_{k}", at(*a), at(*b), 0.017, frame, lods=lods, seg=6)
    tube(f"{name}_post", at(-0.2, 0, 0.82), at(-0.24, 0, 0.92), 0.012, dark, lods=lods[:1], seg=6)
    box(f"{name}_saddle", (0.26, 0.13, 0.06), at(-0.24, 0, 0.94), dark, root, rot=(0, 0, yaw), lods=lods)
    tube(f"{name}_stem", at(*head), at(0.33, 0, 0.98), 0.018, dark, lods=lods, seg=6)
    tube(f"{name}_bars", at(0.32, -0.28, 0.98), at(0.32, 0.28, 0.98), 0.012, dark, lods=lods, seg=6)
    tube(f"{name}_chainring", at(-0.05, 0.05, 0.3), at(-0.05, 0.06, 0.3), 0.1, dark, lods=lods[:1], seg=10)


def sheffield(name, x, half, top, mat, lods=TIERS):
    """A Sheffield stand: one bent tube, an inverted U `2 * half` long along y and `top` high,
    standing at `x`, its corners bent round."""
    bend = 0.12
    pts = [(-half, 0.0), (-half, top - bend), (-half + bend * 0.3, top - bend * 0.3), (-half + bend, top),
           (half - bend, top), (half - bend * 0.3, top - bend * 0.3), (half, top - bend), (half, 0.0)]
    for k, ((ya, za), (yb, zb)) in enumerate(zip(pts, pts[1:])):
        tube(f"{name}_{k}", (x, ya, za), (x, yb, zb), 0.03, mat, lods=lods if k in (0, 3, 6) else lods[:3])


@kind
def bike_rack():
    """Four galvanised Sheffield stands 75 cm apart, each a bent tube on which bikes lean
    either side; three bikes parked, one of them on the end stand's outside."""
    hx, hy, hz = 1.5, 0.9, 0.5
    zinc = textured("rack_tube", "galvanised", chip=0.4, dirt=0.6, rise=0.5)
    for k, x in enumerate((-1.125, -0.375, 0.375, 1.125)):
        sheffield(f"stand_{k}", x, 0.4, 0.82, zinc)
    for name, x, colour, lean in (("bike_a", -1.24, (0.12, 0.016, 0.014), -0.02), ("bike_b", 0.49, (0.02, 0.05, 0.1), 0.03),
                                  ("bike_c", 1.24, (0.16, 0.16, 0.15), 0.025)):
        frame = flat_paint(f"{name}_frame", colour, rough=0.4, grime=0.4)
        bicycle(name, (x, 0.05 if x < 0 else -0.05, 0), math.pi / 2 * (1 if x < 0 else -1), frame, lean=lean,
                lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def playground_frame():
    """A timber play tower: a deck a metre up on four posts under a pitched plastic roof,
    a ladder up its -x end, a slide down to +x, a rope net climbing to the deck from -y,
    and monkey bars out to a gallows frame at +y."""
    hx, hy, hz = 2.0, 1.5, 1.25
    wood = timber("play_timber", (0.17, 0.11, 0.06), chip=0.3, dirt=0.5, rise=0.4, mottle=0.3)
    red = textured("play_red", "hard_plastic", colour=(0.42, 0.04, 0.025), chip=0.2, dirt=0.4, rise=0.3, streak=0.1)
    green = textured("play_green", "hard_plastic", colour=(0.05, 0.22, 0.06), chip=0.2, dirt=0.3, rise=0.3, streak=0.2)
    steel = flat_paint("play_steel", (0.32, 0.33, 0.34), rough=0.4, metal=0.6, wear=0.3, grime=0.5)
    rope = flat_paint("play_rope", (0.16, 0.12, 0.07), rough=0.9, grime=0.3)
    x0, x1, py, deck = -1.5, -0.3, 0.6, 1.0
    for k, (x, y) in enumerate(((x0, -py), (x1, -py), (x1, py), (x0, py))):
        box(f"post_{k}", (0.09, 0.09, 2.2), (x, y, 1.1), wood, root, bevel=0.01)
    for k in range(6):  # the deck's boards, across x
        box(f"deck_{k}", (0.2, 2 * py + 0.09, 0.04), (x0 + 0.1 + k * (x1 - x0 - 0.2) / 5, 0, deck), wood, root, lods=(0, 1))
    box("deck_far", (x1 - x0 + 0.09, 2 * py + 0.09, 0.04), ((x0 + x1) / 2, 0, deck), wood, root, lods=(2, 3))
    for k, s in enumerate((-1, 1)):  # a guard rail and a closed panel each long side
        box(f"rail_{k}", (x1 - x0, 0.06, 0.06), ((x0 + x1) / 2, s * py, deck + 0.75), wood, root, lods=(0, 1, 2))
        box(f"panel_{k}", (x1 - x0 - 0.12, 0.03, 0.5), ((x0 + x1) / 2, s * (py + 0.03), deck + 0.4), green, root, bevel=0.01)
    # the roof: two plastic slopes, the ridge along y
    ridge, eave, over = 2 * hz, 2.15, 0.12
    half = (x1 - x0) / 2 + over
    cx = (x0 + x1) / 2
    for k, s in enumerate((-1, 1)):
        slope = [(cx, ridge), (cx + s * half, eave), (cx + s * half, eave + 0.04), (cx, ridge + 0.04)]
        prism(f"roof_{k}", slope if s > 0 else slope[::-1], 2 * py + 0.3, (0, 0, -0.04), red, root)
    # the slide: a chute from the deck's +x edge to the ground at the box's end
    sx0, sx1, w = x1 + 0.05, hx - 0.05, 0.5
    run, fall = sx1 - sx0, deck + 0.02
    pitch = math.atan2(fall - 0.25, run - 0.4)
    length = math.hypot(run - 0.4, fall - 0.25)
    mid = (sx0 + (run - 0.4) / 2, 0, (fall + 0.25) / 2)
    box("chute", (length, w, 0.03), mid, red, root, rot=(0, pitch, 0))
    box("runout", (0.4, w, 0.03), (sx1 - 0.2, 0, 0.25), red, root)
    for k, s in enumerate((-1, 1)):
        box(f"chute_side_{k}", (length, 0.03, 0.16), (mid[0], s * w / 2, mid[2] + 0.07), red, root, rot=(0, pitch, 0),
            lods=(0, 1, 2))
        box(f"runout_side_{k}", (0.4, 0.03, 0.16), (sx1 - 0.2, s * w / 2, 0.32), red, root, lods=(0, 1, 2))
        box(f"runout_leg_{k}", (0.05, 0.05, 0.25), (sx1 - 0.08, s * (w / 2 - 0.05), 0.125), steel, root, lods=(0, 1))
    # the ladder up the -x end
    for k, s in enumerate((-1, 1)):
        tube(f"ladder_rail_{k}", (-hx + 0.02, s * 0.25, 0.0), (x0 - 0.02, s * 0.25, deck + 0.3), 0.025, steel)
    for k in range(1, 5):
        t = k / 5
        x, z = -hx + 0.02 + (x0 - 0.02 + hx - 0.02) * t, (deck + 0.3) * t
        tube(f"rung_{k}", (x, -0.25, z), (x, 0.25, z), 0.022, steel, lods=(0, 1, 2))
    # the net: ropes from the deck's -y edge to a ground beam at the box's edge
    box("net_beam", (x1 - x0, 0.1, 0.08), (cx, -hy + 0.05, 0.04), wood, root)
    for k in range(6):
        x = x0 + 0.05 + k * (x1 - x0 - 0.1) / 5
        tube(f"net_v_{k}", (x, -hy + 0.05, 0.08), (x, -py, deck), 0.012, rope, lods=(0, 1), seg=4)
    for k in range(1, 4):
        t = k / 4
        y, z = -hy + 0.05 + (hy - 0.05 - py) * t, 0.08 + (deck - 0.08) * t
        tube(f"net_h_{k}", (x0 + 0.05, y, z), (x1 - 0.05, y, z), 0.012, rope, lods=(0, 1), seg=4)
    # monkey bars out to a gallows frame at +y
    for k, x in enumerate((x0, x1)):
        tube(f"bars_side_{k}", (x, py, 2.0), (x, hy - 0.05, 2.0), 0.03, steel)
        tube(f"gallows_{k}", (x, hy - 0.05, 0.0), (x, hy - 0.05, 2.0), 0.035, steel)
    for k in range(1, 4):
        y = py + (hy - 0.05 - py) * k / 4
        tube(f"bar_{k}", (x0, y, 2.0), (x1, y, 2.0), 0.018, steel, lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def swing():
    """A two-seat swing: a steel beam on splayed A-frame legs at each end, a flat rubber
    seat and a toddler's cradle seat on chains."""
    hx, hy, hz = 1.7, 0.9, 1.1
    zinc = textured("swing_frame", "galvanised", chip=0.4, dirt=0.6, rise=0.5)
    chain = flat_paint("swing_chain", (0.25, 0.25, 0.25), rough=0.4, metal=0.8, grime=0.0)
    seat = flat_paint("swing_seat", (0.02, 0.02, 0.02), rough=0.8, grime=0.2)
    cradle = flat_paint("swing_cradle", (0.03, 0.12, 0.3), rough=0.6, grime=0.2)
    top, foot = 2 * hz - 0.05, hy - 0.04
    for k, s in enumerate((-1, 1)):
        x = s * (hx - 0.12)
        for j, sy in enumerate((-1, 1)):
            tube(f"leg_{k}_{j}", (x, sy * foot, 0.0), (x, 0, top), 0.04, zinc)
        tube(f"brace_{k}", (x, -foot * 0.45, 0.9), (x, foot * 0.45, 0.9), 0.025, zinc, lods=(0, 1, 2))
    tube("beam", (-hx, 0, top), (hx, 0, top), 0.05, zinc)
    for k, (x, mat, low) in enumerate(((-0.6, seat, 0.42), (0.6, cradle, 0.5))):
        for j, s in enumerate((-1, 1)):
            tube(f"chain_{k}_{j}", (x + s * 0.2, 0, top - 0.05), (x + s * 0.2, 0, low + 0.03), 0.007, chain, lods=(0,), seg=4)
            tube(f"chain_far_{k}_{j}", (x + s * 0.2, 0, top - 0.05), (x + s * 0.2, 0, low + 0.03), 0.012, chain,
                 lods=(1,), seg=4)
        if mat is seat:
            box(f"seat_{k}", (0.45, 0.17, 0.03), (x, 0, low), seat, root, bevel=0.008, lods=(0, 1, 2))
        else:  # a bucket seat with leg holes: a back, a front and two sides
            box(f"cradle_base_{k}", (0.42, 0.26, 0.04), (x, 0, low), cradle, root, bevel=0.01, lods=(0, 1, 2))
            for j, s in enumerate((-1, 1)):
                box(f"cradle_back_{k}_{j}", (0.42, 0.04, 0.24), (x, s * 0.13, low + 0.12), cradle, root, bevel=0.01,
                    lods=(0, 1, 2))
    return [hx, hy, hz]


# ------------------------------------------------------------------ China
def ebike(name, origin, yaw, colour, lods=(0, 1)):
    """A step-through electric scooter parked upright, 1.6 m long along its own x (front at
    +x): small wheels, a floorboard, a leg shield, a seat over the battery, bars and a basket."""
    at = placed(origin, yaw)
    body = flat_paint(f"{name}_body", colour, rough=0.35, wear=0.2, grime=0.5)
    dark = flat_paint(f"{name}_dark", (0.02, 0.02, 0.02), rough=0.7, grime=0.4)
    wr = 0.24
    for k, x in enumerate((-0.56, 0.58)):
        ring(f"{name}_wheel_{k}", at, (x, 0, wr), wr - 0.05, 0.05, dark, lods=lods, segs=COARSE_RING)
    rot = (0, 0, yaw)
    box(f"{name}_floor", (0.55, 0.3, 0.08), at(0.0, 0, 0.32), dark, root, rot=rot, lods=lods)
    box(f"{name}_tail", (0.62, 0.32, 0.36), at(-0.42, 0, 0.55), body, root, rot=rot, bevel=0.04, lods=lods)
    box(f"{name}_seat", (0.58, 0.28, 0.09), at(-0.38, 0, 0.78), dark, root, rot=rot, lods=lods)
    box(f"{name}_shield", (0.12, 0.42, 0.6), at(0.38, 0, 0.66), body, root, rot=(0, math.radians(-14), yaw), bevel=0.03,
        lods=lods)
    box(f"{name}_fender", (0.42, 0.14, 0.08), at(0.58, 0, 0.5), body, root, rot=rot, lods=lods[:1])
    box(f"{name}_head", (0.16, 0.32, 0.16), at(0.47, 0, 1.0), body, root, rot=rot, lods=lods)
    tube(f"{name}_bars", at(0.44, -0.33, 1.02), at(0.44, 0.33, 1.02), 0.014, dark, lods=lods, seg=6)
    box(f"{name}_basket", (0.26, 0.32, 0.2), at(0.64, 0, 0.9), dark, root, rot=rot, lods=lods[:1])


@kind
def bike_shed():
    """A residential estate's bicycle shed, 10 m by 2.5 m: a mono-pitch roof of blue
    profiled steel on grey steel posts, falling to the open front at -y; a mesh rail
    along the back; parked under it, a row of electric scooters and bicycles, and a
    charging box on the back posts."""
    hx, hy, hz = 5.0, 1.25, 1.2
    roof = textured("shed_roof", "roof_sheet", colour=(0.06, 0.16, 0.42), chip=0.3, dirt=0.2, rise=0.1, streak=0.2)
    post = flat_paint("shed_post", (0.28, 0.29, 0.3), rough=0.45, metal=0.5, wear=0.3, grime=0.6)
    meter = flat_paint("shed_meter", (0.5, 0.5, 0.48), rough=0.4, grime=0.3)
    back, front = hy - 0.12, -hy + 0.45  # the post lines
    top_back, top_front = 2 * hz - 0.06, 2.05
    fall = (top_back - top_front) / (back - front)
    roof_z = lambda y: top_front + (y - front) * fall
    for k in range(5):
        x = -hx + 0.15 + k * (2 * hx - 0.3) / 4
        for j, y in enumerate((front, back)):
            box(f"post_{k}_{j}", (0.08, 0.08, roof_z(y)), (x, y, roof_z(y) / 2), post, root)
        box(f"rafter_{k}", (0.06, 2 * hy - 0.04, 0.12), (x, 0, roof_z(0) - 0.08), post, root,
            rot=(math.atan(fall), 0, 0), lods=(0, 1, 2))
    for j, y in enumerate((front, back)):
        box(f"purlin_{j}", (2 * hx - 0.2, 0.06, 0.1), (0, y, roof_z(y) - 0.02), post, root, lods=(0, 1, 2))
    # the roof sheet overhangs to the box at both long sides; it falls toward -y
    run = 2 * hy / math.cos(math.atan(fall))
    box("roof", (2 * hx, run, 0.03), (0, 0, roof_z(0) + 0.05), roof, root, rot=(math.atan(fall), 0, 0))
    for k, z in enumerate((0.45, 1.1)):
        box(f"back_rail_{k}", (2 * hx - 0.3, 0.04, 0.05), (0, back, z), post, root, lods=(0, 1, 2))
    for k, x in enumerate((-hx + 0.15 + (2 * hx - 0.3) / 4, hx - 0.15 - (2 * hx - 0.3) / 4)):  # on the back posts
        box(f"meter_box_{k}", (0.4, 0.16, 0.55), (x, back - 0.12, 1.5), meter, root, bevel=0.02, lods=(0, 1, 2))
    colours = ((0.5, 0.5, 0.48), (0.25, 0.02, 0.02), (0.03, 0.08, 0.2), (0.4, 0.4, 0.38), (0.02, 0.02, 0.02),
               (0.12, 0.25, 0.3))
    slots = [-hx + 0.6 + k * 1.05 for k in range(9)]
    for k, x in enumerate(slots):
        if k in (2, 6):
            frame = flat_paint(f"bike_{k}_frame", colours[k % 6], rough=0.4, grime=0.4)
            bicycle(f"bike_{k}", (x, 0.15, 0), -math.pi / 2 + rng.uniform(-0.08, 0.08), frame, segs=COARSE_RING)
        elif k != 4:
            ebike(f"ebike_{k}", (x, 0.1, 0), -math.pi / 2 + rng.uniform(-0.1, 0.1), colours[k % 6])
    return [hx, hy, hz]


@kind
def pingpong_table():
    """The estate's table-tennis table, cast in concrete: a 2.74 m by 1.52 m top on two
    solid piers, its face painted and lined, and a steel plate for a net."""
    hx, hy, hz = 1.37, 0.76, 0.45
    conc = textured("table_concrete", "concrete", chip=0.7, dirt=0.5, rise=0.3, streak=0.3, mottle=0.2)
    face = textured("table_face", "concrete", colour=(0.04, 0.12, 0.1), chip=0.15, dirt=0.0, rise=0.1, streak=0.0, grain=0.0)
    line = flat_paint("table_line", (0.6, 0.6, 0.57), rough=0.6, wear=0.5, grime=0.0)
    steel = flat_paint("table_net", (0.03, 0.033, 0.035), rough=0.5, wear=0.3, grime=0.0)
    top, slab = 0.76, 0.08
    box("slab", (2 * hx, 2 * hy, slab - 0.004), (0, 0, top - slab / 2 - 0.002), conc, root, bevel=0.012)
    box("face", (2 * hx - 0.02, 2 * hy - 0.02, 0.004), (0, 0, top - 0.002), face, root)
    for k, s in enumerate((-1, 1)):
        box(f"pier_{k}", (0.16, 1.1, top - slab), (s * 0.85, 0, (top - slab) / 2), conc, root, bevel=0.01)
        box(f"line_end_{k}", (0.02, 2 * hy - 0.04, 0.003), (s * (hx - 0.02), 0, top + 0.0015), line, root, lods=(0, 1))
        box(f"line_side_{k}", (2 * hx - 0.04, 0.02, 0.003), (0, s * (hy - 0.02), top + 0.0015), line, root, lods=(0, 1))
    box("line_mid", (2 * hx - 0.04, 0.006, 0.003), (0, 0, top + 0.0015), line, root, lods=(0,))
    box("net", (0.006, 2 * hy - 0.04, 0.13), (0, 0, top + 0.08), steel, root)
    for k, s in enumerate((-1, 1)):
        box(f"net_post_{k}", (0.04, 0.03, 0.15), (0, s * (hy - 0.015), top + 0.075), steel, root, lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def outdoor_gym():
    """A double air walker from the estate's fitness path: a blue steel frame, a yellow
    top beam, and two yellow pendulum walkers hanging from it on their own axles, one
    stride forward, one back."""
    hx, hy, hz = 0.8, 0.5, 0.9
    blue = textured("gym_blue", "enamel", colour=(0.02, 0.09, 0.32), chip=0.5, dirt=0.5, rise=0.4)
    yellow = textured("gym_yellow", "enamel", colour=(0.55, 0.36, 0.02), chip=0.6, dirt=0.4, rise=0.4)
    plate = flat_paint("gym_plate", (0.2, 0.2, 0.2), rough=0.6, metal=0.5, wear=0.5, grime=0.6)
    top = 2 * hz - 0.05
    for k, s in enumerate((-1, 1)):
        x = s * (hx - 0.1)
        tube(f"post_{k}", (x, 0, 0.0), (x, 0, top), 0.065, blue, seg=12)
        cyl(f"flange_{k}", 0.1, 0.02, (x, 0, 0.01), "Z", plate, root, seg=10, lods=(0, 1))
        cyl(f"cap_{k}", 0.065, 0.06, (x, 0, top + 0.02), "Z", yellow, root, seg=12, lods=(0, 1, 2))
    tube("beam", (-hx + 0.1, 0, top - 0.05), (hx - 0.1, 0, top - 0.05), 0.045, yellow, seg=12)
    tube("mid_post", (0, 0, 0.0), (0, 0, top - 0.05), 0.05, blue, seg=12)
    axle = 1.4
    tube("axle", (-hx + 0.1, 0, axle), (hx - 0.1, 0, axle), 0.035, blue, lods=(0, 1, 2))
    for k, s in enumerate((-1, 1)):
        x = s * 0.4
        for j, (swing, dx) in enumerate(((0.32, -0.12), (-0.28, 0.12))):
            foot = (x + dx, swing, 0.28)
            tube(f"walker_{k}_{j}", (x + dx, 0, axle), foot, 0.04, yellow, lods=(0, 1, 2))
            box(f"pedal_{k}_{j}", (0.26, 0.36, 0.05), (foot[0], foot[1], foot[2] - 0.03), yellow, root, bevel=0.01,
                lods=(0, 1, 2))
        tube(f"handle_stem_{k}", (x, 0, top - 0.05), (x, -0.18, 1.2), 0.035, yellow, lods=(0, 1, 2))
        tube(f"handle_{k}", (x - 0.22, -0.18, 1.2), (x + 0.22, -0.18, 1.2), 0.03, yellow, lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def laundry_poles():
    """A drying rack in the court: two steel posts with T-heads, two bars between them,
    quilts thrown over one bar to air and shirts on hangers along the other."""
    hx, hy, hz = 3.0, 0.15, 1.0
    post = flat_paint("pole_paint", (0.07, 0.11, 0.08), rough=0.5, metal=0.4, wear=0.5, grime=0.6)
    hanger = flat_paint("hanger", (0.4, 0.38, 0.35), rough=0.5, grime=0.0)
    top, bar = 2 * hz - 0.03, hy - 0.04
    for k, s in enumerate((-1, 1)):
        x = s * (hx - 0.04)
        tube(f"post_{k}", (x, 0, 0.0), (x, 0, top), 0.03, post)
        tube(f"head_{k}", (x, -bar, top - 0.02), (x, bar, top - 0.02), 0.02, post, lods=(0, 1, 2))
    for j, y in enumerate((-bar, bar)):
        tube(f"bar_{j}", (-hx, y, top - 0.04), (hx, y, top - 0.04), 0.015, post, lods=(0, 1, 2))
    at_bar = lambda x: top - 0.04
    # quilts thrown over the -y bar: one cloth down each side of it
    for name, x0, x1, colour, seed in (("quilt_a", -2.8, -1.05, (0.5, 0.08, 0.1), 0.4),
                                       ("quilt_b", 0.35, 1.9, (0.3, 0.42, 0.55), 1.7)):
        hang(f"{name}_front", -bar - 0.025, at_bar, x0, x1, 1.05, colour, seed, pegs=False)
        hang(f"{name}_back", -bar + 0.025, at_bar, x0, x1, 0.9, tuple(0.9 * c for c in colour), seed + 0.5, pegs=False)
    # shirts on hangers along the +y bar
    for k, (x, colour) in enumerate(((-0.8, (0.62, 0.62, 0.6)), (-0.25, (0.12, 0.2, 0.42)), (2.2, (0.55, 0.42, 0.12)),
                                     (2.65, (0.62, 0.6, 0.58)), (-1.55, (0.08, 0.08, 0.09)))):
        drop = top - 0.18
        hang(f"shirt_{k}", bar, lambda _x: drop, x - 0.2, x + 0.2, 0.62, colour, 0.9 * k, pegs=False)
        tube(f"hanger_{k}_a", (x - 0.2, bar, drop - 0.01), (x, bar, drop + 0.08), 0.005, hanger, lods=(0,), seg=4)
        tube(f"hanger_{k}_b", (x + 0.2, bar, drop - 0.01), (x, bar, drop + 0.08), 0.005, hanger, lods=(0,), seg=4)
        tube(f"hanger_{k}_c", (x, bar, drop + 0.08), (x, bar, top - 0.04), 0.005, hanger, lods=(0,), seg=4)
    return [hx, hy, hz]


@kind
def bench_china():
    """China's look of the shared bench: red-brown lacquered slats, seat and back, on two
    pale granite ends."""
    hx, hy, hz = 0.9, 0.3, 0.42
    stone = textured("granite", "concrete", colour=(0.42, 0.4, 0.38), chip=0.4, dirt=0.4, rise=0.3, streak=0.1, mottle=0.2)
    wood = timber("lacquer", (0.16, 0.035, 0.02), chip=0.5, dirt=0.4, rise=0.4, mottle=0.2, streak=0.2)
    for k, s in enumerate((-1, 1)):
        x = s * (hx - 0.07)
        box(f"end_{k}", (0.14, 0.5, 0.4), (x, -0.04, 0.2), stone, root, bevel=0.015)
        box(f"back_post_{k}", (0.12, 0.1, 0.84), (x, hy - 0.06, 0.42), stone, root, bevel=0.012,
            rot=(math.radians(-5), 0, 0))
    for k in range(4):
        box(f"seat_slat_{k}", (2 * hx - 0.3, 0.11, 0.04), (0, -0.25 + 0.13 * k, 0.42), wood, root, bevel=0.006,
            lods=(0, 1))
    box("seat_far", (2 * hx - 0.3, 0.5, 0.04), (0, -0.055, 0.42), wood, root, lods=(2, 3))
    for k in range(3):
        box(f"back_slat_{k}", (2 * hx - 0.26, 0.035, 0.09), (0, hy - 0.08 + 0.012 * k, 0.55 + 0.11 * k), wood, root,
            rot=(math.radians(-8), 0, 0), bevel=0.006, lods=(0, 1))
    box("back_far", (2 * hx - 0.26, 0.035, 0.32), (0, hy - 0.07, 0.66), wood, root, rot=(math.radians(-8), 0, 0),
        lods=(2, 3))
    return [hx, hy, hz]


@kind
def bins_china():
    """China's look of the shared bins: a sorting point, four lidded bins in a row in the
    sorting colours (red, blue, green, grey), each with a pale label panel on both faces."""
    hx, hy, hz = 0.65, 0.38, 0.55
    label = flat_paint("bin_label", (0.6, 0.6, 0.57), rough=0.5, grime=0.2)
    wheel = flat_paint("bin_wheel", (0.02, 0.02, 0.02), rough=0.8, grime=0.4)
    w, d, h = 0.3, 0.56, 0.98
    colours = ((0.38, 0.03, 0.025), (0.03, 0.12, 0.38), (0.03, 0.22, 0.07), (0.15, 0.155, 0.155))
    for k, colour in enumerate(colours):
        x = -hx + 0.01 + w / 2 + k * (w + 0.0233)
        body = textured(f"bin_{k}", "hard_plastic", colour=colour, chip=0.15, dirt=0.5, rise=0.4, streak=0.2)
        box(f"bin_{k}_body", (w - 0.04, d - 0.08, h - 0.06), (x, 0.0, 0.04 + (h - 0.06) / 2), body, root, bevel=0.02,
            taper=(1.12, 1.12))
        box(f"bin_{k}_lid", (w + 0.01, d + 0.04, 0.05), (x, 0.0, h + 0.01), body, root, bevel=0.012,
            rot=(math.radians(-2 if k % 2 else 2), 0, 0))
        box(f"bin_{k}_handle", (w - 0.08, 0.03, 0.03), (x, d / 2 + 0.01, h + 0.05), body, root, lods=(0, 1))
        for j, sy in enumerate((-1, 1)):
            box(f"bin_{k}_label_{j}", (w - 0.1, 0.01, 0.22), (x, sy * (d / 2 - 0.015), 0.62), label, root, lods=(0, 1, 2),
                rot=(sy * math.radians(-3.5), 0, 0))
            cyl(f"bin_{k}_wheel_{j}", 0.05, 0.04, (x + sy * 0.09, d / 2 - 0.08, 0.05), "X", wheel, root, seg=10,
                lods=(0, 1))
    return [hx, hy, hz]


# ------------------------------------------------------------------ New York
@kind
def chainlink_fence():
    """A ball court's tall chain-link fence: galvanised fabric between line posts 3 m
    apart, a top rail and a mid rail, a tension wire at the foot. The end posts
    straddle the module's ends, so repeated modules share them."""
    hx, hy, hz = 1.5, 0.05, 1.8
    zinc = textured("fence_tube", "galvanised", chip=0.4, dirt=0.6, rise=0.5)
    fabric = textured("fence_fabric", "chain_link", chip=0.3, dirt=0.5, rise=0.4, coverage=("cutout", 0.5))
    top = 2 * hz
    for k, s in enumerate((-1, 1)):
        cyl(f"post_{k}", 0.045, top, (s * hx, 0, top / 2), "Z", zinc, root, seg=10)
        cyl(f"post_cap_{k}", 0.055, 0.06, (s * hx, 0, top - 0.03), "Z", zinc, root, seg=10, lods=(0, 1, 2))
    for k, z in enumerate((top - 0.05, top / 2)):
        tube(f"rail_{k}", (-hx, 0.03, z), (hx, 0.03, z), 0.021, zinc, lods=(0, 1, 2))
    tube("tension_wire", (-hx, 0.01, 0.06), (hx, 0.01, 0.06), 0.006, zinc, lods=(0,), seg=4)
    sheet("fabric", 2 * hx - 0.09, top - 0.1, (0, 0.0, 0.03), fabric, root)
    return [hx, hy, hz]


@kind
def basketball_hoop():
    """A park hoop facing -y: a steel pole at the box's back, a short gooseneck out to a
    perforated steel backboard with a painted target square, an orange rim and a chain net."""
    hx, hy, hz = 0.9, 0.6, 1.95
    pole_m = textured("hoop_pole", "enamel", colour=(0.03, 0.06, 0.04), chip=0.7, dirt=0.6, rise=0.6, streak=0.3)
    board_m = textured("hoop_board", "perforated", colour=(0.06, 0.07, 0.06), chip=0.4, dirt=0.0, rise=0.0,
                       coverage=("cutout", 0.5))
    paint = flat_paint("hoop_paint", (0.55, 0.55, 0.52), rough=0.6, wear=0.6, grime=0.0)
    rim_m = flat_paint("hoop_rim", (0.5, 0.12, 0.02), rough=0.5, metal=0.3, wear=0.4, grime=0.0)
    chain = flat_paint("hoop_chain", (0.3, 0.3, 0.3), rough=0.4, metal=0.8, grime=0.0)
    py, by, top = hy - 0.08, 0.05, 2 * hz  # the pole's line, the board's
    cyl("pole", 0.075, 3.2, (0, py, 1.6), "Z", pole_m, root, seg=14)
    cyl("pole_base", 0.12, 0.08, (0, py, 0.04), "Z", pole_m, root, seg=12, lods=(0, 1, 2))
    tube("gooseneck", (0, py, 3.15), (0, by + 0.08, 3.3), 0.06, pole_m)
    for k, s in enumerate((-1, 1)):
        tube(f"strut_{k}", (0, py, 2.7), (s * 0.4, by + 0.04, 2.95), 0.03, pole_m, lods=(0, 1, 2))
    bw, b0, b1 = 2 * hx - 0.02, 2.75, top - 0.02
    sheet("board", bw, b1 - b0, (0, by, b0), board_m, root)
    for k, (w, h, loc) in enumerate((((bw, 0.04, (0, by - 0.01, b1 - 0.02))), ((bw, 0.04, (0, by - 0.01, b0 + 0.02))),
                                     ((0.04, b1 - b0, (-bw / 2 + 0.02, by - 0.01, (b0 + b1) / 2))),
                                     ((0.04, b1 - b0, (bw / 2 - 0.02, by - 0.01, (b0 + b1) / 2))))):
        box(f"frame_{k}", (w, 0.025, h), loc, pole_m, root, lods=(0, 1, 2))
    rz = 3.05  # the rim's height
    for k, (w, h, x, z) in enumerate(((0.59, 0.05, 0, rz + 0.43), (0.59, 0.05, 0, rz + 0.02), (0.05, 0.45, -0.27, rz + 0.22),
                                      (0.05, 0.45, 0.27, rz + 0.22))):
        box(f"square_{k}", (w, 0.012, h), (x, by - 0.012, z), paint, root, lods=(0, 1))
    rc, rr = by - 0.15 - 0.23, 0.23
    flat = lambda x, y, z: (x, z, y)  # a ring in the part's XZ plane, laid flat
    ring("rim", flat, (0, rz, rc), rr, 0.012, rim_m, lods=(0, 1, 2))
    box("rim_plate", (0.2, 0.15, 0.1), (0, by - 0.075, rz - 0.03), rim_m, root, lods=(0, 1, 2))
    for k in range(10):  # the net: chains hanging in a tapering cone
        a = 2 * math.pi * (k + 0.5) / 10
        tube(f"net_{k}", (rr * math.cos(a), rc + rr * math.sin(a), rz),
             (0.6 * rr * math.cos(a + 0.4), rc + 0.6 * rr * math.sin(a + 0.4), rz - 0.42), 0.006, chain, lods=(0,), seg=4)
    return [hx, hy, hz]


def _ribbon(bm, pts, width, z):
    """A flat painted line `width` wide along the polyline `pts` (x, y) at height `z`."""
    left, right = [], []
    for i, (x, y) in enumerate(pts):
        a, b = pts[max(0, i - 1)], pts[min(len(pts) - 1, i + 1)]
        tx, ty = b[0] - a[0], b[1] - a[1]
        n = math.hypot(tx, ty) or 1.0
        nx, ny = -ty / n * width / 2, tx / n * width / 2
        left.append(bm.verts.new((x + nx, y + ny, z)))
        right.append(bm.verts.new((x - nx, y - ny, z)))
    for i in range(len(pts) - 1):
        f = bm.faces.new((right[i], right[i + 1], left[i + 1], left[i]))
        if f.normal.z < 0:
            f.normal_flip()


def _arc(cx, cy, r, a0, a1, step=0.25):
    n = max(2, math.ceil(abs(a1 - a0) * r / step))
    return [(cx + r * math.cos(a0 + (a1 - a0) * i / n), cy + r * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]


@kind
def basketball_court():
    """Half a basketball court, 14 m from its baseline at -x to the centre line at +x and
    15 m across: a green acrylic pad on the paving, a red key, and white lines (the key,
    the free-throw circle, the three-point line, half the centre circle). Its hoop stands
    behind the baseline (`basketball_hoop`), so the painted rim point sits just inside it;
    two halves meet at their centre lines as one court."""
    hx, hy, hz = 7.0, 7.5, 0.02
    # smooth acrylic paint, not grained asphalt, which reads as turf
    pad = textured("court_pad", "marking_paint", colour=(0.045, 0.12, 0.07), chip=0.0, dirt=0.15, rise=0.1,
                   streak=0.0, mottle=0.08, grain=0.0)
    key = textured("court_key", "marking_paint", colour=(0.2, 0.045, 0.035), chip=0.0, dirt=0.1, rise=0.1,
                   streak=0.0, grain=0.0)
    white = flat_paint("court_line", (0.62, 0.62, 0.6), rough=0.6, grime=0.0)
    top = 2 * hz - 0.008
    box("pad", (2 * hx, 2 * hy, top), (0, 0, top / 2), pad, root)
    rx = -hx + 0.3  # the rim point, as near the hoop behind the baseline as the court allows
    kw, kl, w = 4.9, 5.8, 0.05  # the key's width and length; a line's width
    box("key", (kl, kw, 0.003), (-hx + kl / 2, 0, top + 0.0015), key, root)
    lz = top + 0.005
    three = 6.75
    corner = math.asin((hy - 0.9) / three)  # where the arc meets the straight corner lines

    def lines(bm, lod):
        if lod == 3:
            return False
        lw = w if lod < 2 else 0.08
        step = (0.2, 0.4, 0.8)[lod]
        _ribbon(bm, [(-hx + lw / 2, -hy), (-hx + lw / 2, hy)], lw, lz)  # baseline
        _ribbon(bm, [(hx - lw / 2, -hy), (hx - lw / 2, hy)], lw, lz)  # centre line
        for s in (-1, 1):
            _ribbon(bm, [(-hx, s * (hy - lw / 2)), (hx, s * (hy - lw / 2))], lw, lz)  # sidelines
            _ribbon(bm, [(-hx, s * kw / 2), (-hx + kl, s * kw / 2)], lw, lz)  # the key's sides
            _ribbon(bm, [(-hx, s * (hy - 0.9)), (rx + three * math.cos(corner), s * (hy - 0.9))], lw, lz)
        _ribbon(bm, [(-hx + kl, -kw / 2), (-hx + kl, kw / 2)], lw, lz)  # free-throw line
        _ribbon(bm, _arc(-hx + kl, 0, 1.8, -math.pi / 2, math.pi / 2, step), lw, lz)
        _ribbon(bm, _arc(rx, 0, three, -corner, corner, step), lw, lz)
        _ribbon(bm, _arc(rx, 0, 1.25, -math.pi / 2, math.pi / 2, step), lw, lz)  # restricted area
        _ribbon(bm, _arc(hx, 0, 1.8, math.pi / 2, 3 * math.pi / 2, step), lw, lz)  # half the centre circle

    mesh_part("lines", lines, white, root)
    return [hx, hy, hz]


@kind
def garage_row():
    """A row of three lock-up garages in painted concrete block, 9 m by 6 m: a flat roof
    falling to the back under a coping, piers between the bays, and a steel roller
    shutter in each, facing -y; a lamp over each door."""
    hx, hy, hz = 4.5, 3.0, 1.3
    block = textured("garage_block", "concrete_block", colour=(0.42, 0.4, 0.36), chip=0.4, dirt=0.25, rise=0.5, streak=0.5,
                     mottle=0.1)
    roof = flat_paint("garage_roof", (0.07, 0.07, 0.068), rough=0.95, grime=0.0)  # felt: a third recipe would near the cap
    lamp = flat_paint("garage_lamp", (0.04, 0.04, 0.04), rough=0.5, grime=0.0)
    front, back = 2 * hz - 0.06, 2 * hz - 0.24
    prism("shell", [(-hy, 0.0), (hy, 0.0), (hy, back), (-hy, front)], 2 * hx, (0, 0, 0), block, root,
          rot=(0, 0, math.pi / 2))
    prism("roof", [(-hy, front), (hy, back), (hy, back + 0.05), (-hy, front + 0.05)], 2 * hx, (0, 0, 0), roof, root,
          rot=(0, 0, math.pi / 2))
    box("coping", (2 * hx, 0.2, 0.06), (0, -hy + 0.1, front + 0.03), block, root, lods=(0, 1, 2))
    bay, door_w, door_h = 2 * hx / 3, 2.4, 2.1
    shades = ((0.22, 0.22, 0.21), (0.11, 0.14, 0.17), (0.17, 0.12, 0.08))
    for k in range(3):
        x = -hx + bay * (k + 0.5)
        door = textured(f"shutter_{k}", "roller_slats", colour=shades[k], chip=0.7, dirt=0.6, rise=0.6, streak=0.5)
        box(f"shutter_{k}", (door_w, 0.04, door_h), (x, -hy - 0.005, door_h / 2), door, root)
        box(f"box_{k}", (door_w + 0.1, 0.12, 0.22), (x, -hy - 0.04, door_h + 0.11), door, root, lods=(0, 1, 2))
        box(f"lamp_{k}", (0.16, 0.1, 0.12), (x, -hy - 0.05, door_h + 0.33), lamp, root, lods=(0, 1))
    for k in range(4):  # the piers' faces between and beside the doors
        box(f"pier_{k}", (0.12, 0.06, front - 0.02), (-hx + bay * k + (0.06 if k == 0 else -0.06 if k == 3 else 0), -hy - 0.01,
            (front - 0.02) / 2), block, root, lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def dumpster():
    """A front-load dumpster: a steel box in dark green with a sloped front, two black
    plastic lids (one not quite shut), fork pockets on its sides and lifting lugs."""
    hx, hy, hz = 1.0, 0.6, 0.65
    steel_m = textured("dumpster_paint", "enamel", colour=(0.03, 0.08, 0.05), chip=0.5, dirt=0.5, rise=0.7, streak=0.5)
    lid_m = textured("dumpster_lid", "hard_plastic", colour=(0.02, 0.02, 0.02), chip=0.1, dirt=0.4, rise=0.2, streak=0.2)
    rim = 2 * hz - 0.1
    prism("body", [(-hy + 0.25, 0.08), (hy, 0.08), (hy, rim), (-hy, rim)], 2 * hx - 0.14, (0, 0, 0), steel_m, root, bevel=0.02,
          rot=(0, 0, math.pi / 2))
    for k, s in enumerate((-1, 1)):
        box(f"skid_{k}", (2 * hx - 0.3, 0.1, 0.08), (0, s * 0.3, 0.04), steel_m, root, lods=(0, 1, 2))
        box(f"pocket_{k}", (0.07, 0.9, 0.16), (s * (hx - 0.035), 0.05, 0.9), steel_m, root, bevel=0.01)
        box(f"lug_{k}", (0.06, 0.12, 0.12), (s * (hx - 0.1), -hy + 0.04, rim - 0.12), steel_m, root, lods=(0, 1))
        # the lids: one shut, one sprung a little open
        lift = 0.0 if s < 0 else math.radians(4)
        box(f"lid_{k}", (hx - 0.12, 2 * hy - 0.02, 0.06), (s * (hx / 2 - 0.05), 0.0, rim + 0.04 + 0.03 * (s > 0)), lid_m,
            root, bevel=0.012, rot=(-lift, 0, 0))
    box("rim_band", (2 * hx - 0.12, 2 * hy, 0.06), (0, 0, rim - 0.03), steel_m, root, lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def bench_new_york():
    """New York's look of the shared bench: the parks' slatted bench, its seat and back
    one curve of oak slats on black cast-iron ends."""
    hx, hy, hz = 0.9, 0.3, 0.42
    wood = timber("ny_slat", (0.2, 0.13, 0.07), chip=0.3, dirt=0.5, rise=0.5, mottle=0.3)
    iron = textured("ny_iron", "enamel", colour=(0.015, 0.017, 0.016), chip=0.6, dirt=0.6, rise=0.5)
    # the slats' line in profile, front lip to the top of the back: (y, z, tilt)
    curve = [(-0.27, 0.42, -0.3), (-0.18, 0.44, 0.0), (-0.08, 0.44, 0.0), (0.02, 0.43, 0.15), (0.11, 0.46, 1.0),
             (0.17, 0.56, 1.35), (0.21, 0.67, 1.4), (0.25, 0.78, 1.45)]
    for k, (y, z, tilt) in enumerate(curve):
        box(f"slat_{k}", (2 * hx, 0.075, 0.03), (0, y, z), wood, root, bevel=0.005, rot=(tilt, 0, 0), lods=(0, 1))
    box("seat_far", (2 * hx, 0.42, 0.04), (0, -0.07, 0.43), wood, root, lods=(2, 3))
    box("back_far", (2 * hx, 0.04, 0.38), (0, 0.2, 0.66), wood, root, rot=(math.radians(-15), 0, 0), lods=(2, 3))
    for k, x in enumerate((-hx + 0.12, 0.0, hx - 0.12)):
        tube(f"front_leg_{k}", (x, -0.24, 0.0), (x, -0.2, 0.41), 0.025, iron, lods=(0, 1, 2))
        tube(f"back_leg_{k}", (x, 0.18, 0.0), (x, 0.1, 0.42), 0.025, iron, lods=(0, 1, 2))
        tube(f"seat_rail_{k}", (x, -0.26, 0.4), (x, 0.1, 0.42), 0.02, iron, lods=(0, 1, 2))
        tube(f"back_rail_{k}", (x, 0.1, 0.42), (x, 0.25, 0.78), 0.02, iron, lods=(0, 1, 2))
        if k != 1:
            tube(f"arm_{k}", (x, -0.24, 0.62), (x, 0.17, 0.6), 0.022, iron, lods=(0, 1, 2))
            tube(f"arm_post_{k}", (x, -0.22, 0.41), (x, -0.24, 0.62), 0.02, iron, lods=(0, 1))
    return [hx, hy, hz]


@kind
def bins_new_york():
    """New York's look of the shared bins: the parks' green wire litter basket, filled to
    the brim, and black rubbish bags heaped beside it."""
    hx, hy, hz = 0.65, 0.38, 0.55
    mesh = textured("basket_mesh", "chain_link", colour=(0.03, 0.12, 0.05), chip=0.6, dirt=0.4, rise=0.4,
                    coverage=("cutout", 0.5))
    band = textured("basket_band", "enamel", colour=(0.03, 0.12, 0.05), chip=0.7, dirt=0.5, rise=0.4)
    bag = flat_paint("bin_bag", (0.014, 0.014, 0.016), rough=0.5, grime=0.3)
    litter = flat_paint("litter", (0.16, 0.14, 0.11), rough=0.8, grime=0.0)
    bx, r, h = -hx + 0.31, 0.3, 0.92
    cyl("basket", r, h - 0.04, (bx, 0, 0.02 + (h - 0.04) / 2), "Z", mesh, root, seg=20, caps=False)
    for k, z in enumerate((0.04, h * 0.55, h - 0.02)):
        cyl(f"band_{k}", r + 0.01, 0.04, (bx, 0, z), "Z", band, root, seg=20, lods=(0, 1, 2) if k else TIERS, caps=False)

    def rubbish(bm, lod):
        lump(bm, lod, (bx, 0, h - 0.1), (0.27, 0.27, 0.2), 0.3, 2.0)

    mesh_part("rubbish", rubbish, litter, root, lods=(0, 1, 2))

    def bags(bm, lod):
        # slumped sacks: wider than tall, lumpy with what is in them, a tied neck on top
        for k, (x, y, z, rx, ry, rz) in enumerate(((0.26, -0.13, 0.2, 0.24, 0.24, 0.2), (0.45, 0.14, 0.19, 0.19, 0.23, 0.19),
                                                    (0.33, 0.02, 0.5, 0.2, 0.21, 0.18), (0.14, 0.2, 0.18, 0.15, 0.17, 0.18))):
            lump(bm, lod, (x, y, z), (rx, ry, rz), 0.32, 11.0 + k)
            lump(bm, lod, (x + 0.03, y, z + rz * 0.95), (0.05, 0.05, 0.07), 0.2, 21.0 + k)

    mesh_part("bags", bags, bag, root, smooth=True)
    return [hx, hy, hz]


box_half = KINDS[KIND]()
rest_on_ground()
bpy.context.view_layer.update()
finish(ao_distance=1.0, ao_strength=0.6, ao_rays=10, paint_scale=1.3)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT, texture_px=TEXTURE_PX)
