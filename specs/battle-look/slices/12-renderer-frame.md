# 12 — Renderer frame and passes

**Status:** done (2026-09-25). **Depends on:** 01 ([verdict](../spikes/01.md): go; port per its file table). **Lane:** renderer.

## Contract

Replace the one-shader `scene.ts` with a frame graph ported per spike 01's verdict: shadows, then the HDR world (sky, opaque, translucent), then bloom and AgX, then display-space overlays. Adopt the 48-float camera layout, one resource registry, and a `FogTerm` in every world material. Nothing visible changes yet except the pipeline.

## API seam

- `battle-renderer`:
  - `BattleFrame`, `FrameTargets` and `WorldPass`;
  - one registry, `trackGpuAllocations`-backed, with texture bytes counted;
  - the 48-float `CameraUniform`, with a layout test against the WGSL struct.
- `FogTerm(worldPos, normal, pixelCoord, isGround)` is shared by every world material, structures included (spike 02). It still reads the 8 m bitset until slice 14, which swaps only its source.
- The frame provides scene depth before the world colour pass, because slice 14 builds per-tile eye lists from depth in a compute pass (spike 02). The depth arrangement under MSAA is delegated.
- Overlays draw after post in display space, over fog, so their colours are unchanged (landmine 13).
- **Structures are world geometry, not overlays** (spike 01, landmine 1). `buildBattleOverlay` today puts the knowledge-drawn standing buildings in `overlay.opaque`. Split them into a `structures` layer drawn in the HDR world pass, so they are lit, fogged, graded and cast shadows. Remembered ruins and wrecks go there too. Only true display-space marks (selection, orders, labels, tracers' HUD marks) stay overlays.
- Only the camera bind group is pinned to index 0. TypeGPU slots the rest; there is no manual renumbering (spike 01, landmine 4).
- GPU timing reports the frame total from `timestamp-query`. Per-pass splits overlap on Apple's GPU and are not reported (spike 01, landmine 3).
- Every ported file has a manifest entry.

## What you can run or see

`/battle/village` unchanged in look, with a pass inspector in the lab.

## Verification

- Tests:
  - the camera uniform layout;
  - overlay colour isolation (`foundation`, `contacts`, `ballistics`, `geometry` pixel checks);
  - resize, rebuild and reset return every allocation to baseline;
  - texture-byte telemetry.
- The manifest test.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** compositing isolation: overlays identical before and after the new frame.
- **Reference crop:**

No external reference. The before and after frames of the same scenes are the pair.

- **Out of scope:** Every material and light, which are unchanged.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Internal pass decomposition.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

None expected. This slice is plumbing.

## As built (seam items)

- `BattleFrame` (`packages/battle-renderer/src/scene.ts`, built by `frame/battleFrame.ts`): `render(target, ViewportCamera)`, `setWorld`, `setStructures(mesh)`, `setOverlay`, `setInstances`, `setFog`, `setView(FrameView)`, `settled()`, `stats()` (GPU frame time, device texture and buffer bytes, shadow receiver range), `dispose()`.
- Passes: shadows (4 cascades) → depth prepass (4× MSAA depth, sampleable) → sky + HDR world → post → overlays → composite, bracketed by two timestamp markers.
- `FogTerm(worldPos, normal, pixelCoord, isGround)` in every world material, structures included; it reads the 8 m bitset until slice 14 (spike 02).
- The frame provides depth before the world colour pass: a depth prepass into the 4× MSAA depth, colour at `greater-equal` without writes (spike 02; see `choices.md`).
- Structures (standing buildings, remembered ruins and wrecks) draw with fog on (spike 02, landmine 3).
- The ported files and their modes are in the reuse manifest; the decisions this slice made are under "Slice 12" in `choices.md`.
