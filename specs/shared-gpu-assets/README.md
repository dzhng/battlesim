# Shared GPU assets across generations (placeholder)

**Status:** placeholder. Cross-generation GPU reuse is not implemented or sliced.
**Updated:** 2026-10-29.

## Next Agent Prompt

You are picking up the remaining asset-replacement memory cost after the
[shared renderer port](../done/one-renderer/README.md). Read its
[memory evidence](../done/one-renderer/memory.md), the
[shared renderer contract](../../crates/renderer-core/README.md), and
[`renderer`](../../.agents/skills/renderer/SKILL.md). Trace current admission and
resource lifetimes, then use `auto-research` and `write-spec` to turn this
placeholder into a measured implementation plan. Start with a matched
asset-generation transition and determine which immutable allocations are
actually duplicated. Preserve the atomic catalog commit and uninterrupted
battlefield throughout. Update this handoff with findings and the next pickup
point before ending a pass.

## Goal and contract

Live and pending asset generations should hold shared references to unchanged
immutable GPU assets on the same device. Prepare only missing or changed assets,
then commit the complete catalog and its matching routing tables together. The
old battlefield remains drawable while preparation runs; failed, cancelled or
stale work must leave its art and routing intact.

This is the usual shared-resource/asset-handle pattern applied to our transactional
catalog replacement. It should reduce duplicated allocations during overlap;
whole-browser peak reduction is a hypothesis to measure, not a promised amount.
Changed assets, generation-local tables, upload staging, CPU allocations and
browser/driver reclamation can still produce a transition peak.

## Starting point and constraints

- [Asset replacement](../../crates/renderer-core/src/runtime/assets.rs) already
  stages complete owners and commits matching presentation facts and GPU
  resources after checking device/runtime/catalog identity. Keep this authority.
- [Model admission](../../crates/renderer-core/src/models/admission.rs) shares
  immutable geometry among appearance rows within a generation. Decoded CPU
  texture payloads already use a weak content index. Extend existing ownership;
  another global asset service or permanent strong registry is not the goal.
- Reuse requires immutable content identity **and** compatible GPU representation
  on the same device. Include authored state/pose and every packing/baking input
  that affects bytes; names, row numbers and source paths alone are insufficient.
- [Packed geometry](../../crates/renderer-core/src/models/geometry.rs) currently
  embeds generation-relative material indices. Model textures are combined into
  per-generation arrays. Establish whether stable local addressing or a different
  resource grouping is needed before claiming these allocations are reusable.
- Share only completed immutable resources. Instance buffers, palettes, routing,
  mutable uniforms and placement knowledge retain their proper owners. Baked
  impostors require all relevant geometry/material/lighting/bake inputs in their
  reuse identity; shared resources must never be overwritten during preparation.
- Generations and pending GPU work retain the resources they use. A lookup index
  must not keep retired art alive indefinitely. Repeated replacement/cancellation
  must stay bounded; device recreation must rebuild from retained CPU sources,
  without reusing old-device GPU handles. Let `wgpu` manage submitted-work safety.
- Implement the policy once in shared Rust for native and browser. Keep shaders,
  visual quality, simulation digests and side knowledge unchanged. No saved-data
  migration or compatibility layer is intended.

## Evidence required before acceptance

Use matched transitions for identical catalogs, a small art change, reordered
material/appearance rows, and substantially different catalogs. Demonstrate
allocation reuse for unchanged compatible assets and replacement for changed
ones, including authored poses. Exercise cancellation, failed baking, stale
completion, repeated swaps, in-flight work and device recreation.

Measure full Chromium process-tree memory through loading, swap and retirement,
with repeated interleaved baseline/candidate runs and all variance retained.
Separately report unique GPU buffer/texture payload, decoded CPU data, WASM
capacity, upload/bake work, transition latency and frame CPU/GPU timing. These
categories overlap and must not be summed into a total. Reuse the archived
memory methodology and record source/build/workload identities; a WASM-only
comparison cannot establish an application-memory win. Check native lifetimes
and performance separately. Record maintained code/complexity cost as well.

Preserve frames during loading and matched post-swap pictures. Use
`compare-screenshots` against the frozen baseline and an unprimed
`screenshot-critique` before accepting visual preservation; show evidence with
`preview-shots`. Keep existing regression thresholds and deterministic battle
outputs. Set numerical acceptance budgets after reproducing the baseline and
before evaluating the candidate.

## Engine precedents

- [Unity Addressables memory management](https://docs.unity3d.com/Packages/com.unity.addressables@1.21/manual/MemoryManagement.html)
  documents reference-counted assets and dependencies, with unloading eligibility
  after references are released; bundle lifetime can delay actual reclamation.
- [Bevy asset handles](https://docs.rs/bevy_asset/0.20.1/bevy_asset/enum.Handle.html)
  reference shared assets rather than copying them, with strong handles retaining
  ownership. [RenderAssets](https://docs.rs/bevy_render/0.20.1/bevy_render/render_asset/struct.RenderAssets.html)
  maps source asset identities to render representations.

These are precedents for identity and lifetime management, not evidence that
those engines implement our exact atomic catalog-swap protocol or that our GPU
memory savings are already established.
