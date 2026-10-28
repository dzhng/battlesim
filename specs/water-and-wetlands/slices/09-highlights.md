# 09: Fresnel and sun highlights

**Lane:** render. **Depends on:** 06 (after 08, if run in order).
**Visual variable:** highlights.
**Status:** done (2026-10-28).

## Contract

- Water's response goes through the shared light owner (`environment.surfaceLight`
  and `standardPbr`). There is no second lighting model (`frame/waterLight.ts`).
- Water's F0 is 0.02: half metal on a black base, in the shared owner's
  terms. The sun and the sky reflect in three lobes of rising roughness
  (`presentation.terrain.water.lobes`, RedGPU `lakeFragment.wgsl:503-609`).
  The light from under the surface is weighted by Schlick's transmission, so
  a grazing view turns to mirror. The body's own lit colour is the
  in-scatter.
- The streaks of light along the stream (slice 06's) are deleted.
- **Screen-space reflections (decision 15):** not built; the call is in the
  [README](../README.md#what-we-can-copy).

## What changed from the plan

- No separate in-scatter term ([choices](../choices.md)).
- The streaks' removal rides here, as slice 06 left it.

## Verification (done)

1. `compare-screenshots` at the low and tactical framings, against slice 08's
   plates and RedGPU's landscape stills:
   - Low: the blown-out sun glints become sharp sparks over a broader sheen,
     and the bed fades as the view grazes.
   - Tactical: the streaks' faint lines are gone and the band is smoother.
   - Close: calmer, and more like water without the brushed-metal lines.
   A sharper first lobe (0.07) was tried and put the sun in one bloomed
   hotspot; it was reverted.
2. An unprimed `screenshot-critique` of the three framings and their crops
   found:
   - soft, blobby glints (they are the normal map's waves, not the lobes'
     roughness);
   - no sparkle from above;
   - a deep channel and a tactical river that read as a dark smear, a tube
     or asphalt;
   - a generic sky reflection with no bank or trees.
   The last is the SSR call. The rest are the whole-look read below.
3. Checkpoint (decision 20, non-blocking): the three framings beside
   RedGPU's were shown in Preview, and five minutes passed without feedback.
   Decided on the evidence: close and low read as water. The tactical river
   doesn't yet, because its normals fade flat at 260 m; slice 18 takes it
   ([README](../README.md#still-open)).
4. The water scene passes on both clients, native within ΔE 0.0 of the
   browser. The frame scene's Low baselines differ by 0 px: their framings
   are far enough out that the normals lie flat.
5. `fixtures/parity/exact-numbers.json` was re-recorded for the retuned
   presentation.

## Native

Free (shared shader and light).

## Must stay green

Digests, replays, and slices 04 to 08's crops away from highlights.
