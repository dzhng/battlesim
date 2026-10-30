# 05 · Composition grammar and one block per binding

**Contract.** A binding is declared once; the composition index says which
group it sits in. The build refuses blocks nobody uses, duplicate block ids and
aliases that rename nothing.

## Seam

- `build.rs`: a block reference written `block@N` substitutes `@group(GROUP)`
  in that block's body. Panic if a block containing `GROUP` is referenced
  without `@N`, or `@N` is given to a block without `GROUP`.
- `build.rs` panics on: a block id defined twice (today `blocks.insert` at
  `build.rs:51-60` silently overwrites); a block no pipeline references; an
  alias whose `from` token doesn't occur in its block.
- Collapse binding variants that differ only in `@group` into one block each.
  The seam draft counted about 50 blocks in 17 sets after normalising
  `@group`. Variants that differ by binding number or access mode are bindings
  of different layouts: they stay separate blocks, named for their layout (for
  example fog-build `maps`, binding 7 read_write vs binding 3 read,
  `shared_bindings.wgsl:254-266`).
- New unit test in `pipeline_registry` (naga as a dev-dependency): for each
  pipeline, every global at `(group, binding)` matches the registry layout's
  entry in presence and kind. Today a wrong group is only caught when a GPU
  builds the pipeline.

## Verify

- The assembled-shader dump is byte-identical to base. No plates needed.
- Scratch-prove each new panic fires once.

**Delegated:** the reference syntax if `@N` collides with something; block
names for the per-layout variants.
**Stays green:** every assembled shader, byte for byte.
