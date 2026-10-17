# Slice 09: order-of-battle lobby screen

**Decisions:** D17, D22, D25. **Needs:** 06, 07 and 08.

**Visual variable:** lobby composition and how clearly it shows state. That covers:
- the balance between the columns;
- the banner hierarchy;
- whether YOU stands out;
- disabled sizes and their reasons;
- preview states;
- Deploy being obvious.

**Crop:** the full lobby. The map's own legibility was judged in slice 08 and is out of scope here.

## Contract it unlocks

The player can set up any match and deploy it.

## Seam

`apps/battle-lab/src/MainMenu.tsx` (the `NewBattle` page) renders the slice 07 model and the slice 08 picture, laid out as Direction 3:
- Blue banners on the left and Red on the right. Each has a big index numeral, a controller (YOU/AI) and a delete ×.
- Each side has a faction header and "+ add player".
- The map sits between the banner stacks, under "N VS M".
- A bottom strip holds presets, map type, size (S · M · L · XL), region, seed status, Reroll and Deploy.
- Deploy is disabled until the preview is ready, and goes to the admitted request's link.

## What the human can run or see

The lobby at `/`, plus a lobby browser scene covering every "after 09" item in the README checkpoints.

## Visual gates

1. Use `game-ui` first.
2. `compare-screenshots` against Direction 3 in [`../visualizations/lobby-directions.html`](../visualizations/lobby-directions.html), and against the previous `menu-skirmish` baseline.
3. `screenshot-critique`, unprimed, as the last check before acceptance.
4. A non-blocking `preview-shots` review: give the user about 5 minutes. If they're silent, decide on the evidence, record why in `choices.md`, close the shots and continue.

## Delegated

Exact spacing and type scale; banner shape details; and the treatment of failed and retrying states, all within `game-ui`.

## Stays green

The other menu pages and the backdrop reel.

## Feedback that would change it

The user's reaction to the shots.
