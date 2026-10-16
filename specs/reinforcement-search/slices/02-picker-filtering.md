# Slice 2: Picker filtering

## Contract this unlocks

With the panel open, typing narrows the family grid across the whole faction;
the field is focused on open and sits at the right end of the tab row.

## API seam

Edit `web/src/battle/present/purchasePicker.tsx`:

- `PurchaseMenu` gains `query` state and a `searchRef`.
- The footer becomes `.hud-purchase-filter`: the existing
  `.hud-purchase-categories` `role="tablist"` (tabs only) plus the search
  `<input>` as a sibling.
- The `families` map is built from `filterCards(available, query)` when the
  query is non-empty (ignoring the category filter) and from the category
  filter otherwise. The flyout lists the family's matching variants while
  filtering.
- Family and tab buttons `onMouseDown` prevent default so pointer use does not
  move focus out of the field.
- The input autofocuses on mount (the menu already unmounts on close, so each
  open is fresh).

Edit `web/src/hud.css`: `.hud-purchase-filter` flex row; a
`.hud-purchase-search` field on the existing `.hud-panel input[type="text"]`
no-browser-controls rules, with the magnifier icon at its start.

The field markup:

```tsx
<label className="hud-purchase-search">
  <Icon path={hudIcon("search")} />
  <input
    ref={searchRef}
    type="text"
    aria-label="Search units"
    placeholder="SEARCH"
    autoComplete="off"
    spellCheck={false}
    value={query}
    onChange={(e) => setQuery(e.target.value)}
  />
</label>
```

## What the human can run

`?shot=purchase` in the lab; type into the field and watch the grid narrow.
`web/scenes/ui.mjs` types a query and captures the state.

## Verification

- `web/tests/purchasePicker.test.tsx`: typing a family name narrows to that
  family; typing a variant name surfaces its family; a match in another
  category appears without switching tabs; no match shows the `NO MATCH`
  state; clearing restores the remembered category; the tabs dim while
  filtering.
- Extend `web/scenes/ui.mjs`'s `picker()` to type a query after the existing
  checks, capture `purchase-search`, and keep the existing card-size,
  flyout-geometry and opacity checks passing.
- `screenshot-critique` unprimed on the captured search shot as the last check,
  and `compare-screenshots` against `purchase-veh` to judge the added footer
  row's density.

## Must stay green

- `purchasePicker.test.tsx`'s existing eight tests, including "opens fresh".
- `ui.mjs`'s existing `purchase-*` baselines keep matching except for the
  intended footer row, which is re-approved deliberately.
- `unit-roster.mjs`'s picker flow.

## Delegated

- Exact `NO MATCH` / count wording and whether it is a separate line or the
  field's empty state.
- Whether the filtered capture is a new `SHOTS` entry or a sub-baseline inside
  the existing `purchase` routine.
- The field's width within the tab row.

## Feedback that would change this slice

If the grid should keep the category scoping during a search, the README's
"spans all categories" contract changes first.
