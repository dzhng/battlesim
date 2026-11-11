"""Armour added to a hull or turret: fields of armour tiles, slat armour,
bolted applique panels and weld beads.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, cyl  # noqa: E402
from vehicle_parts import ALL, FINE, MID, NEAR, place, tube_part  # noqa: E402


def armour_tiles(name, loc, size, grid, depth, mats, parent, gap=0.025, bolts=True, rot=(0, 0, 0), lods=ALL):
    """A field of bolted armour tiles (explosive reactive armour, applique
    modules) lying in the local XY plane on a face at `loc`, standing `depth`
    proud along local +Z: `size` (x, y) of the whole field, `grid` (columns
    along x, rows along y). Each tile is chunky and bevelled, with a bolt head
    at each corner close up. Returns its meshes. Roles: paint, steel."""
    cols, rows = grid
    tx, ty = size[0] / cols, size[1] / rows
    made = []
    for i in range(cols):
        for j in range(rows):
            cx = -size[0] / 2 + tx * (i + 0.5)
            cy = -size[1] / 2 + ty * (j + 0.5)
            made += box(f"{name}_{i}_{j}", (tx - gap, ty - gap, depth), (cx, cy, depth / 2), mats["paint"], parent,
                        bevel=min(0.02, depth * 0.3), lods=lods)
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


def bolted_panel(name, loc, size, mats, parent, bolts=(3, 2), rot=(0, 0, 0), bevel=0.02, lods=MID):
    """An applique armour plate bolted onto a face: a slab `size` (x, y, z
    thickness) lying in its local XY plane on the face at `loc` (its back on
    the face, facing local +Z), with chunky bolt heads in a ring `bolts`
    (along x, along y) round its border. Turn it onto a side or a glacis with
    `rot`. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_plate", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=bevel, lods=lods)
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
