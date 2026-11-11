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
