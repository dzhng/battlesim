# 01 — Spike: renderer port feasibility

**Status:** done (2026-09-25). Verdict: **go**, in [`spikes/01.md`](../spikes/01.md). The pruned closure is 33 files and 3,697 lines, and the `~unstable` encoder works. Four receiver-fitted cascades show no acne and swim within 2–3 texels. Overlays are pixel-identical after post, and allocations return to baseline. The frame costs 1.2–1.5 ms of GPU time and textures grow from 63 to 231 MiB. Two landmines change slices 12 and 13: the overlay mesh carries the standing buildings, and the source's cascades split from the near plane. **Depends on:** 00. **Lane:** spike (throwaway).

## Contract

Can `~/dev/game`'s renderer foundation run on our world, on a pruned import closure? The foundation is the frame, HDR, sky and PMREM, cascaded shadows, and AgX. Our world means triangle terrain, the 40→48-float camera, our resource registry, and overlays after post.

## API seam

A throwaway route `/lab/spike-foundation` in a spike worktree, which is never merged. It renders the village with:
- our `world_layout()` triangles, with no 1.6× relief;
- 3–4 cascades retuned for 0.3–1.6 km;
- an rgba16float target and AgX;
- today's overlays composited after post;
- the 8 m fog term injected into the ported fragment shaders;
- every allocation registered;
- the 48-float camera.

## What you can run or see

The spike route, plus `spikes/01.md` with numbers and screenshots.

## Verification

The verdict records:
- the pruned closure: files and lines per source package;
- which files became `technique` ports;
- frame cost against row 0;
- shadow acne or peter-panning at WARNO height and at Broken Arrow ground height;
- overlay colours unchanged after post;
- whether the allocation baseline returns after resize and reset.

**Kill criteria**, any of:
- the pruned closure is over about 4k lines, or needs `game-renderer/environment` wholesale;
- the `~unstable` encoder fails;
- cascades can't cover 5 m–1.6 km without visible acne or swimming;
- overlays can't composite after post without regrading.

**Fallback:**
- for closure or encoder failure: `technique` ports, hand-written from the sources (CSM, AgX, an analytic sky, roughly 800 lines);
- for cascade failure: a single fitted map plus contact shadows, and ask the user.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Everything inside the spike; it is thrown away.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

None beyond the verdict. Slices 12–13 consume it.
