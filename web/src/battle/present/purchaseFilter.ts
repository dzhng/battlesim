/** The reinforcements picker's rules, as pure data: the groups it browses
 *  by and the order cards stand in, matching and the Enter choice, what can
 *  be bought (a card, or a squad with its carrier, and what the pair costs),
 *  and the carriers that can take a squad. */
import type { UnitCard, UnitCategory, UnitType } from "@packages/scene-assets/src/units";

/** A browsing group: one category's cards, narrowed by role. */
export type UnitGroup = "rec" | "inf" | "mbt" | "ifv_apc" | "arty" | "aa" | "log" | "hel" | "air";

/** The groups the picker and the Armory browse by, in their tab order, each
 *  with its tab label. Presentation only: the card's category stays the game
 *  rule (who holds objectives, what the AI buys), and each group holds one
 *  category's cards. A group holds its category's types with one of its
 *  `roles`; `null` holds the rest of its category. */
export const UNIT_GROUPS: readonly {
  id: UnitGroup;
  label: string;
  category: UnitCategory;
  roles: readonly string[] | null;
}[] = [
  { id: "rec", label: "REC", category: "rec", roles: null },
  { id: "inf", label: "INF", category: "inf", roles: null },
  { id: "mbt", label: "MBT", category: "veh", roles: ["mbt"] },
  { id: "ifv_apc", label: "IFV·APC", category: "veh", roles: null },
  { id: "arty", label: "ARTY", category: "sup", roles: ["artillery"] },
  { id: "aa", label: "AA", category: "sup", roles: ["air_defense"] },
  { id: "log", label: "LOG", category: "sup", roles: ["logistics"] },
  { id: "hel", label: "HELI", category: "hel", roles: null },
  { id: "air", label: "JETS", category: "air", roles: null },
];

/** The group a card browses under: the first of its category's groups that
 *  holds its type's roles. A card with no type yet (a planned one), or a
 *  role none holds, lands in its category's first group. */
export function unitGroup(card: UnitCard, unit: Pick<UnitType, "roles"> | null): UnitGroup {
  const own = UNIT_GROUPS.filter((g) => g.category === card.roster.category);
  const held = unit && own.find((g) => g.roles?.some((r) => unit.roles.includes(r)) ?? true);
  return (held || own[0]).id;
}

/** `cards` in shop order: siblings together, each family where its first
 *  card stands, then cheapest first within it (catalog order breaks ties). */
export function shopOrder(cards: readonly UnitCard[]): UnitCard[] {
  const family = new Map<string, number>();
  for (const card of cards) if (!family.has(card.family)) family.set(card.family, family.size);
  return cards
    .map((card, at) => ({ card, at }))
    .sort(
      (a, b) =>
        family.get(a.card.family)! - family.get(b.card.family)! ||
        a.card.cost - b.card.cost ||
        a.at - b.at,
    )
    .map(({ card }) => card);
}

/** What the battle observes about buying right now. */
export interface BuyFacts {
  credits: number;
  unitCount: number;
  maxUnits: number;
  finished: boolean;
}

/** On sale, affordable, under the unit cap, battle running: whether a card
 *  can be bought now. */
export function cardPurchasable(card: UnitCard, facts: BuyFacts): boolean {
  return (
    !facts.finished &&
    card.disabled_reason === null &&
    card.cost <= facts.credits &&
    facts.unitCount < facts.maxUnits
  );
}

/** What a squad costs with its ride, or alone on foot (`ride` null). */
export const pairCost = (squad: UnitCard, ride: UnitCard | null) => squad.cost + (ride?.cost ?? 0);

/** On sale both, both prices within the credits, room under the cap for two
 *  units, battle running: whether a squad can be bought with its carrier. */
export function pairPurchasable(squad: UnitCard, carrier: UnitCard, facts: BuyFacts): boolean {
  return (
    !facts.finished &&
    squad.disabled_reason === null &&
    carrier.disabled_reason === null &&
    pairCost(squad, carrier) <= facts.credits &&
    facts.unitCount + 2 <= facts.maxUnits
  );
}

/** A section of a squad's transport menu: the carriers under one shop tab. */
export interface CarrierSection {
  group: UnitGroup;
  label: string;
  carriers: UnitCard[];
}

/** The carriers that can take a squad of `squadSize` soldiers: every card on
 *  sale that seats them all, sectioned by its shop tab in tab order, cheapest
 *  first within a section (catalog order breaks ties). `seats` is a card's
 *  seat count, 0 for one that carries no one. */
export function carrierSections(
  squadSize: number,
  cards: readonly UnitCard[],
  seats: (card: UnitCard) => number,
  groupOf: (card: UnitCard) => UnitGroup,
): CarrierSection[] {
  const fit = cards
    .map((card, at) => ({ card, at }))
    .filter(({ card }) => card.disabled_reason === null && seats(card) >= squadSize)
    .sort((a, b) => a.card.cost - b.card.cost || a.at - b.at)
    .map(({ card }) => card);
  return UNIT_GROUPS.map(({ id, label }) => ({
    group: id,
    label,
    carriers: fit.filter((card) => groupOf(card) === id),
  })).filter((section) => section.carriers.length > 0);
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

/** The card Enter buys: the sole match, when it can be bought now; null with
 *  none, several, or one not purchasable. */
export function enterChoice(matches: readonly UnitCard[], facts: BuyFacts): UnitCard | null {
  return matches.length === 1 && cardPurchasable(matches[0], facts) ? matches[0] : null;
}
