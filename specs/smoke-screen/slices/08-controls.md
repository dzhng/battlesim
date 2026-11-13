# 08: player controls and panel

## Contract

- **Command bar:** a player pops smoke with POP SMOKE, placed next to REVERSE,
  or with key **H** (D15).
- **Selection:** every selected unit with a charge pops one.
- **Panel:** the floating panel shows `SMOKE n`, in the warning colour at 0
  (D13). HIDDEN lights up while the unit is in smoke (D14).

## Seam

- **Order mirror:** add `{ kind: "pop_smoke"; units: number[] }` to the
  hand-written `Order` union in `web/src/battle/sim/protocol.ts`.
- **`useUnitControl`:** a `popSmoke` action, plus the exhaustive `describe`
  switch.
- **No flash:** add `orderReveal.ts` to the no-flash cases. A pop is not a
  movement order.
- **Key:** `commandBindings.ts` gets `pop_smoke: { code: "KeyH", label: "H" }`.
  Update `web/tests/commandBindings.test.ts` if it lists the commands.
- **Button:** in `commandBar.tsx`, add a POP SMOKE command right after REVERSE.
  It is disabled when no selected unit has a charge.
- **Panel row:** `panelRows.ts` gets a `smoke` row (`icon: "smoke"`, word
  `SMOKE n`, `lasting`, `amount`), built exactly like `flares`. Add a
  `panelSpecimens` entry, or `web/tests/panelWorkbench.test.ts` fails.
- **Icons:** add `smoke` to `STATE_ICONS` and a `pop_smoke` HUD icon in
  `packages/scene-assets/src/icons.ts`, generated with `asset icons`. The asset
  check refuses stale or orphan icons.
- **HIDDEN:** nothing new. It reads `concealed`, which slice 04 feeds.

## What you can see

The smoke lab, using real input: select, press H, watch the charge count down,
see HIDDEN light up, and see the cloud.

## Tests and checks

- A web test: pressing H sends one `pop_smoke` naming exactly the selected
  units that have a charge, and doesn't change their current order.
- The panel-workbench and command-binding tests.
- **Visual checks:** load `game-ui` first. Shoot the panel row in every state
  (n > 0, 0, with HIDDEN, without HIDDEN) and the command bar with mixed
  selections.
  - Judge against the mock with `compare-screenshots`.
  - Run an unprimed `screenshot-critique` as the **last** check.
  - Show the user with `preview-shots` (non-blocking, about 5 minutes).

**This slice's one visual variable:** the UI surfaces. The cloud and the fog are
frozen.

## Delegated

You may choose the icon's drawing, within the generated icon set's style, and
the tooltip wording.

## Must stay green

The web tests, `asset check`, and every existing panel and command test.

## Feedback that would change this slice

The user may prefer another key. If so, change the binding only.
