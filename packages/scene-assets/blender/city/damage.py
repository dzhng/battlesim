"""What is left of a graph-made building: the geometry of a ruin and of a burnt shell.

The graphs have no damage inputs, so a damage state is made from the intact
export: its walls cut down to ragged stumps, a heap of rubble over its plan,
its floors fallen in slabs, its fittings charred, hung or thrown down. These
are the cuts and the piles; which building gets which is its exporter's.

Everything is numpy on `graph.Soup` with a seeded `random.Random`, so a state
is the same bytes every run. Positions are metres, Z up; nothing here goes
below z = 0.
"""
import math

import numpy as np

from graph import Soup


def noise(x, y, scale, seed):
    """Smooth noise in [0, 1] over the plane, about `scale` metres across, that never repeats."""
    u, v = np.asarray(x, dtype=np.float64) / scale + 0.37 * seed, np.asarray(y, dtype=np.float64) / scale - 0.61 * seed
    i, j = np.floor(u), np.floor(v)
    fu, fv = u - i, v - j
    fu, fv = fu * fu * (3 - 2 * fu), fv * fv * (3 - 2 * fv)

    def corner(a, b):
        n = (a.astype(np.int64) * 374761393 + b.astype(np.int64) * 668265263 + seed * 1442695041) & 0xFFFFFFFF
        n = ((n ^ (n >> 13)) * 1274126177) & 0xFFFFFFFF
        return ((n ^ (n >> 16)) & 0xFFFF) / 65535.0

    low = corner(i, j) + (corner(i + 1, j) - corner(i, j)) * fu
    high = corner(i, j + 1) + (corner(i + 1, j + 1) - corner(i, j + 1)) * fu
    return low + (high - low) * fv


def clip(soup, normal, offset):
    """The part of `soup` where `p . normal <= offset`: triangles the plane crosses are cut along it."""
    if not len(soup):
        return soup
    n = np.asarray(normal, dtype=np.float64)
    d = soup.v @ n - offset
    out = d[soup.t] > 1e-9  # (m, 3): this corner is cut away
    count = out.sum(1)
    keep = soup.keep(count == 0)
    pieces = [keep]
    for lone_out in (True, False):
        # one corner on its own side: turn each triangle so that corner comes first
        pick = count == (1 if lone_out else 2)
        if not pick.any():
            continue
        t = soup.t[pick]
        first = np.argmax(out[pick] if lone_out else ~out[pick], axis=1)
        t = np.stack([t[np.arange(len(t)), (first + k) % 3] for k in range(3)], 1)
        a, b, c = soup.v[t[:, 0]], soup.v[t[:, 1]], soup.v[t[:, 2]]
        da, db, dc = d[t[:, 0]], d[t[:, 1]], d[t[:, 2]]
        tb, tc = (da / (da - db))[:, None], (da / (da - dc))[:, None]
        ab, ac = a + (b - a) * tb, a + (c - a) * tc
        cab = soup.c[t[:, 0]] + (soup.c[t[:, 1]] - soup.c[t[:, 0]]) * tb
        cac = soup.c[t[:, 0]] + (soup.c[t[:, 2]] - soup.c[t[:, 0]]) * tc
        m, s = soup.m[pick], soup.s[pick]
        k = len(t)
        if lone_out:  # the lone corner goes: a quad (ab, b, c, ac) stays
            v = np.concatenate([ab, b, c, ac])
            col = np.concatenate([cab, soup.c[t[:, 1]], soup.c[t[:, 2]], cac])
            i = np.arange(k)
            tris = np.concatenate([np.stack([i, i + k, i + 2 * k], 1), np.stack([i, i + 2 * k, i + 3 * k], 1)])
            pieces.append(Soup(v, col, tris, np.concatenate([m, m]), np.concatenate([s, s]), soup.mats))
        else:  # the lone corner stays: a triangle (a, ab, ac)
            v = np.concatenate([a, ab, ac])
            col = np.concatenate([soup.c[t[:, 0]], cab, cac])
            i = np.arange(k)
            pieces.append(Soup(v, col, np.stack([i, i + k, i + 2 * k], 1), m, s, soup.mats))
    return Soup.join(pieces)


def quad(corners, material, colour=1.0):
    return Soup(np.array(corners, dtype=np.float64), np.broadcast_to(np.asarray(colour, dtype=np.float64), (4, 3)),
                [(0, 1, 2), (0, 2, 3)], [0, 0], [False, False], [material])


def box(lo, hi, material, colour=1.0, matrix=None, top=None):
    """A box over the bounds; `top` is another material for its upper face. `matrix` (4x4) moves it."""
    x0, y0, z0 = lo
    x1, y1, z1 = hi
    faces = [
        ([(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)], material),
        ([(x1, y0, z0), (x1, y1, z0), (x1, y1, z1), (x1, y0, z1)], material),
        ([(x1, y1, z0), (x0, y1, z0), (x0, y1, z1), (x1, y1, z1)], material),
        ([(x0, y1, z0), (x0, y0, z0), (x0, y0, z1), (x0, y1, z1)], material),
        ([(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], top or material),
        ([(x0, y1, z0), (x1, y1, z0), (x1, y0, z0), (x0, y0, z0)], material),
    ]
    out = Soup.join([quad(corners, m, colour) for corners, m in faces])
    return out.transformed(matrix) if matrix is not None else out


def tilted(yaw, pitch, roll, x, y, z):
    """A 4x4 that rolls about X, pitches about Y, turns about Z, then moves."""
    cy, sy, cp, sp, cr, sr = math.cos(yaw), math.sin(yaw), math.cos(pitch), math.sin(pitch), math.cos(roll), math.sin(roll)
    rz = np.array([[cy, -sy, 0], [sy, cy, 0], [0, 0, 1.0]])
    ry = np.array([[cp, 0, sp], [0, 1.0, 0], [-sp, 0, cp]])
    rx = np.array([[1.0, 0, 0], [0, cr, -sr], [0, sr, cr]])
    m = np.eye(4)
    m[:3, :3] = rz @ ry @ rx
    m[:3, 3] = (x, y, z)
    return m


def under(soup, top_m):
    """`soup` with nothing above `top_m` or below the ground: what sticks out is pressed flat."""
    v = soup.v.copy()
    v[:, 2] = np.clip(v[:, 2], 0.0, top_m)
    return Soup(v, soup.c, soup.t, soup.m, soup.s, soup.mats)


# ---------------------------------------------------------------- stumps
def profile(rng, length, step, low, high, gaps=0.12):
    """The broken top of a wall `length` long, one height every `step`: a walk between
    `low` and `high` pulled toward the top, with sudden breaks down to `low`."""
    tops, top = [], rng.uniform(low + 0.6 * (high - low), high)
    for _ in range(max(1, math.ceil(length / step - 1e-9))):
        top += rng.gauss(0.0, 0.22 * (high - low)) + 0.3 * (low + 0.75 * (high - low) - top)
        if rng.random() < gaps:
            top = low + rng.uniform(0.0, 0.25) * (high - low)
        top = min(high, max(low, top))
        tops.append(top)
    return tops


def break_off(soup, start, along, step, tops):
    """`soup` cut into columns `step` wide along `along` from `start`, each cut off at its
    height in `tops`."""
    ax, ay = along
    out = []
    highest = clip(soup, (0, 0, 1), max(tops))
    for k, top in enumerate(tops):
        s0 = start[0] * ax + start[1] * ay + k * step
        column = clip(clip(highest, (-ax, -ay, 0), -s0), (ax, ay, 0), s0 + step)
        out.append(clip(column, (0, 0, 1), top))
    return Soup.join(out)


def broken_edge(start, along, inward, depth_m, length, step, tops, material, colour=1.0, sides=True):
    """The broken top of a wall `length` long and `depth_m` thick: a cap on each column at
    its height, and the face that shows where one column stands above the next."""
    ax, ay = along
    ix, iy = inward[0] * depth_m, inward[1] * depth_m
    at = lambda s, z, deep: (start[0] + ax * s + (ix if deep else 0.0), start[1] + ay * s + (iy if deep else 0.0), z)
    out = []
    for k, top in enumerate(tops):
        s0, s1 = k * step, min((k + 1) * step, length)
        out.append(quad([at(s0, top, 0), at(s1, top, 0), at(s1, top, 1), at(s0, top, 1)], material, colour))
        nxt = tops[k + 1] if k + 1 < len(tops) else None
        if sides and nxt is not None and abs(nxt - top) > 1e-6:
            lo, hi = min(top, nxt), max(top, nxt)
            face = [at(s1, lo, 0), at(s1, lo, 1), at(s1, hi, 1), at(s1, hi, 0)]
            out.append(quad(face if nxt < top else face[::-1], material, colour))
    return Soup.join(out)


# ---------------------------------------------------------------- rubble
def distance_out(x, y, rects):
    """How far a point is outside a union of rectangles (negative inside), roughly."""
    best = np.full(np.shape(x), np.inf)
    for cx, cy, hx, hy in rects:
        dx, dy = np.abs(x - cx) - hx, np.abs(y - cy) - hy
        best = np.minimum(best, np.hypot(np.maximum(dx, 0), np.maximum(dy, 0)) + np.minimum(np.maximum(dx, dy), 0))
    return best


def pile(x, y, rect, high, spill_m, seed):
    """One part's rubble: piled to `high`, lumpy, running out past its walls in tongues of
    up to `spill_m` and falling to nothing there."""
    d = distance_out(x, y, [rect])
    lump = 0.5 + 0.5 * noise(x, y, 5.0, seed) * (0.6 + 0.4 * noise(x, y, 1.7, seed + 3)) + 0.16 * (noise(x, y, 0.8, seed + 7) - 0.5)
    inside = high * lump * (0.7 + 0.3 * np.clip(-d / 2.5, 0.0, 1.0))
    edge = np.clip(1.0 - d / tongue(x, y, spill_m, seed), 0.0, 1.0) ** 1.5 * (0.4 + 0.6 * noise(x, y, 1.1, seed + 6))
    return np.where(d <= 0, inside, high * 0.5 * edge)


def tongue(x, y, spill_m, seed):
    """How far the rubble runs out past the wall at a point: a quarter of `spill_m` to all of it."""
    return spill_m * (0.25 + 0.75 * noise(x, y, 2.6, seed + 5))


def heap_height(x, y, rects, heights, spill_m, seed):
    """The rubble's height over the plan: the highest of the parts' piles."""
    out = np.zeros(np.shape(x))
    for rect, high in zip(rects, heights):
        out = np.maximum(out, pile(x, y, rect, high, spill_m, seed))
    return out


def heap(rects, heights, spill_m, cell_m, seed, materials, shade):
    """The rubble as a surface: each part's pile over a grid of about `cell_m` that ends
    where the pile does, `spill_m` outside its walls. `materials` is (names, a function
    of x and y giving each face's index into them), `shade(x, y, z)` each vertex's colour."""
    names, material_of = materials
    out = []
    for rect, high in zip(rects, heights):
        cx, cy, hx, hy = rect
        nx, ny = max(1, math.ceil(2 * (hx + spill_m) / cell_m)), max(1, math.ceil(2 * (hy + spill_m) / cell_m))
        x, y = np.meshgrid(np.linspace(cx - hx - spill_m, cx + hx + spill_m, nx + 1),
                           np.linspace(cy - hy - spill_m, cy + hy + spill_m, ny + 1))
        x, y = x.ravel(), y.ravel()
        # off the grid's lines, or the heap is a quilt; only inside the walls, where no cell can turn over
        within = (np.abs(x - cx) < hx) & (np.abs(y - cy) < hy)
        jx = x + 0.6 * cell_m * (noise(x, y, 0.37 * cell_m, seed + 21) - 0.5)
        jy = y + 0.6 * cell_m * (noise(x, y, 0.37 * cell_m, seed + 22) - 0.5)
        x = np.where(within, np.clip(jx, cx - hx + 0.02, cx + hx - 0.02), x)
        y = np.where(within, np.clip(jy, cy - hy + 0.02, cy + hy - 0.02), y)
        # the grid's rim is drawn in to where the rubble ends, so its edge is ragged, not the grid's
        px, py = np.clip(x, cx - hx, cx + hx), np.clip(y, cy - hy, cy + hy)
        d = np.hypot(x - px, y - py)
        pull = np.minimum(1.0, tongue(px, py, spill_m, seed) / np.maximum(d, 1e-9))
        x, y = px + (x - px) * pull, py + (y - py) * pull
        z = pile(x, y, rect, high, spill_m, seed)
        a = (np.arange(ny)[:, None] * (nx + 1) + np.arange(nx)[None, :]).ravel()
        tris = np.concatenate([np.stack([a, a + 1, a + nx + 2], 1), np.stack([a, a + nx + 2, a + nx + 1], 1)])
        # a cell is one material, both its triangles: one to a triangle is a chequer of diagonals
        centre = np.tile((np.stack([x, y], 1)[a] + np.stack([x, y], 1)[a + nx + 2]) / 2, (2, 1))
        kept = z[tris].max(1) > 0.02
        tris, centre = tris[kept], centre[kept]
        out.append(Soup(np.stack([x, y, z], 1), shade(x, y, z), tris, material_of(centre[:, 0], centre[:, 1]),
                        np.ones(len(tris), bool), names).keep(np.ones(len(tris), bool)))
    return Soup.join(out)


def scatter(rng, rects, count, size, height_at, top_m, materials, colour):
    """Broken blocks thrown over the heap: `count` boxes of about `size` metres, each
    tipped and half sunk at the heap's height there."""
    out = []
    for _ in range(count):
        cx, cy, hx, hy = rects[rng.randrange(len(rects))]
        x, y = cx + rng.uniform(-hx, hx), cy + rng.uniform(-hy, hy)
        big = rng.uniform(0.4, 1.0) ** 2 * 2.2  # many small, a few large
        w, d, h = (min(1.6, size * big * rng.uniform(0.45, 1.3)) for _ in range(3))
        z = float(height_at(np.array([x]), np.array([y]))[0])
        m = tilted(rng.uniform(0, math.tau), rng.uniform(-0.6, 0.6), rng.uniform(-0.6, 0.6), x, y, z + 0.2 * h)
        shade = rng.uniform(*colour)
        out.append(under(box((-w / 2, -d / 2, -h / 2), (w / 2, d / 2, h / 2), rng.choice(materials), shade, m), top_m))
    return Soup.join(out)


def slabs(rng, rects, heights, height_at, top_m, material, top_material, colour):
    """Floors fallen flat: one or two big slabs to a part, tilted, lying on its heap."""
    out = []
    for (cx, cy, hx, hy), high in zip(rects, heights):
        long_x = hx >= hy
        run, across = (hx, hy) if long_x else (hy, hx)
        for k in range(max(1, round(run / 7.0))):
            length, width = min(2 * run, rng.uniform(6.0, 9.0)), 2 * across * rng.uniform(0.55, 0.8)
            s = -run + (k + rng.uniform(0.3, 0.7)) * 2 * run / max(1, round(run / 7.0))
            x, y = (cx + s, cy + rng.uniform(-0.15, 0.15) * across) if long_x else (cx + rng.uniform(-0.15, 0.15) * across, cy + s)
            z = float(height_at(np.array([x]), np.array([y]))[0])
            m = tilted(0.0 if long_x else math.pi / 2, rng.uniform(-0.22, 0.22), rng.uniform(-0.3, 0.3), x, y, z + 0.15)
            lo, hi = (-length / 2, -width / 2, -0.12), (length / 2, width / 2, 0.12)
            out.append(under(box(lo, hi, material, rng.uniform(*colour), m, top=top_material if rng.random() < 0.3 else None), top_m))
    return Soup.join(out)


# ---------------------------------------------------------------- fire
def charred(soup, gone, seed, light=(0.03, 0.11)):
    """A fitting after the fire: the materials in `gone` burnt away (glass, cloth), the rest sooted."""
    soup = soup.without(gone)
    if not len(soup):
        return soup
    soot = light[0] + (light[1] - light[0]) * noise(soup.v[:, 0] + soup.v[:, 2], soup.v[:, 1] - soup.v[:, 2], 0.6, seed)
    return Soup(soup.v, soup.c * soot[:, None], soup.t, soup.m, soup.s, soup.mats)


def soot_fan(corners_bottom, height_m, spread_m, material, dark, clear=1.0, sides=True, top=None):
    """Soot above an opening, on the wall: from the opening's head (two points, left then
    right seen from outside) up `height_m`, widening by `spread_m`; `dark` at the head and
    `clear`, the wall's own colour, at the top and out to either side, so it has no
    edge. `top` is its colour at the top where it stops short, under another opening.
    Without `sides` it is the one quad over the head, for the farthest tier."""
    (ax, ay, az), (bx, by, bz) = corners_bottom
    length = math.hypot(bx - ax, by - ay)
    ux, uy = (bx - ax) / length, (by - ay) / length
    v = [(ax, ay, az), (bx, by, bz), (bx + ux * spread_m, by + uy * spread_m, bz + height_m),
         (ax - ux * spread_m, ay - uy * spread_m, az + height_m)]
    t = [(0, 1, 2), (0, 2, 3)]
    if sides:
        v += [(ax - ux * spread_m, ay - uy * spread_m, az), (bx + ux * spread_m, by + uy * spread_m, bz)]
        t += [(4, 0, 3), (1, 5, 2)]
    c = np.empty((len(v), 3))
    c[:] = clear
    c[:2] = dark
    if top is not None:
        c[2:4] = top
    return Soup(np.array(v), c, t, [0] * len(t), [False] * len(t), [material])
