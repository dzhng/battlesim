# Decisions ledger: shop variants and transports

The decisions made while building this, where the plan and the
[unknowns map](unknowns-map.html) were silent, checked against the shipped
code. The map's own ledger (rows 1–20, cited in code as M1–M20) holds what
was agreed before the build;
it is not repeated here. Least confident first within each group.

## Provisional: a guess worth revisiting

- **An open transport menu reads the pointer's heading, and holds up to
  300 ms.** A pointer crossing another card while heading into the menu (its
  latest position inside the triangle from a few moves back to the menu's
  bottom corners, the way mega-menus aim) keeps the menu until it stops there
  or turns away; any other move switches at once, so sweeping along the
  infantry row never waits. The 300 ms only bounds a pointer that stops
  part-way; it sits among desktop submenu delays (about 225–400 ms) and was
  never tuned by feel (`MENU_AIM_MS`, `purchasePicker.tsx`).

## Measured: chosen on evidence

- **An AI squad whose ride it can't yet afford waits and buys both together.**
  Tried against buying the squad alone, on the same seeds: waiting took the
  first objective sooner and kept objectives owned as fast as before; buying
  alone was 10 s slower and left most reinforcement squads walking. In its own
  shopping, three cases still buy the squad alone, because waiting would be
  wrong there: an air-defence squad (aircraft won't wait), the opening (its
  credits are fixed for that one decision), and a last free unit slot (or the
  AI could never fill it); a scripted plan also falls back to the squad alone
  (below) (`skirmish_ai/purchase.rs`).
- **The AI change moves the digest of every battle where the AI shops.** That is
  the named, deliberate change: 32 tournament and corpus battles, e.g.
  `self_play_9` `ea0d3d022c13c8b8 → 3e9f5e6f2336b78f`. No checked-in fixture
  replays an AI that shops, so none was re-recorded. On 23 battles, before →
  after: first objective captured 137.6 s → 124.1 s on average (sooner in 19);
  time to first own each objective unchanged (231.0 s → 230.9 s); orders per
  battle second 2.62 → 2.45.

## Sound: the obvious call, recorded so nobody re-asks

Shop and menu

- **The info card is a picture, not a button.** It now overlaps the cards above
  the hovered one, so it lets the pointer through; clicking the card itself
  buys (`purchaseCard.tsx` `PurchaseInfo`).
- **A card you can't buy still shows what it is.** It stays hoverable and
  refuses the click, so the player can read why (`ShopCard`).
- **A squad that is never sold shows its plain info card, no menu;** a squad you
  can't afford still opens its menu with every option dimmed.
- **A ride's "+N CR" turns red when squad plus ride exceed your credits,** since
  that is the sum that decides.
- **"Aboard" is drawn the way the game already draws a rider:** the seat mark
  before the squad's name, and the carrier's info counting the squad in its
  seats (full). Writing the word "ABOARD" next to a carrier reading
  SEATS 0/8 contradicted itself.
- **The total reads carrier + squad,** top to bottom like the two cards above it.
- **Up/Down wrap round the menu:** from nothing highlighted, Down goes to On foot
  and Up to the last ride. The shop takes the arrows from the camera only while
  a menu is open; Enter buys only from the search field.
- **The highlight clears whenever the pointer moves to another card or leaves
  the shop,** so a menu always reopens with nothing highlighted (`point`).
- **Shift-click repeats a pair without checking credits in the browser.** The
  simulation refuses a pair it can't take, as it does a single unit; one
  authority.
- **The Armory orders cards the way the shop does** (`shopOrder`).

Owners

- **One owner each:** a unit's seats and a squad's size (`UnitCatalog.seats`,
  `squadSize`); a pair's cost and whether it can be bought (`pairCost`,
  `pairPurchasable`); the card face, price and info card (`purchaseCard.tsx`);
  the ghosts of a purchase in hand and pending ones (`shopGhosts.ts`); in the
  simulation, a purchase's unit count and total (`Purchase::units`, `total`)
  and "this carrier seats the whole squad" (`carriage::seats_squad`, asked by
  purchase admission and the AI's pairing alike).
- **`Order::purchase(variant, destination)` builds a purchase with no carrier,**
  so call sites already at the file-size limit didn't grow.

Simulation contract

- **Two existing refusals cover a bad pair:** a carrier with no seats, or a
  passenger that isn't a squad, is `NotACarrier`; too few seats is
  `CapacityFull`. No new error for one more reason.
- **Seats are checked in the purchase preview,** which confirming also runs, so
  the placement ghost refuses exactly the pairs the order would.
- **A pair is placed as its carrier:** the entry road and the snapped spot are the
  carrier's, and the preview shows no soldier spots, since the squad stands
  nowhere while aboard.
- **The reservation keeps each unit's price** (`price`, `carrier_price`), so
  arrival and cancel never read the catalog again. The carrier's kind lives
  beside it in the published row; the digest hashes the carrier only when
  there is one, so battles without pairs keep their digests.
- **The published pending row's `carrier` is a type index, −1 for none** (11 →
  12 fields).
- **AI scripted plans pair too:** a squad card followed by a card that seats it
  is one purchase, and the plan's cursor moves past both
  (`purchase_result(accepted, cards)`); a pair it can't afford falls back to the
  squad alone.

Tests and pictures

- **The pair is proven end to end in `unit-roster`,** the journey that already
  buys through the real shop: a Rifle Squad in a Stryker ICV is placed up the
  road (the scene checks it lands more than 20 m from the entry), the carrier
  drives there, and the squad is still aboard 120 ticks after it stops.
- **The tutorial's "buy" picture shows the transport menu,** the new thing to
  learn.
- **Old baselines retired by name:** `*-mbt-variants` became `*-mbt-info`; the UI
  gallery's never-sold card borrows a planned card's id for its silhouette, as
  the Armory gallery does.
- **Four `unit-roster` world pictures were re-approved:** they failed on the
  unchanged scene too, because staging's map commits (homesteads, round road
  corners, a straight entry road) moved that seed's map after their last
  approval.
- **The publication captures were re-recorded from the merged code**
  (`BLESS_PARITY`); against staging they differ only by the carrier column, the
  capture hash that follows from it, and the 2v2 golden's pending pair.
