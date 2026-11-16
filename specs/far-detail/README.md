# Far detail

**Status:** slice 01 is on the feature branch; 00 and 02 are in progress.
**Updated:** 2026-11-15.

In the wide view the game draws about 100,000 small props, most of them only a
pixel or two across, at the same mesh it uses when they are twenty pixels tall.
This plan gives small, high-count props a coarser **far mesh** for when they
cover only a few pixels, then runs two experiments on what is left: merging a
block's props into one mesh, and coarser terrain far from the camera.

The idea comes from Unreal's Nanite and its three.js port,
[three-nanite](https://github.com/apekshik/three-nanite/tree/b730aac13ab0ea67b75214613eea56c14151f6e5):
pick how much geometry to draw from how big its error is on screen. Only that
selection idea transfers. Nanite's answer to "many triangles inside one pixel"
is a software rasteriser the port does not have and WebGPU cannot host, and our
meshes are already smaller than one of its 128-triangle clusters. So the
production work is an ordinary coarser mesh, and the cluster hierarchy is
tested only where it could fit: across instances and on terrain.

## Next Agent Prompt

You are implementing `specs/far-detail/` on branch `t3/38678a68`. Slice 01 is
merged there. Slices 00 and 02 are being built in their own checkouts. Nothing
is on `staging` yet, and nothing goes there before slice 00 says go.

1. Read this README and [the decisions](decisions.md), then
   [`renderer`](../../.agents/skills/renderer/SKILL.md) and
   [`write-tests`](../../.agents/skills/write-tests/SKILL.md). The measured
   background is in [the performance research](../done/performance/research.md)
   ("Dominant prop appearances", "Direct culling audit", "Distant-wall
   simplification rejected") and [distant detail](../done/distant-detail/README.md).
2. Start [slice 00](slices/00-census-and-ceiling.md). It changes no production
   code and decides whether the rest is worth building.
3. Slices [01](slices/01-far-mesh-in-bundles.md) and
   [02](slices/02-far-selection.md) may be written while 00 measures, in their
   own checkouts. Nothing from them reaches `staging` before 00 says go.

Warnings:

- Every GPU run, in any checkout, goes through the one lock:
  `lockf -k /Users/server/dev/battlesim/throwaway/gpu.lock <command>`.
- Check `df` before a build. Each checkout's build is 8 to 22 GB.
- Fetch only the LFS files a slice names. Run only the tests a change reaches;
  the whole repository runs once, in [slice 07](slices/07-closeout.md).
- Scratch code, logs and candidate renders stay in `throwaway/far-detail/`.
  Only receipts, accepted crops and verdicts enter `assets/`.

Before you end a pass, update this section: status, date, the next pickup
point, and any decision that moved into [the ledger](decisions.md).

### To do

- [ ] 00: census of the wide view, and the ceiling probe's go or no-go → [slice 00](slices/00-census-and-ceiling.md)
- [x] 01: a static prop's bundle may carry a far mesh → [slice 01](slices/01-far-mesh-in-bundles.md)
- [ ] 02: presentation picks the far mesh, the renderer draws and counts it → [slice 02](slices/02-far-selection.md)
- [ ] 03: parked cars carry one; the measured win → [slice 03](slices/03-parked-car.md)
- [ ] 04: courtyard walls, then whatever the census ranks next → [slice 04](slices/04-next-kinds.md)
- [ ] 05: spike, merging a block's props into one mesh → [slice 05](slices/05-spike-block-merge.md)
- [ ] 06: spike, cluster detail for terrain → [slice 06](slices/06-spike-terrain.md)
- [ ] 07: closeout → [slice 07](slices/07-closeout.md)

## Contracts

These are the interfaces the work changes. Each says what it is today and what
it becomes.

1. **Bundle tiers (asset bake ↔ renderer).** Today every bundle state carries
   exactly four meshes, finest first. After: a static bundle drawn as a prop
   carries four, or five when its source names `_LOD4` meshes. The fifth is the
   **far mesh**. It has at least one triangle and strictly fewer than tier 3,
   and every state of an appearance carries it or none does. Units (skinned
   and articulated bundles), building kits, trees, dressing and grass stay at
   exactly four and refuse a fifth. The container format does not change: a bundle
   already records how many meshes each state has, so every bundle without a
   far mesh stays byte-identical.
2. **Tier choice (`crates/presentation/src/view.rs`).** Today a drawn model
   gets tier 0 to 3, or the card (`IMPOSTOR = 4`) when it has one and is under
   `impostor_px`. After: the far mesh is tier 4 (`FAR`), the card moves to 5,
   and the choice takes one value saying what the appearance carries past its
   four meshes: nothing, a far mesh, or a card. A model with a far mesh draws
   it below `far_px`. A model without one draws tier 3 at any small size, as
   today. Nothing asks for a mesh its bundle lacks.
3. **Shadow tier.** Unchanged outside the far band: a model casts one tier
   coarser than it draws, never past tier 3. A model drawn with its far mesh
   casts with its far mesh. A prop outside the view still casts with tier 3.
   Which props cast, and into which cascades, does not change at all.
4. **Threshold (`fixtures/presentation.json`).** `presentation.models` gains a
   required `far_px`, in the same projected-height pixels as `lod_px`. Loading
   refuses it unless `0 < far_px < lod_px[2]`, and now also refuses `lod_px`
   that does not fall and stay positive. There is no off value and no setting.
5. **Drawn-instance counts (renderer ↔ browser capture ↔ native).** Today the
   props' per-appearance counts are four numbers, one per tier. After: five.
   The buildings' and scenery's counts stay four. Readers refuse the old
   shape, and the committed count run under `fixtures/parity/capture/counts`
   is recorded again once.

Unchanged: simulation digests, replays, what each side is told, the batch and
instance records, cascade routing, the units' cards, building tiers, and the
terrain.

## One owner per concept

| Concept | Owner |
| --- | --- |
| Which tier a model draws and casts with | `view.rs`, for every model |
| Whether an appearance has a far mesh | its bundle's bytes; presentation reads the count it already decodes |
| When the far mesh draws | `presentation.models.far_px` |
| What a valid far mesh is | the bake's validator in `packages/scene-assets` |
| Which props cast, and where | the prop population's caster lists, as today |

No list of prop names decides anything at run time. No renderer code picks a
tier. No second copy of the tier rule exists for props. The finished code
should read as "four tiers, and a fifth some static props carry", not as a far
path beside the tiers.

## Firewalls

- Presentation and renderer only. The simulation, the map generator and what a
  side is told are not touched.
- A visible object never vanishes, and a prop outside the view never stops
  casting a shadow onto ground the player can see.
- Tiers 0 to 3 of every kind, `lod_px`, `impostor_px` and `shadow_caster_px`
  keep their values.
- No 2D cards for props, no per-cascade prop lists, no filtering of casters
  against the cascade union, no single-joint vertex path. Each was measured
  and rejected ([performance](../done/performance/README.md)).
- No compute rasteriser, visibility buffer or mesh shader.
- No switch, no off-by-default path, no backward compatibility, no migration.
- Spike code lives only in `throwaway/far-detail/` and is deleted once its
  verdict is written.

## Slices

```
00 census + ceiling probe ──go──► 03 parked car ──► 04 next kinds ──► 05 spike: block merge ──┐
        │                          ▲                                                        ├─► 07 closeout
        │      01 bundle far mesh ─┤                                                        │
        │      02 far selection ───┘                                                        │
        └─────────────────────────────────────────► 06 spike: terrain ──────────────────────┘
```

- 01 and 02 touch different packages and can run side by side. 02's last
  gate, a bundle baked by 01 drawn by 02, needs both.
- 06 needs only 00's session and can run beside the production slices.
- 05 measures against the shipped far mesh, so it follows 04.
- A no-go from 00 stops 01 to 04 where they stand and returns here to reslice.
  It does not end the requirement to make far props cheaper.

## Measuring

Every timing claim in this plan uses this protocol.

- **Scene.** The [frozen receipt](assets/baseline.json): Metro XL, seed 4, 100
  units a side, Low graphics, tick 90 paused, digest `60ede87ba60aab0b`, held at
  65 m, 260 m and 2,400 m. If today's build no longer reproduces the digest,
  find the input that changed and record a fresh paired control. Never compare
  unmatched workloads.
- **Arms.** Two production builds held apart, interleaved A B A B A B under the
  GPU lock, plus one A against A pair first to learn the session's noise. The
  harness is the paused view-cost hold in `web/scenes/endurance.mjs` and
  `web/scenes/_viewCost.mjs`.
- **Counts before time.** Assert equal instance counts in both arms and the
  expected triangle difference before reading any timing. An earlier car
  comparison was void because one arm had silently lost its cars.
- **Report** per view: GPU median per pair, CPU per frame, submitted triangles
  and instances by pass, GPU buffer and texture bytes.
- **Tripwires**, chosen before the work and loose on purpose: the candidate
  wins at least two of three wide pairs by more than the A against A spread;
  no view's median GPU time rises more than 5%; CPU per frame and GPU memory
  stay within 5%. Missing one is a result to record, not a number to retune.

The player's camera stops at `zoom_max`; slice 00 records whether 2,400 m is
within it. A view the player cannot reach is a diagnostic, never an acceptance
view.

## Looking

Every slice that produces a picture ends with these, in this order:

1. [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md),
   candidate against the current game at the same camera and state, on the
   slice's named crop.
2. A fresh, unprimed
   [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md)
   as the last check before accepting.
3. [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) for the
   user, with about five minutes to answer. Silence means decide on the
   evidence, record the decision in [the ledger](decisions.md), close the
   shots and continue.

Known and out of scope in every crop: the diagonal fog hatch that dominates the
wide view, the HUD, and infantry contrast.

## Tests

Write each test first and watch it fail. Tests set their own thresholds and
fake art; none reads the fixture's `far_px`, a shipped kind's triangle count,
or a constant's value. What stays green throughout: the presentation tests for
view, props and settings; the scene-assets bake and validate tests; the
renderer-core asset-generation and card tests; the native client's `counts`
and `props` tests; and the approved frame baselines under
`web/scenes/baselines/frame/`, re-approved only where a far mesh is in frame
and only with the comparison as evidence.

## Unknowns, and who settles them

| Unknown | Settled by |
| --- | --- |
| Which kinds cost the most today, and how many cast | 00 |
| Whether far geometry still saves GPU time now that tiny props skip their shadows | 00 |
| The far threshold's value | 03, from the zoom crops |
| Whether a wall survives at a few pixels | 04 |
| What is left for block merging after the far mesh ships | 05 |
| Whether terrain geometry costs enough to simplify | 06 |
