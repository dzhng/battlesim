# Mechanics editor

The local authoring tool runs inside the game's development server. Open
`/mechanics` through the developer menu after running the root development command.
It edits authored fixtures, keeping the simulation's Rust resolver as the
authority for inheritance and valid gameplay values.

The development command builds the native validator once. After changing Rust
validation code, rebuild it with `bun run build:mechanics`; fixture edits are
validated by passing current JSON to that executable, without recompilation.

The spreadsheet finds concrete units; expanded rows expose explained fields,
their inheritance source, local soldier overrides and globally shared weapons.
Raw and battlefield values are both editable. Landing spread describes one-axis
standard deviation at maximum range, so changing range preserves that spread.

Preview shows exact JSON replacements. Save rejects stale drafts and publishes
authored sources with their matching generated catalog. Interrupted publication
is recovered before the next reader, while outside edits are preserved. File
replacement is atomic individually; the journal provides recovery across files.

Each battle page captures one accepted generation. Saving leaves running battles
alone; starting or explicitly restarting a battle loads the latest saved values.
The development-only API accepts local, same-origin writes and never takes file
paths from the browser.

The [server](server.ts) owns file publication and the
[field descriptions](src/fields.ts) own labels, units and reversible conversions.
The [feature spec](../../specs/mechanics-editor/README.md) records remaining proof.
