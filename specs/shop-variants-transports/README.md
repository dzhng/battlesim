# Shop variants and transports: one card per variant, squads bought in a ride

**Status:** slice 2 merged; slice 1 in flight; slice 4's picker half in flight.
**Updated:** 2026-11-16.

The reinforcements shop stops grouping catalog variants into family tiles: every
variant is its own card, with today's card face unchanged. Hovering a squad
opens a transport menu of the carriers that seat it; picking one buys both, and
the squad arrives aboard. The AI uses the same combined purchase.

The [unknowns map](unknowns-map.html) is the source of truth for every
decision: its ledger rows (cited here as **M#**) and landmine cards are givens.
Throwaway mocks of the agreed look live in `throwaway/shop-mock/` (ignored, may
be gone); the map describes the look in words.

## Next Agent Prompt

Implement the slices below in graph order. Read, in order: this file, the
[unknowns map](unknowns-map.html), the [decisions ledger](decisions.md), then
the owning READMEs ([contract](../../crates/contract/README.md),
[sim](../../crates/sim/README.md), [web](../../web/README.md)). Before player-
visible work load [`game-ui`](../../.agents/skills/game-ui/SKILL.md); before a
mechanic, [`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md).

Pickup: **slices 1 and 2 in parallel** (no shared files), then 3 and 4 in
parallel, then 5 and closeout.

Warnings:

- Every map decision is settled. If the work shows one is wrong, stop, record
  the evidence in the ledger, and reslice; never settle a gap silently.
- Slice 1 must leave every existing digest, replay and parity fixture
  byte-identical (the new field is skipped when empty and hashed only when
  present). Slice 3 is the named digest change for AI battles.
- Tests pin mechanics, not roster data or class names (AGENTS.md).

## Contract

```
Order::ConfirmPurchase { variant, carrier: Option<String>, destination }   // carrier skipped when None
OpeningPurchase, PendingPurchase, PurchasePreview, Reservation             + carrier: Option<…>
publication pending row, reader, wasm preview_purchase, web protocol       + carrier
```

- **Admission** refuses a pair unless the carrier seats the whole squad, the
  unit cap has room for two units, and credits cover both prices.
- **Arrival:** one reservation, one dispatch, one tick. The carrier spawns with
  its move order to the destination; the squad spawns inside it (hold Inside,
  positions copied) with no order. On arrival the squad stays aboard (M13).
- **Money:** each unit records its own card price; cancelling a pending pair
  refunds the sum; on the map refunds stay per unit (M14).
- **Count:** a pending pair is two units wherever units are counted, the AI's
  slots included.

## Slices

```
1 contract + sim ──┬── 3 AI combined purchase ──┐
                   └── 4 transport menu (web) ──┼── 5 tutorial, scenes, closeout
2 flat shop + Armory + cap deletion (web/contract catalog) ──┘
```

### 1. The combined purchase (contract, sim, wasm, web protocol)

Contract field and admission, reservation with carrier, same-tick spawn aboard
in dispatch, unit-cap counting, per-unit price records, publication row and
reader, wasm `preview_purchase` taking the optional carrier, the web protocol
and observation view carrying it. No UI.

Done when: a sim end-to-end test buys a squad in a carrier and sees it arrive
aboard at the destination, counted as two units, refunded per unit; a pair
refused for seats, cap or credits; every existing digest, replay and parity
fixture unchanged.

### 2. Flat shop and Armory (web, contract catalog)

One card per variant in the picker (M1–M6): click places that variant, hover
shows its single info card, order by family then price, each card under its
own `unitGroup` tab, labels are `card.name`, anchor keyed by card id, hover
delay only while a transport menu is open (none exist yet). Search Enter buys
when exactly one card matches. Delete `familyGroups` and the family rule in
`enterChoice`. Armory: one tile per variant showing `card.name`; chips and the
"N VAR" tag go (M18). Delete `FAMILIES_PER_CATEGORY` / `check_families` (M19).
Rewrite the picker and Armory tests by behaviour; update the scenes that find
cards by family name (`ui`, `unit-roster`, `_appJourney`, `armory`, `allies`,
`_contact-battle`) and re-approve their baselines after inspecting them.

Done when: picker and Armory tests and the touched scenes pass; screenshots
judged with screenshot-critique.

### 3. AI buys a squad with its ride (sim)

Depends on 1. Where the AI would buy a squad and a carrier for it (the
opening's squad-and-ride pairs, a new squad short of a carrier), it sends one
combined purchase (M17). Its pending bookkeeping counts the carrier; its
scripted-plan cursor advances past both cards. Boarding stays for re-pairing
after a carrier is lost.

Done when: AI tests pin the combined purchase and no double-buying; a
before/after skirmish tournament shows no regression in squads reaching
objectives; the digest change is named in the ledger.

### 4. Transport menu (web)

Depends on 1 and 2. Hovering any squad card (squad body, INF and REC alike,
M7) opens a menu above it: an "On foot" card, then carriers whose seats fit the
squad (M8), sectioned by shop tab and cheapest first (M9), all today's card
face. Nothing highlighted at first (M10). Hovering On foot shows the squad's
info card; hovering a carrier stacks the carrier's card, the squad's marked
aboard, and a total, above the whole menu aligned to the hovered card (M11,
M12). Clicking a carrier starts placing the pair; affordability checks both
prices and two cap slots. Keyboard (M16): one matching squad opens its menu,
Up/Down step through it, Enter buys the highlighted option or on foot.
Placement state holds squad plus optional carrier; ghost and loaded art are the
carrier's; Shift-click repeats the pair. The army deck's pending squad card
carries the aboard mark.

Done when: picker tests cover hover, keyboard and the pair order; a scene buys
a Rifle Squad in a carrier and shows it arriving aboard; screenshot-critique
and compare-screenshots on the menu.

### 5. Tutorial, scenes, closeout

Depends on 2–4. Rewrite the tutorial's shop text and re-shoot "buy"; finish
any scene or baseline the earlier slices left; run the whole check ladder
once; whole-spec review; consolidate the ledger; close the spec.

## Checklist

- [ ] 1 The combined purchase
- [x] 2 Flat shop and Armory (`unit-roster` has four world-render baseline diffs unrelated to the HUD: owned by closeout)
- [ ] 3 AI buys a squad with its ride
- [ ] 4 Transport menu
- [ ] 5 Tutorial, scenes, closeout
