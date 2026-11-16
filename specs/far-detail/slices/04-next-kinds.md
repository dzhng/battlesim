# 04: Courtyard walls, then what the census ranks next

## What this unlocks

The same far mesh for the next kinds [slice 00](00-census-and-ceiling.md)
ranks highest. Content only. Each kind earns its place alone: one kind, one
commit, one verdict.

## Why walls get their own judgement

A coarser wall was tried before, as a replacement for tier 3 (88 triangles to
40), and rejected on sight: it lost contrast and stopped reading as an upright
wall ([performance research](../../done/performance/research.md), "Distant-wall
simplification rejected"). This slice differs in one way: the far mesh draws
only below `far_px`, a much smaller size than tier 3 covers. That is a
hypothesis the crops must prove, not an exemption.

## Seam

- Walls: `packages/scene-assets/blender/courts.py`, `courtyard_wall`. A wall
  is a module repeated along its run; each module carries its own far mesh.
  Merging a run into one piece belongs to [slice 05](05-spike-block-merge.md).
- Nothing else, on today's [census](../assets/census.md): after walls the next
  kind is garden fences at 3.7% of prop triangles, and planters (2.3%) already
  skip nearly all their shadows. Add a kind only if a later census moves it.
- No per-kind threshold and no per-kind code. If a kind fails at the shared
  `far_px`, it does not carry a far mesh.

**The wall's far mesh keeps** its height, thickness and run, and every colour
band tier 3 shows from outside (plinth, body, coping), since the bands carry
the contrast. **It drops** pier relief, caps and faces hidden by the next
module.

## Order of work, per kind

1. Pictures first, before any timing: the kind's review sheet, an oblique
   courtyard or street view, the wide hold and the threshold stops.
2. If the pictures pass, measure against the build that shipped the previous
   kind. Report this kind's own saving and the running total separately.
3. Native counts and one capture.

## What the user can see

For each kind: tier 3 beside the far mesh on its sheet, and paired crops of a
complete wall run with its top edge, an upright corner and its ground shadow
(walls), or rim, body and ground contact (planters).

## Checks

**Slice variable:** the kind's contrast and upright silhouette at a few
pixels. **Crop:** as above. **Out of scope:** everything else in frame.

1. [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
   against the current game, looking specifically for the earlier failure:
   less contrast, a less distinct upright.
2. A fresh, unprimed
   [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
   last.
3. Counts, timing, the off-view caster and the digest, exactly as
   [slice 03](03-parked-car.md).
4. [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md), about
   five minutes, then decide, record and close.

A kind that fails its pictures, or whose saving is inside the noise, is
dropped in the same commit that tried it: its `_LOD4` meshes and budget entry
go, and the [ledger](../decisions.md) records which kinds shipped and which
did not. Dropping a kind does not undo the cars.

## Delegated

- The order of kinds after walls.
- Each far mesh's shape.
- At most two attempts per kind.

## Must stay green

The car's shipped far mesh and its numbers; module fitting and the regions'
looks; the README's global checks.

## Feedback that would change this slice

- "Walls still read flat": drop walls here and leave them to slice 05, where
  a merged run can keep one crisp top edge.
- "Do every prop": the mechanism needs no more code; it is more art, ranked by
  the census.
