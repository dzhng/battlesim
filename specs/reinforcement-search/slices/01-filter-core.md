# Slice 1: Filter core

## Contract this unlocks

The picker can ask, without any DOM: which cards match this query, can this
card be bought now, and which card does Enter choose. Every later slice feeds
the real `UnitCard` list through this seam.

## API seam

New `web/src/battle/present/purchaseFilter.ts` (pure, no React):

```ts
export interface BuyFacts {
  credits: number;
  occupiedSlots: number;
  maxUnits: number;
  finished: boolean;
}

/** The one owner of "can this card be bought now". */
export function cardPurchasable(card: UnitCard, facts: BuyFacts): boolean;

/** Cards whose family name or variant matches every whitespace token of
 *  the query. Whitespace/empty query returns every card, same order. */
export function filterCards(cards: readonly UnitCard[], query: string): UnitCard[];

/** The card Enter buys: the first purchasable variant of the sole matching
 *  family, or null when no family, more than one family, or none of the
 *  sole family's matches is purchasable. */
export function enterChoice(matches: readonly UnitCard[], facts: BuyFacts): UnitCard | null;
```

Grouping key is `UnitCard.family` (what `PurchaseMenu` already groups by).
Card order is preserved, so "first available" means the first purchasable
match in catalog order.

## What the human can run

Nothing visual yet. The proof is the test file.

## Verification

`web/tests/purchaseFilter.test.ts`, built from test-unit fakes only (never the
roster). Cover:

- `filterCards`: family-name match, variant-name match, multi-token AND,
  case-insensitivity, no match → empty, empty/whitespace query → all cards in
  order.
- `cardPurchasable`: disabled reason, credits below cost, no free slot,
  finished, and each boundary equal to the threshold.
- `enterChoice`: one family whose first variant is available → that card; one
  family whose first purchasable is a later variant → that later card; two
  families → null; one family with no purchasable variant → null; empty
  matches → null.

Write one test at a time and watch it fail before making it pass. Falsify the
`enterChoice` "one family" guard once by removing it and confirming red.

## Must stay green

Nothing else runs yet. Keep the module free of imports from React or the DOM
so it stays a pure unit.

## Delegated

- Token splitting/normalization internals (split on whitespace, lowercase,
  trim).
- Whether `cardPurchasable` composes helper predicates internally.

## Feedback that would change this slice

If "first available" should mean the first purchasable across *all* matched
families rather than only when one family remains, the README's frozen decision
and this contract change first.
