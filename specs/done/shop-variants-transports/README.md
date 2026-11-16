# Shop variants and transports

The reinforcements shop shows every catalog variant as its own card, and a
squad can be bought together with the carrier it rides in: the pair arrives as
one unit of reinforcement with the squad already aboard. The AI buys its
squads the same way.

The [unknowns map](unknowns-map.html) records what was agreed before the build
(its decision ledger, rows 1–20, cited in code as M1–M20) and the landmines found in the code; the
[decisions ledger](decisions.md) records what was decided while building.

## Why

Grouping variants into family tiles hid the real choice. A tile showed the
family's cheapest price and one silhouette; the variants lived in a hover
fan-out, and clicking the tile bought "the first affordable variant", a choice
the player never saw. Flat cards make every purchase a visible, single click.

The hover-to-choose interaction was worth keeping where there is a genuine
second choice: how a squad gets to the fight. Before this, a player who wanted
infantry in a Stryker bought two units, waited for both to drive in, then
ordered the boarding. Now the ride is part of buying the squad, and the AI's
own boarding dance (with its history of carriers swapped back and forth so
nobody boarded) is skipped for every new squad.

## What the player sees

- **One card per variant, today's card face:** role icon, silhouette, price, no
  name. Siblings stand together (family, then cheapest first), and each card
  sits under its own shop tab. Look-alike variants are told apart by hovering:
  the card's info panel shows its weapons and price.
- **A squad's transport menu:** hovering the card of any squad on sale opens a
  menu of "On foot" and every carrier on sale that seats the whole squad, sectioned by shop
  tab, cheapest first. Hovering the base card highlights On foot and shows the
  squad's info card. On foot uses a boot icon without a price, leaving the squad's
  cost in its info card. Pointing at a carrier stacks its info card above the
  squad's and adds the total; returning to the base card previews On foot again.
  Clicking the squad's card, or On foot, buys it alone.
- **The keyboard path:** a search narrowed to one squad opens its menu; Up and
  Down step through it; Enter buys the highlighted option, or on foot.
- **On arrival** the carrier drives to the placed spot and the squad stays
  aboard until the player unloads it. The army deck shows a pending pair as two
  cards, the squad marked aboard as a rider on the map is.
- **The Armory** shows one tile per variant, named, in the shop's order.

## Principles and invariants

- **A pair is one purchase of two units.** One order, one reservation, one
  dispatch, one tick: the carrier spawns with its move order and the squad
  spawns inside it. It holds two places under the unit cap and costs both
  prices; cancelling refunds the sum; once fielded each unit keeps its own
  price for refunds, bounties and settlement.
- **Who can ride is physical.** A carrier qualifies when its seats hold every
  soldier of the squad; a squad is any unit with a squad body, whatever its
  category. No role or label decides it, on either side of the boundary.
- **Admission and the ghost agree.** The purchase preview checks seats, and
  confirming runs the preview, so the placement ghost refuses exactly what the
  order would.
- **Battles without pairs are untouched.** The carrier field is left out of
  every serialized order and record when empty, and hashed into the digest only
  when present. The AI's use of pairs is the one named digest change.
- **The simulation is the only authority on affordability.** The browser
  dims what it believes can't be bought; the simulation refuses what can't.
- **The catalog's family stays data.** `roster.family_name` still names cards,
  feeds search, and groups siblings in the shop's order; nothing groups cards
  into tiles any more, and the catalog no longer caps families per category.

## Where it lives

- Contract: `Order::ConfirmPurchase { variant, carrier, destination }` and
  `Order::purchase` (`crates/contract/src/command.rs`); `PendingPurchase`,
  `PurchasePreview` with `arriving()`, `kinds()`, `units()`
  (`crates/contract/src/skirmish.rs`); `OpeningPurchase`
  (`crates/contract/src/players.rs`).
- Simulation: `Purchase`, `Reservation`, `Skirmish::reserve_at`
  (`crates/sim/src/skirmish.rs`); same-tick spawn aboard in
  `crates/sim/src/battle/dispatch.rs`; seats checked by `preview_purchase`
  (`crates/sim/src/battle/purchase_preview.rs`) through `carriage::seats_squad`,
  the one rule the AI's pairing also asks;
  the published pending row in `crates/sim/src/publication/pending_row.rs`.
- AI: `crates/sim/src/skirmish_ai/purchase.rs` (pairing, saving for a ride,
  scripted plans) and `transport::carrier_for`.
- Browser: `purchasePicker.tsx`, `transportMenu.tsx`, `purchaseCard.tsx`,
  `purchaseFilter.ts` (`shopOrder`, `pairCost`, `pairPurchasable`,
  `carrierSections`) under `web/src/battle/present/`; placement in
  `web/src/battle/input/purchasePlacement.ts`; the deck's pending pair in
  `armyDeck.tsx`; ghosts in `apps/battle-lab/src/shopGhosts.ts`;
  `UnitCatalog.seats` and `squadSize` in `packages/scene-assets/src/units.ts`.
- Tests: `crates/sim/tests/combined_purchase.rs` (admission, arrival aboard,
  money, the AI's pair through the real host),
  `crates/sim/src/skirmish_ai/tests/ride_purchases.rs`,
  `web/tests/purchasePicker.test.tsx`, `purchasePlacement.test.ts`,
  `armyDeck.test.tsx`; the `unit-roster` scene buys a squad in a carrier and
  sees it arrive aboard; the `ui` scene's `transport-menu` picture.

## Dead ends

- **Names on the cards.** Two-line names (family, then variant) were mocked on
  the real 64 × 88 px card. They truncated at that size, and the existing cards
  carry no text; the player chose the unchanged face and hover for detail.
- **Two linked purchases for a pair.** Each entry road fields one unit per
  dispatch interval, so two linked purchases would arrive seconds apart,
  perhaps with another player's unit between them; one reservation carrying
  both avoids it.
- **The AI buying a squad alone when it can't afford the ride.** Measured
  slower to take objectives than saving for both (ledger).

## Visual provenance

The agreed look came from throwaway mocks built on the game's own stylesheet
during the unknowns walk; the map describes them in words. The shipped look is
pinned by the `ui` scene's `transport-menu` baseline
(`web/scenes/baselines/ui/transport-menu.png`), the `unit-roster` scene's
`journey-transport-arrived` baseline, and the tutorial's
`assets/runtime/tutorial-media/buy.jpg`.
