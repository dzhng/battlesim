# 06 — Lighting, shadows, sky, haze, fog, and post-processing

**Contract:** native consumes fixture-owned evaluated environment values and preserves the browser distinction between sunlight, material shading, aerial perspective, fog visibility, and post-processing.

**Seam:** browser light/environment/sky/shadow/post owners and fixture presentation values; native adapter configures one frame-level environment/light pass. Captured fog cells remain Rust-owned visibility evidence.

**Runnable checkpoint:** matched horizon, building-shadow, and close-vehicle cuts with deterministic sun, fill, cascades/contact shadows, sky/clouds, haze, exposure, tone mapping, bloom/AO, and fog.

**Focused gates:** fixture value export, shadow/fog separation, horizon stability across cuts, no hidden-cell leak, backend capability, shader/material validation, and GPU/resource telemetry.

**Visual variable:** illumination, grounding shadows, sky/atmosphere, fog, and post-processing. Grass/model identity is out of scope.

**Acceptance evidence:** sky-only, shadow-only, and fog-separated masks; `compare-screenshots`, unprimed `screenshot-critique`, and `preview-shots`.
