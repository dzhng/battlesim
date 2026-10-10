# 04a — Light: sun, ambient and shadows

**Contract:** native lights the scene with the browser's sun and fill. Shadows fall in the same direction with the same softness.

**Seam:**
- The manifest carries the browser's raw `presentation.light` plus `sunDirection`; native ports the derivations, and `environment/light.gd` refuses a missing, extra or non-finite field. The sun colour, cascade splits and fill derive as in `light/sceneLight.ts`, `physicalEnvironment.ts` and `cascadePolicy.ts`.
- `environment/environment_adapter.gd` converts the sun through `sim_to_godot`, and `prepare(camera, viewport, focus)` runs once per frame.
- PSSM splits come from the cascade policy.
- Ambient is the browser's sky radiance times `fill`, written into Godot's sky slot, as the browser's environment light is.

**Runnable artifact:** one plate frame with shadows falling on open ground, plus a paris-corner street frame.

**Visual variable:** shadow direction and softness. Buildings are still approximations until phase 2, so judge direction and softness, not shapes.

**Focused gates:**
- A TS test checks the manifest's light against the resolved fixture.
- Native parses it with no defaults.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** sky and haze (04b), tone (04c) and terrain colour. Until 04c, native shows linear radiance through Godot's own linear tonemap, so judge direction and softness, not brightness.

**Delegated:** how Godot's shadow softness is set (angular size or PCSS) to match the browser's look.
