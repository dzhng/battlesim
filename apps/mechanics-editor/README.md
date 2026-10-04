# Mechanics editor

Run from the repository root:

```bash
bun run setup          # first checkout: install web dependencies
bun run dev:mechanics  # build, start localhost and open /mechanics
```

Use the localhost address printed by Vite. The editor opens at `/mechanics`
(default: <http://localhost:5173/mechanics>). If that port is occupied, stop the
other server or run `bun run dev:mechanics --port 5175`. The normal `bun run dev`
also serves the editor; open it through the developer menu or visit `/mechanics`.
Prerequisites are in the root [Running it](../../README.md#running-it) section.

1. Search for the exact unit, then edit its spreadsheet cells or expand its row.
   Each field explains its effect and units. Shared weapon edits affect every
   listed user; unit and soldier overrides apply to the selected authored type
   and descendants that inherit it.
2. Edit either the battlefield value or its raw stored value; its partner updates
   automatically. Use **Undo edit** to cancel a draft change, or **Restore inherited
   value** where offered to remove an authored override.
3. Click **Preview changes**. Fix any errors, then review before-and-after values,
   affected units and the exact JSON replacements.
4. Click **Save authored JSON**. This replaces fixture files in this checkout and
   updates their generated catalog; Git shows the changes. Start or explicitly
   restart a battle to use the saved values. A running battle keeps its settings.

The editor uses the simulation's Rust resolver as the authority for inheritance
and valid gameplay values. If files change elsewhere, your draft is retained;
**Reload sources**, then **Discard draft and reload**, explicitly replaces it
with current files.

The development command builds the native validator once. After changing Rust
validation code, rebuild it with `bun run build:mechanics`; fixture edits are
validated by passing current JSON to that executable, without recompilation.

The spreadsheet finds concrete units; expanded rows expose explained fields,
their inheritance source, local soldier overrides and globally shared weapons.
Unfinished input stays with its field through filtering. Replacing or restoring a
parent discards that subtree's edits; removing a soldier kind's last slot also
discards its edits scoped to that unit.
Raw and battlefield values are both editable. Landing spread describes one-axis
standard deviation at maximum range, so changing range preserves that spread.

Preview shows exact JSON replacements; Rust supplies the generated catalog's
canonical bytes. [Shared fixture publication](../fixture-publication/README.md)
owns stale-save rejection, interrupted-write recovery and outside-edit preservation.
The editor formats authored JSON. The local development API accepts logical edits
rather than browser-selected paths.

The [server](server.ts) owns file publication and the
[field descriptions](src/fields.ts) own labels, units and reversible conversions.
The [feature rationale](../../specs/done/mechanics-editor/README.md) records the
authoring and publication principles.
