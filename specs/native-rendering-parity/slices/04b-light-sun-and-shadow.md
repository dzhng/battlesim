# 04b — Light: sun, ambient and shadows

**Contract:** native lights the scene with the browser's sun and fill. Shadows fall in the same direction with the same softness.

**Seam:**
- The manifest gains `light.sun` (direction in sim space, colour and illuminance), `light.fill` and `light.cascades`. These come from the browser's resolved light (`light/sceneLight.ts`, `physicalEnvironment.ts`, `cascadePolicy.ts`).
- Native `build_sun(light)` converts the sun through `sim_to_godot`.
- PSSM splits come from the cascade policy.
- Ambient comes from the manifest fill, not from Godot's sky.

**Runnable artifact:** one plate frame with shadows falling on open ground, plus a paris-corner street frame.

**Visual variable:** shadow direction and softness. Buildings are still approximations until phase 2, so judge direction and softness, not shapes.

**Focused gates:**
- A TS test checks the manifest's light against the resolved fixture.
- Native parses it with no defaults.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** sky, haze, tone (04a) and terrain colour.

**Delegated:** how Godot's shadow softness is set (angular size or PCSS) to match the browser's look.
