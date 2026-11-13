/** The reinforcements picker's filter: the groups it browses by, matching,
 *  buyability and the Enter choice, as pure data. */
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

/** What the battle observes about buying right now. */
export interface BuyFacts {
  credits: number;
  unitCount: number;
  maxUnits: number;
  finished: boolean;
}

/** On sale, affordable, under the unit cap, battle running: the one answer the
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
