# 07 · Names: no suffixes, no mangling, no dead aliases

**Contract.** Every block and identifier is named for what it is. No
`_primary`/`_alternate`/`_extended`/… suffixes, no compiler-mangled names
(`params_1`, `item_1`, `_arg_0`, `*_Input`/`*_Output`), and aliases only where
two callers genuinely bind different names. Slice 05's dead-alias panic enforces
the last part.

## Sub-slices, each landed and proved alone

1. **Probe:** rename `params_1` to `params` across the fog blocks, dropping its
   44 aliases. If the menu reel with `PARITY_FOG=1` isn't byte-identical, renames
   aren't free: stop and reslice.
2. Fog and sight.
3. Models.
4. Terrain, scenery and grass.
5. Water, sky and post.

## Verify (each sub-slice)

- A scratch token-map check: the assembled-shader diff is a consistent
  identifier rename in every pipeline.
- All three reels byte-identical; GPU pixel tests; browser scenes `fog`,
  `fog-look`, `water`, `ground`, `street`, `air-high`.

**Delegated:** every new name, following `shaders/README.md`'s concepts.
**Stays green:** the struct names `pipeline_abi` reflects (they match record
names), entry point names the registry uses.
