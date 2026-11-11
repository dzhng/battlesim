# 09: hull outline and side panels (coordinator)

## Contract

Side skirts and track fenders are drawn by one helper, whose panels never jut
past the hull's side outline (decision D2). They stop where the glacis takes
over, or follow its slope. This answers the "front side panels stick out" notes
on 11 vehicles. Today the cause is boxes each family draws for itself, squared
to the hull length, for example:
- `bradley.py:73`;
- `cv90.py:81`;
- `sp_howitzers.py:101-105`.

## Seam

```python
# hull_shape.py
class Outline:
    """The hull's side elevation. Built from the convex XZ profiles a family
    already extrudes (`Outline(*profiles_xz)`, as passed to parts.prism), or
    from loft rings (`Outline.from_rings(rings)`); raises ValueError on a
    non-convex profile."""
    def top(self, x) -> float | None
    def front(self, z) -> float | None
    def rear(self, z) -> float | None

# armour_parts.py
MIN_PANEL_M = 0.12
def side_skirt(name, outline, side, y, x0, x1, foot, mats, parent, top=None, panels=1,
               depth=0.06, gap=0.02, inset=0.03, bolts=(2, 1), lods=ALL) -> list
"""A run of plates in the side plane, outer face at side*y. Each panel is the
rectangle [x_k0, x_k1] x [foot(x), top(x)] clipped to the outline shrunk by
`inset`: never above outline.top(x) - inset, never ahead of
outline.front(z) - inset. A panel left lower than MIN_PANEL_M is dropped, so
the run stops where the glacis takes over. Names {name}_{k}. Roles: paint, steel."""
def track_fender(name, outline, side, y_in, y_out, x0, x1, z, mats, parent, slope=0.0) -> list
"""The shelf over the track's top run, its front end clipped to the outline."""
```

Families pass names such as `f"skirt_{s}"`. That keeps wreck selectors working,
for example `sp_howitzers.py:253-262` and `bradley.py:209`.

`side_skirt` bevels its panels at `PLATE_BEVEL`. Until slice 11 adds that
constant, use today's 0.01 m.

## Steps

1. **Probe in `throwaway/`:** build an `Outline` for every tracked hull, from
   its profiles or rings. This catches non-convex hulls early, such as Armata's
   stepped rings and the Abrams loft.
   - A hull that fails gets an outline built from its convex envelope.
   - Record which hulls those are.
2. Write `Outline`, `side_skirt` and `track_fender`.
3. **First consumer:** replace `cv90.follow_tiles` (`cv90.py:92-116`), the
   helper's prototype. The CV90120 also uses it.
   - Re-export `cv90` and `cv90120`.
   - The result is byte-identical if the generalisation is exact; otherwise
     name the difference.
4. **Proof on a user note:** draw the Bradley's front fender (`bradley.py:73`)
   with `track_fender`. Re-export `bradley`.
5. **Test:** add `hull_shape_test.py` and run it from `asset check`'s Blender
   runner. It checks that a panel run clipped to a sloped glacis outline never
   leaves it. It tests the mechanic, not data.

Other families migrate in their own slices, through the template checklist.
That includes the seven tank families that have their own
`skirt_top`/`skirt_foot`:
- `abrams.py:113`;
- `leopard.py:67`;
- `challenger_3.py:55-67`;
- `leclerc.py:86`;
- `type15.py:50`;
- `armata.py:101-139`;
- `t72.py:146`.

## The variable and crop

- **The one variable:** where the side panels end at the nose.
- **The crop:** a low three-quarter front view of the front corner, on the
  Bradley M2A4 and CV9040C.
- **Out of scope:** body shape and turrets.

## Verification

- the probe report;
- the test;
- byte identity, or a named difference, for the CV90;
- **closeups**, then **`compare-screenshots`** of the Bradley after against
  before;
- **last check: an unprimed `screenshot-critique`** of the crop.

## Delegated

The clip algorithm. Sutherland–Hodgman against the convex outline is
suggested.

## Result (2026-11-11)

Lane 09+11 (decision C16), branched from `e598a571c`. Evidence is in
`assets/after/09/`.

**The seam as built.** Three calls changed from the plan; each is recorded
in `decisions.md`.

```python
# hull_shape.py
class Outline:
    def __init__(self, *profiles, envelope=False)
    @classmethod
    def from_rings(cls, rings, envelope=False)
    def top(self, x) / front(self, z) / rear(self, z)  # None off the outline
    def clip(self, polygon, inset)                     # never above top - inset, ahead of front - inset,
                                                       # behind rear + inset; the belly doesn't cut
def clip_half_plane(polygon, nx, nz, c)               # Sutherland-Hodgman, one half-plane
def span(polygon, at, axis)

# armour_parts.py
MIN_PANEL_M = 0.12
def side_skirt(name, outline, side, y, x0, x1, foot, mats, parent, top=None, panels=1, rows=1,
               depth=0.06, gap=0.02, inset=0.03, bolts=(2, 1), lods=ALL)
def track_fender(name, outline, side, y_in, y_out, x0, x1, z, mats, parent, slope=0.0, depth=0.04)
```

- `side_skirt` gained `rows`, so it can carry the CV90's two-row tile grid.
  Its panels are then named `{name}_{k}_{j}`, so the CV90 wreck's column
  selectors (`side_armour_R_1_`) still work.
- `foot` and `top` are each a height or a line of (x, z) points. A foot
  must be convex (it may rise toward a sprocket) and a top concave.
  Otherwise the call raises `ValueError`.
- `track_fender` gained `depth`: z is the fender top at x0, and `slope`
  is dz/dx toward the front.

**Probe.** `outline_probe.py` built an `Outline` from the `hull*` prisms
and lofts of all 37 tracked appearances; the results are in
`outline-probe.json`. Each entry gives the notch depth: how far the
offending point sits inside its convex hull.

- **Five are non-convex**, and in each case `hull_upper` is the notched
  profile:
  - **BRM-3K:** 14 cm. The point is the roof step at x = -1.10, where the
    raised fighting compartment drops to the engine deck. A panel run
    reaching above the engine deck would need its own `top`.
  - **T-72B3 and T-90M:** 5.1 cm. **T-80BVM:** 3.4 cm. **Type 99A:**
    10.5 cm. In all four, the upper box's underside breaks upward at
    `lower_foot_x` toward the nose. The notch is on the underside, which
    no side panel reaches, and the lower hull fills it in the union. Those
    families pass `envelope=True` or include `hull_lower`.
- **Every other hull is convex.** That includes the Armata's lofted rings
  (T-14, T-15) and the Abrams family, which the risk table expected to
  fail.
- The T-72 family and the Type 99 also extrude `hull_upper_fender_L/R`
  strips. Their family slices decide whether those strips bound the
  panels.

**CV90 identity** (`glb-diff.txt`). Replacing `follow_tiles` with
`side_skirt`:

- **CV9040C and Mk IV:** byte-identical (`6bcb7d48…`, `fa50151d…`). They
  never called `follow_tiles`.
- **CV90120:** a named difference. 1,107 of its 1,139 nodes are identical,
  and the rest differ in three ways:
  - Tile `side_armour_{L,R}_4_1` straddles the glacis head. Its top now
    follows the outline exactly: level to x = 1.62, then down the glacis.
    The prototype cut a chord from the glacis head at x = 1.30 to the
    tile's front corner, up to 3 cm under the outline.
  - Tile `side_armour_{L,R}_4_0` loses a redundant collinear vertex at
    x = 1.30, which the prototype added to every tile spanning the glacis
    head. Its shape is the same.
  - Ten bolts per side, on tiles 4_1, 5_0 and 5_1, move 0.4–8.4 cm. Bolts
    now sit across each tile's own clipped span. Before, they sat on the
    full cell and were clamped 6 cm under the top.
  - Triangles at tiers 0–3 go from 53,031 / 17,243 / 5,329 / 1,792 to
    53,055 / 17,243 / 5,321 / 1,784.

**Fenders.** The CV90 and Bradley front fenders are now `track_fender`
shelves with the old box's top line. They are bounded by the envelope of
the upper and lower hull.

- **Bradley:** the front end comes back up to 4 cm, to the hull's side
  outline. The rest of each Bradley is identical, node for node.
- **CV9040C:** loses its front-bottom corner, so the shelf has five sides
  at tiers 0–2.
- **CV90120:** its fender is not clipped.

**Test.** `hull_shape_test.py` runs in the pinned Blender, and `asset
check` finds it on its own. It checks four things:

- a notched profile or stepped rings raise;
- a skirt run carried past a sloped glacis never leaves the outline less
  its inset, drops its last panel, and cuts its front panels down;
- a two-row grid with a rising foot stays inside;
- a fender ends at the nose.

On its first run it caught a real defect. The clip had skipped every
downward-facing edge, so a fender passed under a shallow lower nose. Now
only the belly is skipped.

**Closeups and comparison** (`compare-metrics.txt`, `bradley-fender-pair.png`).
The front-corner shot is `corner:6.8m,4.6m,1.2m:3.0m,1.45m,1.0m:50`. Before
and after differ only at the fenders, and on the CV90120 at its glacis
tile. The parity distance is under 0.004 on every pair. Verdict: the
candidate is less wrong, with every fender inside the side outline, but the
change is small on screen. What visibly "sticks out" on the Bradley and the
CV9040C is their side armour standing above the glacis: the applique tiles
on the Bradley, and `armour_tiles` on the CV9040C and Mk IV. Those are
family geometry, left to slices 25 and 26, which now only need to call
`side_skirt`.

**Critique** (unprimed, on the final geometry):

| Vehicle | Silhouette | Surface | Fittings | Materials | Cohesion |
|---|---|---|---|---|---|
| M2A4 Bradley | 5 | 6 | 6 | 5 | 5 |
| CV9040C | 5 | 5 | 4 | 5 | 5 |
| CV90120 | 6 | 5 | 4 | 5 | 6 |

The critic's first finding was the Bradley's and CV9040C's side armour
standing above the glacis as a square wall, a fin on the far side. It noted
that the CV90120, on `side_skirt`, has no such fin. Its other findings are
family work, carried to slices 25 and 26:

- fender shape: no lip or downturn on the Bradley, a narrow strip on the
  CV90;
- the CV90 skirt-lip rail poking past the skirt;
- placeholder headlights;
- tracks drawn as faceted drums;
- smeared camo on the CV90's slopes.

**Lint.** `vehicle_lint.py --variant` on the CV90120, CV9040C, Mk IV, M2A4
and M3A3 reports 0 new issues each.
