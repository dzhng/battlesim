# 14 — Fog geometry: sight lights

**Status:** planned. **Depends on:** 02 ([verdict](../spikes/02.md): sight lights), 04, 13. **Lane:** renderer.

## Contract

Sharp urban fog of war (Q4, F1): what your units see, cut by the actual building and terrain silhouettes, per pixel, using spike 02's chosen technique. It is driven by `OwnUnit.sight` at the published tick, never interpolated. Foliage attenuation and canopy match the simulation. The 8 m sweep stays the rule for learning.

## API seam

- `FogVisibility` consumes `OwnUnit.sight`, the terrain triangles and known props, and produces the visibility that `FogTerm` samples.
- Shape coefficients come from Rust. The GPU uses the same piecewise definition, pinned against Rust oracle vectors.
- Whether a unit is drawn is still decided by identification. Fog only shades the world (decision).
- **The technique is settled by spike 02.**
  - Each own eye has a polar horizon map: azimuth rays × log-spaced radial bins, one word per bin (horizon slope f16, the jump position within the bin u8, foliage metres u8). Terrain is marched coarse; buildings are intersected exactly at full azimuth resolution.
  - Only eyes that moved are rebuilt. Turret traverse never triggers a rebuild, because the sight shape is applied per pixel.
  - A per-frame compute pass lists, from depth, which eyes reach each screen tile. `FogTerm` tests only those eyes.
- **Build in the prototype's five fixes (spike 02):**
  - the jump position within a bin;
  - probes 0.1 m outside faces, with a facing test per eye;
  - a facade blend whose tolerance scales with the viewing angle;
  - foliage interpolated linearly within a bin;
  - the coarse-terrain / fine-building split of the rebuild.
- Provisional numbers go in `presentation.fog_geometry`: 4096 azimuth rays, 512 terrain rays, 64 radial bins, 16 px tiles, a 0.1 m face probe.

## What you can run or see

`/lab/fog` over the village street: seen/unseen as a debug mask, plus the live frame.

## Verification

- Tests:
  - CPU/GPU boundary agreement against oracle vectors;
  - turret rotation moves a vehicle's lobe;
  - garrison eyes;
  - hidden props never occlude (metamorphic);
  - the disagreement metric against the simulation sweep is within spike 02's bar (5%; the spike measured 0.02–0.60%), as a scene assertion;
  - frame cost at 100 a side is within spike 02's measured 0.5–1.6 ms typical fog cost, with the worst case recorded.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** the sight boundary: sharpness and where it falls.
- **Reference crop:**

`armaphract/x-urban-fog-t9s.jpg`, the lit sight wedge between buildings (roughly x 230–740, y 280–560 on the 852×720 frame).

- **Out of scope:** ARMAPHRACT's lavender halftone and dark city palette, and the unseen style (slice 15).

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** The acceleration structure.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the edges read too hard or too soft, change only the edge filtering. Coverage follows the simulation.
