# SG3: road wear read

**Depends on:** none (start here). **Kind:** slice.

## Question
Do a packed core, a jittered shoulder of 2–4 m and soft ruts avoid reading as an outline or a shadow at the default camera (Q-G2, Q-G3)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG3.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
Paint the shoulder and ruts through the existing per-fragment road loop (the village has 2 roads), replacing the road-keyed verge (`terrainMaterial.ts:415-424`). Try two palettes (country road, dirt).

## What the human can run or see
250, 65 and 25 m at the village road.

## Verification
- **Note:** at 65 m a 0.4 m rut is 4–8 px wide, so the "fade below 2 px" rule never kicks in where the game is judged.
- **Kill if**, after two tuning rounds, the unprimed critique still says yes to "does the road read as an outline?" or "could any dark region read as shadow?", or the shoulder must go below grass luminance to read at all.
- **Fallbacks:**
  1. shoulder by hue and grass thinning only;
  2. ruts as roughness and normal only;
  3. ruts cut.

## Delegated to the implementer
Palettes. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.

## Feedback that would change this slice
Wear that hides the road core changes the proposed wear width/contrast before shoulder/rut slices.

## Outcome

**Answered on the way through C67 and C68,** on the real terrain material and the rig's stations, not in a scratch worktree (`../choices.md`). Each row is what the unprimed critique said after the rounds it was given.

| Question | Verdict | What ships |
|---|---|---|
| A packed core per kind | **In.** Both kinds named correctly; no dark region read as shadow. | [C66](C66-road-core.md#outcome) |
| A shoulder of 2 to 4 m in a worn colour of its own | **Killed.** "A sandy outline" in every gravel-road frame from 25 m to 250 m after two tuning rounds; in the road's own colour it read as torn paper. It never had to go under the grass's luminance to read. | Fallback 1: **hue and grass thinning only** ([C67](C67-road-shoulder.md#outcome)), faded out by the tactical camera, where a few-pixel fringe read as a halo |
| Soft ruts on the country road | **Cut** (fallback 3). "Pencil lines" and "pinstripes down the lanes" at 65 m in both shapes tried. | None on `roads.default` |
| Soft ruts on the dirt track | **In.** "A grass strip growing between two wheel ruts" at 25 m. | [C68](C68-ruts-and-centre-strip.md#outcome) |

**The kill check, as it fell out.** The outline question was answered yes for the full shoulder and for ruts on a wide road, and no for the core, the wash up close, and the track. The shadow question was never answered yes for anything on a road.

**What this leaves.** A wide gravel road has a textured surface and a soft edge up to about 120 m, and nothing across it: at the tactical camera it is still a pale ribbon. Lines along it are not the answer. Wear that follows travel would need a distance along the road, which the export does not carry.

**What the three slices learned about painting ground from noise.**
- A level cut through one octave of value noise shows the lattice as straight, axis-aligned edges on every blotch. Turn each octave off the grid and read them at a point a broader octave has pushed about.
- A cooler patch reads as shadow at equal luminance, and two tones in equal shares read as stains or camouflage. Variation inside a surface goes to the warmer hue only, sparsely, at the surface's own luminance.
- One fade for a whole grain draws a line across the ground where it switches off. Each octave fades as its own cells near a pixel.
- A band of a third colour along an edge is an outline, however ragged its far side. A band a few pixels wide is a halo, whatever its colour: fade it out by its width in pixels before it gets there.
- Thin parallel lines along a wide road are pinstripes, and a strip broken along its length is a dashed line. Ruts read as ruts only on a track narrow enough that two of them and the grass between are the whole road.
- "Never darker than" is built, not tuned: a wash over other ground is lifted to the luminance of what it lies on.
- Mean-zero detail fades to nothing: a rut that darkens its line and lightens the rest leaves, once faded, the pixels of the road without it.
- Queue a GPU job from a copy-on-write snapshot of what the scene serves (`cp -cR` of `apps`, `assets`, `fixtures`, `packages`, `web` into `throwaway/`), not from the tree being edited: a scene waiting on the GPU lock starts whenever the lock frees, and from then on any save reloads its page.
