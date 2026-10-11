# 20b — The static world resolved in Rust

**Contract:** what the world holds before anything happens is resolved once, in Rust, for both clients:
- building template expansion into kit modules and states;
- scenery placement (forest, understorey, backdrop, dressing);
- the terrain tables (height pages, plots, surfaces, params);
- the grass tables (props grid, shapes);
- the tier rules.

The browser draws Rust's output; native draws the same output with no browser export in between. The TypeScript resolvers and the manifest artifacts that carried their output to native are deleted.

**Seam:**
- The resolvers move next to the map and catalog owners in Rust: `mapgen` and `contract`, or a new crate. The research step decides where.
- The output is flat records both renderers consume as GPU instance or table data.
- The browser gets them through WASM, replacing `templateLibrary.ts`, `buildingPlacements.ts`, `scenery/placement.ts`, `dressing.ts` and the terrain and grass packers.
- The manifest keeps identity, lens, light and frame records. Placement no longer travels through it.

**Research first:**
- Each resolver's size and its dependencies on browser-only data, such as the scene-assets catalog loaded in JS.
- Whether Rust can read those catalogs directly.

Write the findings into this file before building.

**Order: web first.** Land the change in the browser first. Measure before and after with slice 07's harness on the graphics test and a medium battle: average FPS, 1% low, p95 and p99, main-thread CPU and GPU timestamps, three runs each. Only then switch native to the same code.

**Focused gates:**
- Equality: the Rust resolvers' output equals today's TypeScript output on both reel maps, record for record, before the TypeScript is deleted.
- The shipped browser draws the same frame. Plates are byte-identical, and the terrain and grass WGSL is unchanged.
- No meaningful browser regression; startup stays within the 30 s budget.
- Less code, as in 19 and 20.
- Native plates and counts unchanged.

**Delegated:** the crate placement and the record shapes.
