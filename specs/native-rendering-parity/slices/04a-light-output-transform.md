# 04a — Light: output transform

**Contract:** native turns scene radiance into display colour the way the browser does: exposure, bloom, grade and AgX tone mapping, then sRGB output (`packages/battle-renderer/src/world/post.ts`, `light/sceneLight.ts` `postSettings`).

**Seam:**
- The manifest gains `light.post`, the resolved fixture values from `presentation.light` (`fixtures/game.json`).
- Native `environment/environment_adapter.gd` exposes `build_post(light) -> LayerResult`.
- Native first tries Godot's built-in AgX tone mapper with matching exposure and glow off. If the built-in doesn't read the same, it ports the browser's post as one `CompositorEffect` at `POST_TRANSPARENT`, with Godot's tonemap set to linear. Record which path was taken in `choices.md`.

**Runnable artifact:** one broad market-town plate frame, browser beside native.

**Visual variable:** overall tone (brightness, contrast, saturation roll-off) only.

**Focused gates:**
- A TS test checks that the manifest's `light.post` equals `postSettings` for the fixture.
- Native parses it with no defaults; a missing field is a blocker.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking, about 5 minutes).

**Out of scope:** sun direction, shadows, sky, haze and terrain colour.

**Delegated:** the CompositorEffect's internal structure, if that path is taken.
