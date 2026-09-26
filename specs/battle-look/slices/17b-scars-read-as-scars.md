# 17b — Scars read as scars, never as shadow or fog

**Status:** planned. **Depends on:** 17. **Lane:** renderer.

## Contract

Slice 17 built the scar pipeline: authoritative learned cells, a delta texture upload, material terms and the grass response. It failed the standing gate. The final unprimed critique answered **yes** on 5 of 6 frames to "Could any dark region be mistaken for sun shadow, or any shadow for fog?":
- a crater grid reads as egg-crate mounds or camouflage;
- scorch between craters reads as smoke or cloud shadow;
- single craters read as mud or flat puddles at grazing views;
- old trampling reads as a pale path.

This slice closes the gate by fixing the scar look only.

## API seam

- The scar texture's footprint gets sharper than bilinear 1 m cells: for example a 2× supersampled mask with a cubic or signed-distance reconstruction for crater lips. It still derives only from the authoritative learned cells.
- Tighter scorch: a smaller, higher-contrast, sooty-radial shape centred on bursts, not a diffuse wash.
- Bowl parallax or a relief term, so craters read as depressions at grazing views. This is shading only: simulation triangles, shadows and FogTerm are untouched (slice 17's rule).
- The look numbers stay in `fixtures/biomes/summer.json` under `scars`.

## Verification

- Slice 17's tests stay green: uploads, deltas, resync, and only learned cells drawn.
- **Gate:** an unprimed screenshot-critique asked exactly "Could any dark region be mistaken for sun shadow, or any shadow for fog?" answers no on slice 17's six scar frames and the village crater framings, under `dusk`.
- compare-screenshots against the Defilade crater crops.
- Record frame cost (benchmark short run).

## Decision budget

- **Delegated:** the reconstruction technique and the numbers.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Every existing scene and test. `bun run check` and `bun run verify` at closeout.
