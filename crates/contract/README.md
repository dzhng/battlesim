# Shared contracts

The contract crate owns the vocabulary that native simulation, WebAssembly and
fixture readers must agree on. It carries data shapes and admission, not battle
execution. [The crate boundary](src/lib.rs) names current modules; a consumer must
use those owners rather than recreating a competing resolver or numeric policy.

Coordinates use ground XY with +Z up, metres, seconds and radians. Explicit
component units, such as authored vehicle speeds, follow their own schema.
[Commands](src/command.rs) address authoritative units or side-scoped target
handles. [Observations](src/observation.rs) contain admitted side evidence; renderer
knowledge cannot expand that vocabulary with hidden state.

[Map admission](src/maps.rs), [physical templates](src/templates.rs) and
[catalog resolution](src/catalog.rs) validate inputs before consumers build a
world. Templates describe physics, not meshes, and every admitted template is a
whole building: its floors, an entrance and a bay lattice on each exposed facade,
never a gap a consumer fills in. [scene-assets](../../packages/scene-assets/README.md)
dresses them without changing the map's physical identity. [Fixtures](../../fixtures/README.md)
owns authored documents and resolved catalog publication.

The catalog separates player card identity from physical admission. Planned
variants retain faction memberships, price and a disabled reason without acquiring
a live type index. Admitted variants supply validated physics and carried mounts;
developer scenario types may have no player membership. Shared equipment has one
canonical variant and several memberships, so factions do not duplicate tuning.

What a card shows is authored beside what code addresses: a unit, family, mount,
weapon row, active protection or planned weapon carries a player-facing label
apart from its id, and every weapon a card shows names its generated icon.
[Labels](src/labels.rs) owns the rule that makes a label concise; resolution
refuses a label that reads as an identifier or a sentence, or an icon that is not
an icon id, naming the entry. No reader derives a label from an id.

[Identity](src/identity.rs) defines canonical content and exact generation seeds.
Its decimal-text seed contract must survive JSON without JavaScript number
rounding. Typed serialization order and meaningful authored sequence order can
change identity even when an unordered JSON comparison looks equal. [Generation
compilation](../mapgen/README.md#compilation) explains how physical identity,
execution allowance and source receipts differ.

[Tests](tests/) pin admission, materialization and deterministic shared arithmetic.
[The template report](examples/template_report.rs) inspects a supplied descriptor
and placement frame without constructing a battle or loading art. Its source
usage owns arguments; an admitted descriptor alone is not a rendered-fit proof.
