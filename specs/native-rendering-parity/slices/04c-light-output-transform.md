# 04c — Light: output transform

**Contract:** native turns scene radiance into display colour the way the browser does. It comes last in the light slices because tone can only be judged once sun and sky radiance match (04a, 04b). The steps are: exposure, bloom, grade and AgX tone mapping, then sRGB output (`packages/battle-renderer/src/world/post.ts`, `light/sceneLight.ts` `postSettings`).

**Seam:**
- The manifest gains `light.post`, the resolved fixture values from `presentation.light` (`fixtures/game.json`).
- Native `environment/environment_adapter.gd` exposes `build_post(light) -> LayerResult`.
- Native ports the browser's post chain by formula (`packages/battle-renderer/src/shaders/post.ts`: bloom high-pass, blur and composite; grade; AgX; sRGB output) as one `CompositorEffect` at `POST_TRANSPARENT`.
- Godot's tonemap is set to linear, and its glow is off.
- Godot's built-in AgX is not used: the browser's grade has split-tone, shadow lift and lift tint, which Godot's environment can't express.

**Runnable artifact:** one broad market-town plate frame, browser beside native.

**Visual variable:** overall tone (brightness, contrast, saturation roll-off) only.

**Focused gates:**
- A TS test checks that the manifest's `light.post` equals `postSettings` for the fixture.
- Native parses it with no defaults; a missing field is a blocker.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking, about 5 minutes).

**Out of scope:** sun direction, shadows and sky radiance (04a, 04b), and terrain colour.

**Delegated:** the CompositorEffect's internal structure.
