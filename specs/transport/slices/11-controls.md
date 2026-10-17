# 11 · Controls: board, unload, cursor, marker, refusals

**Owns:** D7, D8 (input), D9 (refusal text), D21, D22, L8. **Depends on:** 07, 10.

Load `game-ui` first.

## Contract
- **Board by right-click.** Right-clicking your own unit is a new intent in `web/src/battle/input/pointerIntent.ts`. Right-clicking only handles buildings today (`:101`).
  - Squads selected, own carrier under the cursor: `Board`.
  - A carrier selected, own squad under the cursor: `Collect`.
  - Shift sets `queued`.
- **Board cursor.** A new generated `board` cursor in `web/src/battle/present/gameCursor.tsx:7-15`, shown while that intent is live.
- **Unload.** A new `unload` command mode, `KeyU`, in `web/src/battle/input/commandBindings.ts`; it is free, and the bindings test checks uniqueness.
  - It also gets a command-bar button (`readouts.tsx:663-738`), disabled unless the selection holds a carrier with passengers.
  - `commandReach.ts` gains `unload`, reaching carriers that hold passengers.
  - Behaviour:
    - press U, then left-click the ground: `Unload { goal: Some }`;
    - Shift-click queues it;
    - press U again while the mode is armed: `Unload { goal: None }`, sent at once;
    - right-click cancels, as other modes do.
  - No other command acts on a second press today. This is the first, and it is specific to Unload.
- **Unload marker.** The ordinary move line and destination mark, with a new unload icon at the destination. One component with different data, not a parallel marker.
- **Order wording** in `useUnitControl.ts:140-175` (tsc forces it to be complete).
- **Refusal text.** `REFUSAL_TEXT` in `rejectedOrder.tsx:17-27` gets `NO ROOM TO UNLOAD`, `NOT A CARRIER` and `ABOARD`. Per-unit refusals come from `CommandAck.refused`; refusals on arrival come from `ObservationFrame.refusals`, shown at the carrier.
- **Help.** `apps/battle-lab/src/Tutorial.tsx` lists the U key.
- **Army deck:** a passenger squad's deck card shows it is aboard (dimmed), and clicking it selects its carrier, matching the card stack. It isn't hidden, because the deck is the player's whole army.
- **Passengers can't be selected** (D10). Presentation doesn't draw them (slice 06), so picking never finds them, and a squad that boards drops out of the current selection. Box-selecting over a carrier selects only the carrier.
- **Leave building is unchanged:** button only, no hotkey (D8).

## What a human can see
The transport lab now runs through the production controls:
- board both ways, with Shift queues;
- U then a click;
- U twice;
- the refusals.

## Verification
- `unitControl` tests:
  - both board directions;
  - U then a click, U twice, Shift-queued unload;
  - the button disabled for an empty carrier;
  - a mixed selection: only eligible units get the order.
- The `commandBindings` uniqueness test.
- The cursor lab's pictures, re-blessed for the new cursor only.
- The transport scene's `controls` variant.
- **Visual gates:**
  1. `compare-screenshots` of the order line with its unload icon against the ordinary move line;
  2. an unprimed `screenshot-critique` last;
  3. the non-blocking `preview-shots` checkpoint.

## Delegated
Button label and icon drawing, as long as the button and icon stay within `game-ui`.

## Must stay green
Every existing hotkey, the cursor scenes, the move and order-line baselines, Leave building.

## Feedback that would change it
The cursor or the unload icon reading poorly at far zoom.
