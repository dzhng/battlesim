# Research and reuse boundary

Inspected 2026-09-25. Sibling source revision: `e93a79aaa2d5f2636cbe4a763ddef1852ddac24b`; working tree was clean when checked. These are source findings, not passing runtime results. Resolve paths from `/Users/david/dev/game` or retrieve this exact Git revision when the sibling has moved.

## Reuse inventory to materialize in slice 01

| Source | Reuse disposition | Verification reference |
|---|---|---|
| `package.json`, `web/package.json`, `web/vite.config.ts`, `Cargo.toml` | Same dependency-free root task runner, web-owned Bun dependencies, Cargo crates, source-only packages/apps, Vite React/TypeGPU integration. Preserve pinned resolved versions, omit unused campaign/Three-only surfaces. | Inspect pinned lockfile and each copied dependency; no workspace-wide upgrade. |
| `packages/renderer-core/src/{camera3d,mat4,depthContract,cameraUniform,device,capabilities}.ts` | Audit transitive imports; port camera/depth/device primitives into the corresponding new owner. XY ground, +Z up, column-major, reverse-Z. | `web/tests/camera3d.test.ts`: screen/world round-trip and reverse-Z monotonicity; `cameraUniform.test.ts`. |
| `packages/battle-renderer/src/sceneLifecycle.ts` | Port resource-lifetime primitive if consumed; not a new parallel lifecycle wrapper. | `web/tests/battle-renderer/sceneLifecycle.test.ts`: pending-operation exclusion and cleanup after failure. |
| `packages/battle-renderer/src/battleScene.ts`, `world/terrain.ts` | Composition exemplars; not standalone modules. Trace crowd/environment/asset dependencies before choosing any additional copy. Build the new primitive scene around audited reusable foundations. | Existing scene has cross-package imports; do not claim complete renderer portability. |
| `web/src/battle/sim/{battleAuthority,battleSimClient,battleSimWorker,simTiming}.ts` | Adapt authority/buffer ownership and scheduling pattern to new contracts. | `web/tests/battleSimAuthority.test.ts`, `battleSimParity.test.ts`, `battleSimPublication.test.ts`. |
| `web/src/battle/sim/{publicationLayout,publicationProducer}.ts` | Reuse transfer/credit ideas only; old layout exports raw truth. | Preserve ownership/alignment tests, replace old all-truth payload expectations. |
| `crates/sim/src/terrain.rs`, `path.rs`, `missiles.rs` | Do not port gameplay policies: height is decorative, pathfinding ignores speed and allows off-map terrain, projectile collision waits for ground landing. | `crates/sim/tests/terrain_height.rs` intentionally asserts old presentation semantics; new tests must not inherit that invariant. |
| `web/scene.mjs`, `web/scene.test.mjs`, `web/scenes/system/battle-elevation.mjs` | Reuse harness conventions, lifecycle/error reporting and deterministic capture pattern, not old game route registries. | Throwing scenes clean up pages; elevation scene demonstrates authoritative probes plus screenshots. |

Write `assets/reuse-manifest.json` during slice 01 with source commit/path, destination, transitive imports, license/attribution, locally changed behavior, and retained test per copied file. No file-copy approval is needed, but unknown licensing blocks that file's reuse; use an original primitive instead. Never import sibling source through runtime filesystem paths in the finished repo.

## External primary sources

- [TypeGPU device initialization](https://docs.swmansion.com/TypeGPU/api/typegpu/type-aliases/initfromdeviceoptions/) and [pipeline API](https://docs.swmansion.com/TypeGPU/apis/pipelines/) support sharing an owned WebGPU device and explicit pipeline preparation. Current docs can differ from the pinned library: reproduce a tiny render with the sibling's actual installed signatures before translating. No automatic upgrade to match current documentation.
- [Rapier scene queries](https://rapier.rs/docs/user_guides/rust/scene_queries/) distinguish ray/shape casts and query filtering. [Rapier CCD documentation](https://rapier.rs/docs/user_guides/templates_injected/advanced_collision_detection) identifies tunneling as the fast-body problem. These inform a swept-query spike; they do not mandate importing a rigid-body engine.
- [Fix Your Timestep, Glenn Fiedler](https://gafferongames.com/post/fix_your_timestep/) motivates fixed simulation steps separated from rendering, bounded catch-up, and explicit slowdown instead of unbounded debt.
- [Deterministic Lockstep, Glenn Fiedler](https://gafferongames.com/post/deterministic_lockstep/) explains why equal inputs require exact reproducibility and why floating-point differences complicate it. Same-build replay is our contract; native/WASM equivalence and future network lockstep are not assumed.

Research consequences are design inferences, not claims these sources implement this game. Slice 01 reproduces the pinned camera/device behavior before adapting it. Slice 07 reproduces analytic flight and a fast projectile crossing a moving thin target before integrating combat. Archive input and output evidence inside the spec; do not label either spike passed during planning.

## Draft synthesis

Independent drafts optimized fewest slices, risk-first validation, and seam ownership. All agreed on Rust truth, side-filtered observation, explicit commands, simple TypeGPU presentation, and narrow reuse rather than old combat. The five-rung minimal draft is a useful milestone summary but hides geometry, information, weapon transitions, and garrison collision in broad rungs; canonical slices separate those seams. Risk-first collision/observation gates are kept before composed combat. Primitive assets and a small village are chosen over a full roster. All full-game decisions remain in `requirements.md` and the continuation slices.

The reference game is an architectural exemplar, not a visual target. No WARNO or Broken Arrow screenshots are required for prototype acceptance; the user requested their tactical inspiration, not an exact visual replica.

## Draft provenance and limitation

Three fresh independent local agents produced fewest-slices, risk-first and seam-quality drafts. The seam-quality agent also attempted the requested cross-vendor consultation with Claude Opus/high. The first read-only run could not read the sibling under its permission configuration; a single retry with explicit sibling access produced no response after 12 minutes and was terminated. No usable Claude plan was obtained, and this is not claimed as a successful cross-model review. The independent local seam draft and verified source findings were synthesized with the other two. No source edits or auth changes were delegated.
