# Renderer cleanup

**Status:** planned. Nothing is implemented yet.
**Updated:** 2026-10-30.

## Next Agent Prompt

You are implementing the structural cleanup of the shared Rust renderer
(`crates/renderer-core`, shared by native and the browser through
`crates/renderer-browser`; its closed spec is
[`specs/done/one-renderer`](../done/one-renderer/README.md)). Read this README,
then [the map](map.md): it holds the verified ground truth, every decision with
who made it, and the landmines. Load [`renderer`](../../.agents/skills/renderer/SKILL.md)
before GPU work and [`implement-spec-with-codex`](../../.agents/skills/implement-spec-with-codex/SKILL.md)
to run the slices: Codex
(gpt-6.1-sol, high) implements each slice, the orchestrator reviews and
integrates by cherry-pick, and pushes each verified slice to the open PR.

**Status (2026-10-30):** 01 and 02 landed; 03 and 04 in progress.

**Blocker on the plate proof:** the base build doesn't render grass the same
way twice. Paris-corner (little grass) reproduces byte for byte; market-town
and wetlands differ in scattered grass pixels (0.01–0.24%, up to 41 levels),
and the varying pixels take more than two values, so no tolerance or
"matches one of the base renders" test separates noise from change. The likely
cause is the grass build's atomic slot append and its tier cap
(`passes_models.wgsl` `atomicAdd(&args[…])`, `if (slot >= cap)`). A fix that
makes grass deterministic lands first, in its own commit; until then, slices
whose proof is an empty assembled-shader diff proceed, and plate proofs wait.

Warnings:
- Every slice before 15 must leave every plate byte-identical. A byte
  difference fails the slice until traced (see [Proof](#proof)).
- Run GPU work under `lockf -k /Users/server/dev/battlesim/throwaway/gpu.lock`.
- Build output once filled the disk. Reuse one base build per slice, fetch
  only `assets/runtime` LFS into a child checkout, and delete a child's
  `target/` once its work is integrated.

TODO:
- [x] [01](slices/01-windows-cutout.md) Windows cutout as its own WGSL variant
- [x] [02](slices/02-record-schema.md) Record schema: nested records, i32, TS export list
- [ ] [03](slices/03-ground-records.md) Ground records: terrain, grass, scars, masks
- [ ] [04](slices/04-renderer-records.md) Renderer records, lengths and the reverse check
- [ ] [05](slices/05-composition-grammar.md) Composition grammar and one block per binding
- [ ] [06](slices/06-concept-moves.md) Move each struct and function to its concept's module
- [ ] [07](slices/07-names.md) Names: no suffixes, no mangling, no dead aliases
- [ ] [08](slices/08-collapse-copies.md) Collapse copies and loop the cascades
- [ ] [09](slices/09-scenery-tier-mesh.md) Scenery tier mesh: one Rust owner
- [ ] [10](slices/10-world-geometry.md) World geometry: one Rust constructor
- [ ] [11](slices/11-track-side.md) Track side: decided once, in presentation
- [ ] [12](slices/12-frame-timing.md) GPU frame timing: one owner
- [ ] [13](slices/13-move-tests.md) renderer-core's tests in renderer-core
- [ ] [14](slices/14-brittle-tests.md) Tests and scenes assert behaviour
- [ ] [15](slices/15-v-wakes.md) V-wakes read on screen, or `kind` goes
- [ ] Closeout: full `check` and `verify` once, net line count, `close-spec`

Before you end a pass, tick what landed, move the pickup point, and record any
decision the slices didn't make in `choices.md` beside this README (create it
on first use, in the `audit-choices` ledger form).

## Goal

Future agents building renderer features find each concept in one place and
can't break a GPU layout silently. Nothing the player sees changes, except V-wakes
in slice 15, which ship only with the user's sign-off.

## Definition of done

Accepted by the user during the walk:

1. Every shared WGSL struct and function is defined once, in the module named
   for its concept; [`shaders/README.md`](../../crates/renderer-core/src/shaders/README.md)
   maps concept to module.
2. No compiler-mangled names or position suffixes; aliases remain only where
   two callers genuinely bind different names.
3. Every buffer layout a shader reads is checked by a test, and every fixed
   array length and mask in WGSL is a constant exported from its Rust owner. A
   Rust length that the shader doesn't follow fails a test.
4. The scenery tier mesh, GPU frame timing, world geometry and track side each
   have one owner.
5. `cargo test -p renderer-core` runs the renderer's own GPU tests; native keeps
   no re-export shims.
6. The generated TypeScript records hold only what something reads.
7. Slices 01–14 leave every plate byte-identical.
8. Net line count goes down; reported at closeout, not a target.

## Single-owner invariants

Every slice keeps these true, and none adds a layer a later slice must delete:

- **GPU record layouts:** `presentation::records` (the `records!` macro and its
  `LAYOUTS` lists). Packers derive sizes and offsets from it; WGSL is checked
  against it by `crates/renderer-core/tests/pipeline_abi.rs`.
- **Shader constants:** `presentation::shader_constants`, prepended to every
  built-in shader. Each value is owned by the CPU concept that bounds or
  allocates it.
- **The TypeScript records** are a chosen subset, not the whole schema.
- **Shader text:** one authored body per algorithm, in its concept's module;
  `pipeline_sources.txt` composes them; nothing rewrites shader text at runtime.
- **Scene admission:** Rust builds scenery tiers, world geometry and track facts
  for both shells; TypeScript passes inputs, never rebuilds them.
- **GPU timing:** `renderer_core::timing`.
- **Renderer tests:** live in `renderer-core`, with one shared test-device helper.

## Proof

The proof is a scratch script per slice (map Q2), not a committed tool.

1. **Determinism probe (once, first):** build the base `native-client` and
   render one menu-reel scene twice; `cmp` the plates. If they differ, stop:
   the byte-identical rule can't work, and the user decides what replaces it.
2. **Frozen inputs:** film three parity runs once from the base build and reuse
   them in every slice: the menu reel, the menu reel with `PARITY_FOG=1`, and the
   water reel (`bun run --cwd web scene -- native-parity-capture`, with
   `PARITY_CAPTURE=water` for the last). They live in `throwaway/native-parity/`.
3. **Assembled-shader diff first:** dump `renderer_core::pipelines::builtin_source`
   for every built-in shader from the base and the changed build, and diff. An
   empty diff proves a slice that only moves code, with no GPU. A rename-only
   diff gets a token-map check.
4. **Plates:** render the three runs with the base and the changed build
   (`native-client --run <dir> --scene <map> --out <dir>`) and `cmp` every plate.
5. **GPU pixel tests** (high air, impostors, x-ray, probes, marks, fog views,
   frame views, scars, pipelines, water sampling) and the browser scenes each
   slice names.

**A byte difference** fails the slice until traced to its cause. Put the
original expression back, unless the change is deliberate maths; a deliberate
change is a named decision in `choices.md` with diff stats (map unknown 1).

## Slice graph

```
01 ─────────────────────────────┐
02 ─┬─ 03 ─┐                    ├─ 05 ─ 06 ─ 07 ─ 08 ─ 09 ─ 10 ─ 11 ─ 12 ─ 13 ─ 14 ─ 15
    └─ 04 ─┴────────────────────┘
```

- 01 is first because it must land before any WGSL rename, and 03 and 04 rename
  some struct members.
- 03 and 04 both need 02's schema; they touch different records and can run in
  parallel.
- 09–11 all touch scene admission; they run in order to keep each diff
  reviewable.
- 12 touches no pixels and could run beside 09–11; it waits only to keep one
  slice in flight on a shared file set.

**Alternatives considered.** One draft merged everything into six slices
(layouts; shader composition; scene admission; timing; test harness; wakes).
It was rejected because each merged slice needed several verdicts. Another
draft added a committed proof tool as slice 0; the user chose a scratch script
instead (map Q2).

## Firewalls

- No change to battle outcomes or digests: nothing here touches `crates/sim`.
- No change to what any shell draws before slice 15.
- No backward compatibility or migrations: hard cutovers inside the repo
  (map Q6). Nothing outside the repo reads these layouts or records.
- No new renderer features. A latent bug found on the way gets its own commit
  and a `choices.md` entry.
