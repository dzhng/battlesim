# Shader composition

WGSL modules own the rendering rules. The [composition index](pipeline_sources.txt)
selects ordered named blocks for each pipeline; the core build writes expanded
sources only into Cargo's output directory. The typed Rust catalog supplies
pipeline state. Neither platform carries shader text or descriptor policy.

One algorithm has one authored body, even when its callers bind differently
named uniforms. A block reference may supply identifier aliases in brackets,
separated by semicolons: `block[from=to;other=target]`. Substitution replaces
whole identifier tokens once, so aliases can rename colliding locals without
changing expressions, operation order or unrelated names. Binding declarations
remain explicit; aliases do not select a different algorithm.

A built-in shader's source starts with the presentation's
[typed constant table](../../../presentation/src/shader_constants.rs). Each
value belongs to the CPU concept that bounds, allocates or clocks it; the
prelude and generated TypeScript export that same table. Integer constants
remain integers so workgroup sizes and array lengths share the CPU's owner.

GPU-only tuning stays as named constants beside its composed function. Keep
the original literal spelling and inferred type: an abstract WGSL constant
must not acquire an f32 rounding step before the original expression did.

Keep modules grouped by the rule they own. Add a helper to that owner and name
its dependency in the composition index, rather than copying it into another
material or maintaining an expanded pipeline file. The core's reflection tests
validate assembled WGSL against canonical presentation layouts, while real GPU
scenes judge its output and lifetime.
