# 01 — Browser capture run

**Contract:** one unattended browser run writes, for each scene, a `battle-presentation-capture`, a `native-visual-presentation` manifest and control plates. Every plate is read back from the canvas in the same frame as its frame record. This is the only slice that changes the capture format in phase 1. The frame record is complete from now on, and later slices only add manifest sections.

**Format (changed in place, no `/v1`):**
- **Frame record:** `{index, reelMs, tick, simTime, eye[3], target[3], up[3], canvas[w,h], plate}`, in sim space and metres.
  - `simTime` is the interpolated sim time the frame drew with. Grass wind reads that same clock (`useBattleSession.ts:714`, `grassPass.ts:584`), so there is no separate wind field.
  - `eye` and `up` come from `eyePosition` and `screenAxes` in `camera3d.ts`, applied to the drawn `Camera3DParams` the viewport hands the pilot (`LabViewport.tsx:821`).
  - `plate` is the plate's file name, or null.
- **`samples[]`:** keeps `{tick, digest, publication}` and drops `camera`, which had no reader.
- **Manifest:** `{schema, identity{scene, map, encounter, seed, mapHash, captureHash}, viewport{width, height, scale:1}, lens{fovY, near, depth:"reverse-z", far:"infinite"}, layers[], omissions[{owner, reason, sourceId}]}`. For the menu reel, `layers` excludes the fog overlay.

**Seam:**
- **Types and validators:** `web/src/battle/benchmark/presentationCapture.ts` (capture and frame record) and a new `web/src/battle/benchmark/visualPresentation.ts` (manifest type, validator, writer). Both are strict and reject the old shapes.
- **Recording:** `apps/battle-lab/src/benchmark/reelRun.ts` builds the frame record in `pilot.frame`. `ViewportPilot.frame` gains `simTime` and `canvas`.
- **Readback:**
  - `LabViewport.tsx:636` configures the canvas with `usage: RENDER_ATTACHMENT | COPY_SRC`.
  - On a plate frame, the pilot copies the current canvas texture to a buffer right after `draw()`, in the same task, before present. The BGRA swizzle is handled.
  - Plate frames are shot midpoints where the reel's `black` is 0 (`reelRun.ts:144`). Only those frames pay for the copy.
- **Route:** `apps/battle-lab/src/routes/graphicsTest.tsx` gains `?scenes=<map,…>`, so one scene can run alone, and `?plates=1`, which enables readback and marks the report's frame timing as not comparable.
  - The run fails fast with a named reason when battle resources abort. Today it hangs in `running`.
- **Harness:** a new scene fixture `native-parity-capture` (`web/scenes/native-parity-capture.mjs`, registered in `apps/battle-lab/src/fixtures.json` with its own timeout).
  - It passes 1280×720 explicitly and keeps the page visible.
  - It runs under the GPU lock and waits on the route's stage, never on timers.
  - It names a missing prerequisite: wasm, the mechanics validator, or an LFS runtime bundle.
  - It writes to `throwaway/native-parity/<run>/<scene>/{capture.json, manifest.json, plate-<index>.png}`.

**Runnable artifact:** `lockf -k <main>/throwaway/gpu.lock bun run --cwd web scene -- native-parity-capture` produces the folder for paris-corner in about a minute (`?scenes=` set by an option), or for both scenes in about 4 minutes.

**Visual variable:** none. Check by eye that the plates show the battle with no HUD, no veil and no fog overlay.

**Focused gates:**
- Vitest: the capture and manifest validators accept the new shape and reject the old one and the `/v1` id; plate-frame selection is a pure function; frame records are monotonic.
- The `graphics-test` route smoke stays green.
- Digest and replay are unchanged; the sim isn't touched.
- Run [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md) on one plate as the last check (is it a clean battle frame?).

**Out of scope:** any native change, light, terrain or grass values in the manifest, and units.

**Delegated:** PNG encoding, how the scene receives plate bytes (an exposed binding or a download), and how the capture JSON is laid out on disk.

**Stays green:** the existing benchmark and graphics-test reports, the `graphics-test` scene, and `web/tests/presentationCapture.test.ts` (rewritten for the new shape).

**Feedback that would change it:** plates that disagree with what a person sees on the route, which would mean the readback is reading a different frame.
