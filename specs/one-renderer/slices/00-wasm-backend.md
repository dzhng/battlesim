# Slice 00 — browser `wgpu` backend proof

**Status:** complete 2026-10-26.

## Contract

`renderer-wasm-spike` exposes a Rust `wgpu` renderer to browser WASM. It owns
an HTML canvas surface, requests the browser adapter, compiles WGSL, submits a
fullscreen triangle, and exposes resize and shader-source seams. It does not
read simulation state or replace the battle renderer.

## Comparison

The `one-renderer` lab runs four paths for 120 frames: TypeGPU authoring,
exported WGSL through browser WebGPU, Rust `wgpu` through `BrowserWebGpu`, and
a Rust base with a TypeGPU overlay canvas. The scene captures all four panels
and checks mean RGB differences for the shared shader workload.

## Verification

```sh
cargo check -p renderer-wasm-spike --target wasm32-unknown-unknown
bun run --cwd web build:renderer-wasm
WEBGPU_BROWSER=local bun run --cwd web scene -- one-renderer
```

The scene must report four completed rows, 120 frames per row, and zero mean
RGB difference between Rust, exported WGSL and TypeGPU controls. Any visual
shot uses `compare-screenshots` for the control/candidate comparison and a
fresh `screenshot-critique` before acceptance. The final evidence belongs in
`throwaway/one-renderer-spike/`, not in source.

## What this slice does not prove

It does not prove battle-frame performance, full model/material parity, fog,
shadows, postprocessing, GPU lifetime reset, or same-texture mixed interop.
