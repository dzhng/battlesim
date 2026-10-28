# 08: Caustics

**Lane:** render. **Depends on:** 07. **Visual variable:** the caustic pattern
on the bed.
**Status:** done (2026-10-28).

## Contract

Procedural caustics are computed in the water shader (`waterCaustics`,
`frame/waterCaustics.ts`):

- two samples of the normal texture, each distorting the other's coordinates,
  with crests where they agree, drifting with the surface's own layers;
- projected onto the bed reconstructed from the straight depth, along the
  view and the sun;
- faded with depth, masked off steep walls and stopped in the surface's
  shadow (RedGPU `lakeFragment.wgsl:435-501`), and faded where a pixel can't
  hold the finer layer's finest wave.

They are gated by `waterDetail` (`TerrainParams.view.z`) and are off on Low.
Each look carries their numbers (`WaterLook.caustics`, from
`presentation.terrain.water.caustics`).

## What changed from the plan

- The pattern's cells are several times a real stream's, so they read at the
  close framing ([choices](../choices.md)).
- A crest narrower than its pixel widens and dims, so the thin lines don't
  crawl at the low view.

## Verification (done)

1. `compare-screenshots` on the shallows at the close, tactical and low
   framings, refraction at slice 07's accepted output, against
   `../assets/redgpu-overhead-shadows.jpg`:
   - Close: before, the shallows were a plain gradient. Now a net of light
     lies over them, as in the reference, stopping at the waterline and
     fading into the channel.
   - Low: smooth caustic lines on the near shallows.
   - Tactical: unchanged; at 260 m no pixel can hold the pattern, and it
     fades out.
   Verdict: less wrong. The bed itself still has no detail of its own.
2. Low: the frame scene's Low baselines (foundation, river, geometry) differ
   by 0 px. One run of three found 1 px on the river and the next found
   none: a flake for slice 18's full run to chase.
3. Cost: four taps a water pixel where it refracts. High interleaved, `full`
   against depth colour alone (refraction and caustics together): 7.32 ±
   0.24 ms against 7.47 ± 0.85 ms, within noise.
4. Two rounds of unprimed `screenshot-critique` on the shallows crops:
   - The first found dark single-pixel flecks and crawling lines at the low
     view, a cobbled look, and a bare strip at the waterline. These are
     fixed: the bed is rebuilt from the straight depth, crests are
     antialiased, and the fade starts in the first centimetre.
   - The second found the cells too small to read at full scale, so the
     pattern grew to a 24 m tile.
   - Their notes on the streaks (09), the waterline's pale line (05), the
     bed showing at grazing angles with no loss to reflection (09), and
     the river reading flat from the tactical height stay with those
     slices.
5. The water scene passes on both clients, native within ΔE 0.0 of the
   browser.
6. Codex found the crest's `smoothstep` edges reversed, undefined in WGSL;
   it is now `1 − smoothstep` the right way round, and the plates are the
   same to within encoding noise.

## Native

Free: the shared shader, the shared look record.

## Must stay green

Digests, replays, and Low shots.
