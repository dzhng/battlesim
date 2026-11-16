# Decisions ledger: shop variants and transports

Decisions made where the spec and the [unknowns map](unknowns-map.html) were
silent, appended per pass with a verdict. The map's own ledger rows are givens
and are not repeated here.

## Slice 2: flat shop and Armory

- **The info card is a pass-through tooltip, not a button.** It now covers cards above the hovered one, so a clickable card would block them; clicking the card itself buys. *Sound.*
- **A card that can't be bought stays hoverable (`aria-disabled`) and refuses the click.** The player can still read what it is and why not. *Sound.*
- **The Armory orders cards with the shop's `shopOrder`.** One order for both views. *Sound.*
- **Search scenes search one exact variant** ("abrams sep v2 trophy"); other scenes click a card by its full name. Follows M6/M9 of the map. *Sound.*
- **The UI gallery's never-sold card borrows a planned card's id** for its silhouette, as the Armory gallery already does. Test fixture only. *Sound.*
- **`*-mbt-variants` baselines retired, replaced by `*-mbt-info`.** The pictures now show one info card. *Sound.*
- **Shop cards renamed `.hud-purchase-card(s)`, shared with Armory tiles.** One class for one face. *Sound.*
- **Checked:** no shipped family splits across tabs under per-card `unitGroup` (152 cards, 108 families). Names are unique within a faction, not across factions. *Fact, confirms M6.*

## Slice 1: the combined purchase

- **Pair refusals reuse existing errors:** a carrier without seats or a passenger that isn't a squad is `NotACarrier`; too few seats is `CapacityFull`. No new contract error for one more refusal reason. *Sound.*
- **Seats are checked in `preview_purchase`,** which admission runs, so the placement ghost refuses a pair the order would refuse. *Sound.*
- **A pair is placed as its carrier:** entry choice and snapping use the carrier; a pair's preview publishes no squad spots or area. *Sound.*
- **`Reservation` keeps `price` (the squad's) with an optional `carrier_price` beside it;** dispatch and cancel read those, never the catalog again. *Sound.*
- **The pending publication row's `carrier` is a type index, −1 for none** (11 → 12 fields). The layout fixtures were hand-patched for the two layout edits, because the captures' recorded digests are stale against staging and can't be re-simulated here. *Sound, but the stale captures are pre-existing debt (see closeout).*
- **`Order::purchase(variant, destination)` constructor** for the no-carrier order, so call sites over the size budget didn't grow. *Sound.*
- **Size-budget splits:** the pending row writer (`publication/pending_row.rs`), lane purchase destination (`skirmish_ai/purchase.rs`) and purchase admission moved out of their files by concept. *Sound.*
- **Merge with staging:** `fog-sight-vectors.json` conflicted (staging re-recorded it); resolved by applying slice 1's two layout edits (pending fields + copy alignment) to staging's version. *Sound.*
