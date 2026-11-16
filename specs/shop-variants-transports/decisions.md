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
