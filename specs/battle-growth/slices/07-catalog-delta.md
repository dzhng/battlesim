# 07 Catalog delta

**Question:** can Rust stage a complete catalog when the shell supplies only
the art Rust does not hold?

**Depends on:** 02.

## What happens today

Rust keeps unchanged bundles across generations
([appearance.rs](../../../crates/renderer-browser/src/appearance.rs) clones
the live library) but not their catalog facts or placements. So
[`stage_catalog`](../../../crates/presentation/src/presentation/catalog.rs)
needs every asset again, and the shell re-sends all of them.

## Contract (the WASM seam)

The shell declares the wanted art as name, content identity and placement. It
passes bytes only for content Rust reports missing. A staged catalog built
from a delta equals one built from everything.

## Seam

- The renderer answers which of a list of content identities it lacks.
- The draft takes declarations for every wanted name, and bytes for the
  missing content only.
- `stage_catalog` takes the wanted set and the new facts; facts for content
  already committed come from what presentation retains at commit.
- The library keeps what the wanted set names and drops the rest.
- **Deleted:** passing every asset on every admission.

**Invariants:** a draft still cannot revive sources it did not declare, and a
late draft cannot restore stale ones.

## Verification

Write first:

1. [`crates/presentation/tests/catalog.rs`](../../../crates/presentation/tests/catalog.rs):
   staging {a, x, y} with facts for `a` only equals staging with all three.
2. The missing list is exactly the content never supplied; supplying content
   already held decodes nothing.
3. A name declared with a different content than the live one is replaced.
4. `tests/bundle_memory.rs` still holds.

The WASM check passes.

## What you can run or see

Nothing changes on screen. Until slice 08 the browser still sends everything,
which stays valid: known content is a hit.

## Delegated

The binding's function names and how the missing list crosses the seam.

## Stays green

The armory, roster and workbench pages, which replace art under one name
through this seam.

## Feedback that would change this slice

If presentation does not retain every fact a later stage needs, retain it at
commit; do not ask the shell for it again.
