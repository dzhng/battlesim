# 04c — Light: output transform

**Contract:** native turns scene radiance into display colour the way the browser does. It comes last in the light slices because tone can only be judged once sun and sky radiance match (04a, 04b). The steps are: exposure, bloom, grade and AgX tone mapping, then sRGB output (`packages/battle-renderer/src/world/post.ts`, `light/sceneLight.ts` `postSettings`).

**Seam:**
- The manifest carries the browser's raw `presentation.light` plus `sunDirection`; native ports the derivations, and `environment/light.gd` refuses a missing, extra or non-finite field.
- `environment/environment_adapter.gd` builds the post effect.
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
