# Mechanics editor

The local authoring app makes gameplay tuning readable in battlefield terms.
It runs within the existing development server, with a searchable unit spreadsheet
and expanded explained controls. Authored fixture JSON remains the source of
truth; the editor has no separate database or gameplay schema.

## Why these boundaries matter

A developer thinks about where rounds land, while the simulation stores angular
scatter. Raw and computed controls are therefore both editable. Landing spread
expresses baseline one-axis launch spread in metres at maximum engagement
range, not a maximum miss or blast radius. Movement, suppression, cover and
guidance can alter observed impacts. Changing range preserves that visible spread. Percentages,
cadence and full dimensions follow the same reversible conversion principle.
Invalid partial text stays visible rather than silently turning into zero.

Units inherit shared definitions. Editing a selected unit must not accidentally
change its siblings: unit overrides stay sparse, and soldier edits create local
variants bound to the selected authored row. Descendants follow normal
inheritance; preview discloses their changes too. Restoring inheritance removes
the selected authored override. Weapons are intentionally global; preview shows every affected user.
Values masked by upgrade parts must fail visibly rather than appear saved.

JavaScript owns forms, editable-field restrictions, model-fit checks and
publication; Rust owns catalog resolution and shared gameplay admission. The same
catalog, supply and flight checks used by battle startup admit previews. Original
source text also reaches Rust, allowing duplicate keys to be rejected even
though JavaScript parses a separate UI view.
The generated catalog is saved using Rust's exact returned bytes: equal JSON
values alone do not satisfy its existing freshness contract.

Publication protects both accepted generations and work made elsewhere. Preview
never publishes draft changes. Reads may first recover an interrupted save. Save revalidates the draft's source revision, serializes this store's operations,
rejects competing publications,
and stages individual atomic file replacements. A scratch journal supports
rollback and interrupted-save recovery across files; it does not turn a set of
files into an operating-system transaction. Detected outside edits are preserved, and a
conflict prevents coordinated editor readers from serving a mixed generation.
Arbitrary filesystem readers have no operating-system snapshot guarantee.
An outside writer can also race between a byte comparison and replacement;
ordinary file replacement provides no compare-and-swap against external writers.

A running battle is an experiment with one captured rules and catalog
generation. Save must not reload it or modify its worker. In development, newly loaded battle pages and
explicit restarts capture current accepted sources together. Diagnostic resets
retain the captured generation so comparisons remain reproducible.

## Ownership

- [The app](../../../apps/mechanics-editor/README.md) explains local authoring.
- `MechanicsStore` in [server.ts](../../../apps/mechanics-editor/server.ts)
  owns source addresses, revisions and publication. Its Vite plugin serves the
  API only in development and accepts local, same-origin writes.
- [fields.ts](../../../apps/mechanics-editor/src/fields.ts) owns allowed gameplay
  fields, explanations and reversible conversions. [draft.ts](../../../apps/mechanics-editor/src/draft.ts)
  owns the restored draft projection; the form must display that projection.
- [mechanics_validate.rs](../../../crates/sim/examples/mechanics_validate.rs)
  admits supplied sources through `sim::fixtures::admit` and `weapons::check_rules`.
- [startWithMechanics](../../../web/src/mechanicsLifecycle.ts) captures accepted
  data before gameplay modules load; session restart owns the refresh boundary.
- [The editor tests](../../../web/tests/mechanicsEditor.test.tsx),
  [publication tests](../../../web/tests/mechanicsServer.test.ts) and
  [lifecycle tests](../../../web/tests/mechanicsLifecycle.test.ts) pin the
  editing, filesystem and generation contracts at their consumer boundaries.

## Alternatives ruled out

The existing development server avoids another service; keeping authored
fixtures authoritative avoids a second persisted representation. Direct
browser paths would give forms filesystem authority. Recompiling Rust for every
preview would waste work on embedded fixture data, so development builds the
validator once and requests pass current source text to it. JavaScript catalog
reserialization changes canonical number spellings; its parsed view is for the
UI, while the original native text is for publication.

Live fixture hot reload would mix new presentation with an old worker. Development
instead suppresses fixture reloads and captures sources before loading gameplay
consumers. Explicit restart reloads the page with its route parameters intact.

## Visual standard

There was no approved mockup or image reference for this internal tool. The
standard was the user's searchable spreadsheet with expandable explained fields,
editable raw/computed values, readable errors and exact JSON preview. Desktop
and narrow actual `/mechanics` development-route captures were checked against those requirements,
with an independent critique and a same-fixture before/after pixel comparison.
Review images are scratch evidence rather than a visual design specification.

[The final choices ledger](choices.md) records the implementation conventions
left open by those contracts.
