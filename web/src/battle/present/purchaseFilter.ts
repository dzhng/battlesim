/** The reinforcements picker's filter: matching, buyability and the Enter
 *  choice, as pure data. */
import type { UnitCard } from "@packages/scene-assets/src/units";

/** What the battle observes about buying right now. */
export interface BuyFacts {
  credits: number;
  unitCount: number;
  maxUnits: number;
  finished: boolean;
}

/** On sale, affordable, a slot free, battle running: the one answer the
 *  family card and the variant card both read. */
export function cardPurchasable(card: UnitCard, facts: BuyFacts): boolean {
  return (
    !facts.finished &&
    card.disabled_reason === null &&
    card.cost <= facts.credits &&
    facts.unitCount < facts.maxUnits
  );
}

/** The cards a query matches, in catalog order: every whitespace token must
 *  appear in the family's name or the variant's. A blank query matches all. */
export function filterCards(cards: readonly UnitCard[], query: string): UnitCard[] {
  const tokens = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!tokens.length) return [...cards];
  return cards.filter((card) => {
    const text = `${card.roster.family_name} ${card.roster.variant}`.toLowerCase();
    return tokens.every((token) => text.includes(token));
  });
}

/** The card Enter buys: the sole matching family's first purchasable variant,
 *  or null with no family, several, or none purchasable. */
export function enterChoice(matches: readonly UnitCard[], facts: BuyFacts): UnitCard | null {
  if (new Set(matches.map((card) => card.family)).size !== 1) return null;
  return matches.find((card) => cardPurchasable(card, facts)) ?? null;
}
