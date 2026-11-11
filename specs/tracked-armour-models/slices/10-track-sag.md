# 10: track sag (coordinator)

## Contract

Every tracked belt sags between the supports of its top run (user decision T4).
No call site changes. The belt never enters a wheel.

The renderer's track node contract is unchanged: `track_L/R`,
`track_length_m` and `link_pitch_m` (`vehicle_parts.py:22-24`). The belt still
scrolls its UVs by its length.

## Seam

```python
# running_gear.py
TRACK_SAG = 0.03   # metres of droop per metre of unsupported top run (gentle)
def track_path(wheels, thickness, sag=TRACK_SAG) -> list[tuple[float, float]]
"""Pure. The belt's closed counter-clockwise centre line: the convex hull of
the grown wheel circles (as today), with every top-run tangent segment between
two different wheels (outward normal z > 0.5) replaced by a parabola sampled at
8 points. Mid-span drop = min(sag * L, clearance - thickness / 2 - 0.01), where
clearance is the gap down to the highest wheel circle under the span."""
def track_loop(name, wheels, y, width, pitch, mats, parent, thickness=0.08, sag=TRACK_SAG)
def tracked_running_gear(..., sag=TRACK_SAG)   # passes through
```

- `track_length_m` becomes the length of the sagged path.
- No family overrides `sag` in this pass.
- The clearance clamp is built into `track_path`, because the lint can't catch
  a sag cutting into a wheel: tracks and wheels sit under the same top node.

## Also in this slice

The lint's ground-contact check finds tracks by name (`"track_" in name`), a
second definition of what a track is (slice 01). Make it use the running-gear
node contract this module owns.

## The variable and crop

- **The one variable:** the shape of the top run.
- **The crop:** a low side view of the running gear only. Shoot:
  - the T-72B3 (low return rollers);
  - the Leopard 2A6 (four return rollers);
  - the ZTQ-15 (no return rollers visible).
- **Out of scope:** wheel and skirt shapes.

## Steps

1. Write `track_path` and its clamp.
2. Re-export the three models.
3. **Closeups**, then **`compare-screenshots`** against before.
4. **Last check: an unprimed `screenshot-critique`** of the crop.
5. **Checkpoint,** which never blocks.

## Must stay green

- the GLB node contract;
- lint;
- the rendered belt animation, checked with `asset sheet` on one model under
  the GPU lock.

## Delegated

- samples per span;
- parabola or catenary.

## Feedback that would change it

"More Soviet sag." Slice 17 then passes `sag=0.06` for the T-72 family, and
that is recorded.
