# Pipelines

The browser's shaders and pipeline settings, as a native client loads them:
the shaders exist once, written by the browser's TypeGPU passes. [The export
test](../../web/tests/shaderExport.test.ts) writes them, from the browser's own
code, into [`packages/battle-renderer/pipelines/`](../../packages/battle-renderer/pipelines/):
one WGSL file per pipeline, and `pipelines.json` with each pipeline's stages,
vertex buffers, targets, depth, multisampling and bind group layouts by name.
That test fails when the browser's text no longer matches the files.

The text is the same for every battle and every canvas size. What a battle
or a size changes (the light, the bloom's texel steps) reaches the shaders
through uniforms, so one compiled set serves every map.

This crate's test holds the export to what `wgpu` accepts: naga parses and
validates every file, writes it as Metal Shading Language, and finds each
pipeline's entry points.

A second test holds the uniform records the presentation crate resolves to
the structs the shaders declare, member by member, so the [native
client](../../native/client/README.md) uploads their bytes as they are.
