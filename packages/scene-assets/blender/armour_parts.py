"""Armour added to a hull or turret: fields of armour tiles, side skirts and
track fenders kept inside the hull's outline, slat armour, bolted applique
panels and weld beads.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import hull_shape as HS  # noqa: E402
from parts import box, cyl, prism  # noqa: E402
from vehicle_parts import ALL, FINE, MID, NEAR, place, tube_part  # noqa: E402

# The narrow bevel every armour plate's edge carries in tiers 0-1, so the
# edge catches the light. A plate thinner than about twice this takes less
# (`armour_tiles`, `track_fender`), so the bevel never eats the plate.
PLATE_BEVEL = 0.02
# A side panel the outline leaves lower than this is dropped, so a run of
# skirts stops where the glacis takes over rather than ending in a sliver.
MIN_PANEL_M = 0.12


def armour_tiles(name, loc, size, grid, depth, mats, parent, gap=0.025, bolts=True, rot=(0, 0, 0), lods=ALL):
    """A field of bolted armour tiles (explosive reactive armour, applique
    modules) lying in the local XY plane on a face at `loc`, standing `depth`
    proud along local +Z: `size` (x, y) of the whole field, `grid` (columns
    along x, rows along y). Each tile is chunky, its edges bevelled at
    PLATE_BEVEL, with a bolt head at each corner close up. Returns its
    meshes. Roles: paint, steel."""
    cols, rows = grid
    tx, ty = size[0] / cols, size[1] / rows
    made = []
    for i in range(cols):
        for j in range(rows):
            cx = -size[0] / 2 + tx * (i + 0.5)
            cy = -size[1] / 2 + ty * (j + 0.5)
            made += box(f"{name}_{i}_{j}", (tx - gap, ty - gap, depth), (cx, cy, depth / 2), mats["paint"], parent,
                        bevel=min(PLATE_BEVEL, depth * 0.45), lods=lods)
            if bolts:
                for k in range(4):
                    bx = cx + (k % 2 - 0.5) * (tx - gap) * 0.7
                    by = cy + (k // 2 - 0.5) * (ty - gap) * 0.7
                    made += cyl(f"{name}_{i}_{j}_bolt_{k}", 0.018, 0.016, (bx, by, depth + 0.006), "Z", mats["steel"],
                                parent, seg=6, lods=FINE)
    return place(made, loc, rot)


def slat_armour(name, loc, size, mats, parent, spacing=0.11, bar=0.022, rot=(0, 0, 0)):
    """A bar (slat) armour panel standing in the local XZ plane at `loc` (its
    foot's centre), `size` (x length, z height): a frame of two rails and two
    end posts, vertical slats `spacing` apart between them. Returns its meshes.
    Roles: paint, dark."""
    sx, sz = size
    made = []
    for k, z in enumerate((bar, sz - bar)):
        made += box(f"{name}_rail_{k}", (sx, bar * 2.2, bar * 2), (0, 0, z), mats["paint"], parent, bevel=0.006)
    for k, x in enumerate((-sx / 2, sx / 2)):
        made += box(f"{name}_post_{k}", (bar * 2.2, bar * 2.2, sz), (x, 0, sz / 2), mats["paint"], parent,
                    bevel=0.006, lods=MID)
    n = max(1, int(sx / spacing) - 1)
    for j in range(n):
        x = -sx / 2 + sx * (j + 1) / (n + 1)
        made += box(f"{name}_slat_{j}", (bar, bar * 1.6, sz - bar * 3), (x, 0, sz / 2), mats["dark"], parent,
                    lods=NEAR)
    return place(made, loc, rot)


def bolted_panel(name, loc, size, mats, parent, bolts=(3, 2), rot=(0, 0, 0), bevel=PLATE_BEVEL, lods=MID):
    """An applique armour plate bolted onto a face: a slab `size` (x, y, z
    thickness) lying in its local XY plane on the face at `loc` (its back on
    the face, facing local +Z), its edges bevelled `bevel` but never less
    than PLATE_BEVEL, with chunky bolt heads in a ring `bolts` (along x,
    along y) round its border. Turn it onto a side or a glacis with
    `rot`. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_plate", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=max(bevel, PLATE_BEVEL), lods=lods)
    nx, ny = bolts
    spots = set()
    for i in range(nx):
        for j in range(ny):
            if i in (0, nx - 1) or j in (0, ny - 1):
                spots.add((i, j))
    for i, j in sorted(spots):
        x = -sx / 2 + 0.07 + (sx - 0.14) * (i / max(1, nx - 1))
        y = -sy / 2 + 0.07 + (sy - 0.14) * (j / max(1, ny - 1))
        made += cyl(name=f"{name}_bolt_{i}_{j}", r=0.026, depth=0.03, loc=(x, y, sz + 0.012), mat=mats["steel"],
                    parent=parent, seg=6, bevel=0.006, lods=FINE)
    return place(made, loc, rot)


def weld_line(name, points, mats, parent, radius=0.007, loc=(0, 0, 0), rot=(0, 0, 0)):
    """A weld bead along `points` (on a plate seam, in the part's frame), seen
    close up only. Returns its meshes. Roles: paint."""

    def seam(lod):
        return points

    made = tube_part(name=name, path_for=seam, radius=radius, mat=mats["paint"], parent=parent, lods=FINE)
    return place(made, loc, rot)


def side_skirt(name, outline, side, y, x0, x1, foot, mats, parent, top=None, panels=1, rows=1, depth=0.06,
               gap=0.02, inset=0.03, bolts=(2, 1), lods=ALL):
    """A run of armour plates in the side plane (skirts over the track, or
    side armour tiles), the outer face at `side * y` (`side` +1 left, -1
    right) and `depth` thick inward. The run spans [x0, x1] in `panels`
    columns and `rows` rows, each cell less `gap`, from `foot` up to `top`:
    each a height or (x, z) points of a line carried on along its end
    segments, the foot convex (rising toward a sprocket) and the top
    concave. Without `top` the run reaches the outline. Every panel is
    clipped to `outline` (`hull_shape.Outline`) moved in by `inset`: never
    above outline.top(x) - inset, never ahead of outline.front(z) - inset.
    A panel left lower than MIN_PANEL_M is dropped, so the run stops where
    the glacis takes over. Each has a grid of `bolts` (along x, up z) bolt
    heads close up. Names {name}_{k}, or {name}_{k}_{j} in row j of
    several. Returns its meshes. Roles: paint, steel."""
    feet, tops = _line(foot, x0, x1), None if top is None else _line(top, x0, x1)
    cuts = _cuts(feet, -1) + ([] if tops is None else _cuts(tops, 1))
    lo = min(z for _, z in feet)
    hi = max(z for _, z in tops) if tops is not None else max(z for _, z in outline.points) - inset
    w, h = (x1 - x0) / panels, (hi - lo) / rows
    made = []
    for k in range(panels):
        for j in range(rows):
            a, b = x0 + w * k + gap / 2, x0 + w * (k + 1) - gap / 2
            zlo, zhi = lo + h * j + gap / 2, lo + h * (j + 1) - gap / 2
            panel = [(a, zlo), (b, zlo), (b, zhi), (a, zhi)]
            for cut in cuts:
                panel = HS.clip_half_plane(panel, *cut)
            panel = outline.clip(panel, inset)
            if len(panel) < 3 or max(z for _, z in panel) - min(z for _, z in panel) < MIN_PANEL_M:
                continue
            # Start at the rear foot, as the cell does, so a panel the clip
            # leaves whole keeps the cell's own corners in order.
            first = panel.index(min(panel))
            panel = panel[first:] + panel[:first]
            tag = f"{name}_{k}" if rows == 1 else f"{name}_{k}_{j}"
            made += prism(tag, panel, depth, loc=(0, side * (y - depth / 2), 0), mat=mats["paint"], parent=parent,
                          bevel=PLATE_BEVEL, lods=lods)
            made += _panel_bolts(tag, panel, side, y, bolts, mats, parent)
    return made


def track_fender(name, outline, side, y_in, y_out, x0, x1, z, mats, parent, slope=0.0, depth=0.04):
    """The shelf over the track's top run, from `y_in` to `y_out` out from the
    centre line on the `side` (+1 left, -1 right) and [x0, x1] along the
    hull, `depth` thick: its top at height `z` at x0, rising `slope` (dz/dx)
    toward the front, its front end clipped to `outline`
    (`hull_shape.Outline`). Returns its meshes. Roles: paint."""
    z1 = z + slope * (x1 - x0)
    shelf = outline.clip([(x0, z - depth), (x1, z1 - depth), (x1, z1), (x0, z)], 0.0)
    return prism(name, shelf, y_out - y_in, loc=(0, side * (y_in + y_out) / 2, 0), mat=mats["paint"],
                 parent=parent, bevel=min(PLATE_BEVEL, depth * 0.45), lods=MID)


def _line(spec, x0, x1):
    """A panel run's foot or top as (x, z) points: a height is level from
    `x0` to `x1`."""
    if isinstance(spec, (int, float)):
        return [(x0, spec), (x1, spec)]
    return list(spec)


def _cuts(points, sense):
    """The half-planes (nx, nz, c) of a line through `points` that keep what
    lies above it (`sense` -1, a foot) or below it (+1, a top). Raises
    ValueError on a foot that isn't convex or a top that isn't concave: the
    half-planes would cut what the line leaves."""
    cuts, last = [], None
    for (xa, za), (xb, zb) in zip(points, points[1:]):
        if xb <= xa:
            raise ValueError(f"a panel line must run forward: {points}")
        slope = (zb - za) / (xb - xa)
        if last is not None and sense * (slope - last) > 1e-9:
            raise ValueError(f"a panel {'top' if sense > 0 else 'foot'} must bend {'down' if sense > 0 else 'up'}: "
                             f"{points}")
        cuts.append((-sense * slope, sense, sense * (za - slope * xa)))
        last = slope
    return cuts


def _panel_bolts(tag, panel, side, y, bolts, mats, parent):
    """Bolt heads in a grid of `bolts` (along x, up z) across a panel's own
    span, seen close up only."""
    nx, nz = bolts
    xs = [x for x, _ in panel]
    a, b = min(xs), max(xs)
    made = []
    for n in range(nx * nz):
        i, j = n % nx, n // nx
        bx = (a + b) / 2 + ((i / (nx - 1) if nx > 1 else 0.5) - 0.5) * (b - a) * 0.7
        lo, hi = HS.span(panel, bx, 0)
        bz = (lo + hi) / 2 + ((j / (nz - 1) if nz > 1 else 0.5) - 0.5) * (hi - lo) * 0.7
        made += cyl(f"{tag}_bolt_{n}", 0.018, 0.016, (bx, side * (y + 0.006), bz), "Y", mats["steel"], parent,
                    seg=6, lods=FINE)
    return made
