// The Armory, layout A (U1): faction tabs, group tabs and one tile per family
// on the left, the selected card's full sheet on the right, and the stage
// between them. Until the stage arrives (slice 11) the menu's backdrop shows
// through the centre. The selection is the page's own state, mirrored to the
// address (`/armory?unit=<card id>`) with a raw `history.replaceState`, which
// the router never sees, so a pick never remounts the page (L13).
import { Suspense, useEffect, useMemo, useState, type ReactNode } from "react";
import { FACTIONS, type Faction, type UnitCard } from "@packages/scene-assets/src/units";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { SessionCatalogScope, useSessionCatalog } from "@web/battle/catalog/context";
import { UNIT_GROUPS, unitGroup, type UnitGroup } from "@web/battle/present/purchaseFilter";
import { UnitFace } from "@web/battle/present/unitFace";
import { spoken } from "../battleLinks";
import { ArmorySheet } from "./ArmorySheet";

/** What the Armory shows: the open faction tab, the open group tab and the
 *  card whose sheet is open. */
export interface ArmorySelection {
  faction: Faction;
  group: UnitGroup;
  card: string;
}

/** The cards the Armory browses, and the group each one sits under. */
export interface ArmoryRoster {
  cards: readonly UnitCard[];
  groupOf(card: UnitCard): UnitGroup;
}

/** A family sits under the group of its first variant with a type, so its
 *  tile never splits across two tabs; a family with no typed variant takes
 *  its category's first group, as `unitGroup` places a planned card. */
export function armoryRoster(catalog: SessionCatalog, cards: readonly UnitCard[]): ArmoryRoster {
  const families = new Map<string, UnitGroup>();
  for (const card of cards)
    if (catalog.units.has(card.id) && !families.has(card.family))
      families.set(card.family, unitGroup(card, catalog.units.type(card.id)));
  return { cards, groupOf: (card) => families.get(card.family) ?? unitGroup(card, null) };
}

/** The cards one faction fields under one group, in catalog order. */
const shown = (roster: ArmoryRoster, faction: Faction, group: UnitGroup) =>
  roster.cards.filter((c) => c.roster.factions.includes(faction) && roster.groupOf(c) === group);

const firstGroup = (roster: ArmoryRoster, faction: Faction) =>
  UNIT_GROUPS.find((g) => shown(roster, faction, g.id).length > 0)?.id;

/** The selection a pick leads to from `from`. A card opens under its own
 *  group, and under the open faction when that faction fields it. A tab
 *  opens on its first tile; a faction keeps the open card where it fields
 *  it. An unknown card, or none, opens the first tile of the first faction's
 *  first group that has one. */
function settle(
  roster: ArmoryRoster,
  from: ArmorySelection | null,
  pick: Partial<ArmorySelection>,
): ArmorySelection {
  const byId = (id: string | undefined) => roster.cards.find((c) => c.id === id);
  const card = byId(pick.card);
  if (card) {
    const faction =
      [pick.faction, from?.faction].find((f) => f && card.roster.factions.includes(f)) ??
      card.roster.factions[0];
    return { faction, group: roster.groupOf(card), card: card.id };
  }
  const faction =
    pick.faction ?? from?.faction ?? FACTIONS.find((f) => firstGroup(roster, f)) ?? FACTIONS[0];
  const open = byId(from?.card);
  if (!pick.group && from && open?.roster.factions.includes(faction))
    return { faction, group: from.group, card: open.id };
  const wanted = pick.group ?? from?.group;
  const group =
    wanted && shown(roster, faction, wanted).length > 0 ? wanted : firstGroup(roster, faction);
  if (!group) return { faction, group: UNIT_GROUPS[0].id, card: "" };
  return { faction, group, card: shown(roster, faction, group)[0].id };
}

/** The page's selection, opened from `?unit=` and mirrored back to it. The
 *  address is replaced, never pushed: Back leaves the Armory, as it came. */
export function useArmorySelection(
  roster: ArmoryRoster,
): [ArmorySelection, (pick: Partial<ArmorySelection>) => void] {
  const [selection, setSelection] = useState(() =>
    settle(roster, null, { card: new URLSearchParams(location.search).get("unit") ?? undefined }),
  );
  useEffect(() => {
    const url = new URL(location.href);
    if (url.searchParams.get("unit") === selection.card) return;
    url.searchParams.set("unit", selection.card);
    history.replaceState(history.state, "", `${url.pathname}${url.search}${url.hash}`);
  }, [selection.card]);
  return [selection, (pick) => setSelection((from) => settle(roster, from, pick))];
}

interface ArmoryPageProps {
  /** The cards to browse: the game's roster, or a fixed set in the gallery. */
  cards: readonly UnitCard[];
  /** What stands in the centre for the open card, under the open faction's
   *  look (L11). Absent, the menu's backdrop shows through. */
  stage?: (card: UnitCard, faction: Faction) => ReactNode;
}

/** The Armory's list, stage and sheet, on the session catalog. */
export function ArmoryPage({ cards, stage }: ArmoryPageProps) {
  const catalog = useSessionCatalog();
  const roster = useMemo(() => armoryRoster(catalog, cards), [catalog, cards]);
  const [selection, select] = useArmorySelection(roster);
  const card = roster.cards.find((c) => c.id === selection.card);
  const families = new Map<string, UnitCard[]>();
  for (const c of shown(roster, selection.faction, selection.group))
    families.set(c.family, [...(families.get(c.family) ?? []), c]);
  const group = UNIT_GROUPS.find((g) => g.id === selection.group)!;
  return (
    <>
      <section className="hud-panel armory-list" aria-label="Units">
        <div className="armory-browse">
          <div className="armory-tabs armory-factions" role="tablist" aria-label="Factions">
            {FACTIONS.map((faction) => (
              <button
                key={faction}
                type="button"
                role="tab"
                className="hud-menu-choice"
                aria-selected={faction === selection.faction}
                disabled={!firstGroup(roster, faction)}
                onClick={() => select({ faction })}
              >
                {spoken(faction)}
              </button>
            ))}
          </div>
          <div className="armory-tabs armory-groups" role="tablist" aria-label="Unit groups">
            {UNIT_GROUPS.map(({ id, label }) => {
              // How many tiles the tab holds, beside its name.
              const count = new Set(shown(roster, selection.faction, id).map((c) => c.family)).size;
              return (
                <button
                  key={id}
                  type="button"
                  role="tab"
                  className="hud-menu-choice"
                  aria-label={label}
                  aria-selected={id === selection.group}
                  disabled={count === 0}
                  onClick={() => select({ group: id })}
                >
                  {label}
                  {count > 0 && <span className="armory-count">{count}</span>}
                </button>
              );
            })}
          </div>
          <div className="armory-tiles" role="tabpanel" aria-label={group.label}>
            {[...families].map(([family, variants]) => {
              // A family is planned until one of its variants has a type: its
              // tile is greyed, as the picker greys a family never sold.
              const typed = variants.find((v) => catalog.units.has(v.id));
              return (
                <button
                  key={family}
                  type="button"
                  className={`hud-army-card hud-purchase-family armory-tile${typed ? "" : " unavailable"}`}
                  data-family={family}
                  data-cue-hover="off"
                  aria-pressed={family === card?.family}
                  onClick={() => select({ card: variants[0].id })}
                >
                  {/* A planned family says so; a family of several, how many. */}
                  {(!typed || variants.length > 1) && (
                    <span className="armory-tile-tag">
                      {typed ? `${variants.length} VAR` : "PLANNED"}
                    </span>
                  )}
                  <UnitFace kind={(typed ?? variants[0]).id}>
                    <span className="armory-tile-name">{variants[0].roster.family_name}</span>
                    <span className={`hud-purchase-price${typed ? "" : " unsold"}`}>
                      {Math.min(...variants.map((v) => v.cost))} CR
                    </span>
                  </UnitFace>
                </button>
              );
            })}
          </div>
        </div>
      </section>
      <div className="armory-stage">{card && stage?.(card, selection.faction)}</div>
      {card && (
        <ArmorySheet
          card={card}
          group={group.label}
          variants={shown(roster, selection.faction, selection.group).filter(
            (v) => v.family === card.family,
          )}
          onVariant={(id) => select({ card: id })}
        />
      )}
    </>
  );
}

/** The menu's Armory page: the game's roster. */
export function ArmoryVisit() {
  return (
    <Suspense fallback={null}>
      <SessionCatalogScope set="game">
        <GameArmory />
      </SessionCatalogScope>
    </Suspense>
  );
}

function GameArmory() {
  const { units } = useSessionCatalog();
  return <ArmoryPage cards={units.cards} />;
}
