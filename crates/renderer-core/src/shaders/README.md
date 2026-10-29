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

A built-in shader's source is its composed modules after the presentation's
look numbers the WGSL reads by name (`SWAY_*`, `WAVE_*`), so a shader and the
Rust that bounds or clocks the same motion read one owner.

Keep modules grouped by the rule they own. Add a helper to that owner and name
its dependency in the composition index, rather than copying it into another
material or maintaining an expanded pipeline file. The core's reflection tests
validate assembled WGSL against canonical presentation layouts, while real GPU
scenes judge its output and lifetime.
