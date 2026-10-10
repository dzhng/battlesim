import { useLayoutEffect, useRef, useState } from "react";
import type { Faction, UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import { useSessionCatalog } from "../catalog/context";
import { InfoPanel } from "./infoPanel";
import { purchasePanel, type PanelRules } from "./panelRows";
import { UnitFace } from "./unitFace";
import type { SkirmishView } from "../sim/observation";
interface PickerProps {
  cards: readonly UnitCard[];
  faction: Faction;
  match: SkirmishView;
  onChoose: (variant: string) => void;
  /** What a purchase's info card is built with, as the deck's are. */
  rules: PanelRules;
}
/** A price, in the warning colour when the credits can't pay it. */
function Price({ cost, credits }: { cost: number; credits: number }) {
  return (
    <span className={cost > credits ? "hud-purchase-price too-dear" : "hud-purchase-price"}>
      {cost} CR
    </span>
  );
}

/** How long the pointer rests on another family, while a menu is open,
 *  before the hover moves to it. */
const HOVER_INTENT_MS = 250;

const CATEGORIES: readonly UnitCategory[] = ["rec", "inf", "veh", "sup", "hel", "air"];
/** The Reinforcements button and the credits. The menu exists only while
 *  open, so every opening starts fresh: first category, no family shown. */
export function PurchasePicker({ match, ...menu }: PickerProps) {
  const [open, setOpen] = useState(false);
  return (
    <div className="hud-reinforcements" data-occludes-readouts>
      <div className="hud-purchase-controls">
        <button
          type="button"
          className="hud-menu-choice"
          aria-label="Reinforcements"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          Reinforcements
        </button>
        <span className="hud-credits" aria-label="Credits">
          {Math.floor(match.credits)} CR
        </span>
      </div>
      {open && (
        <PurchaseMenu
          {...menu}
          match={match}
          onChoose={(variant) => {
            menu.onChoose(variant);
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}

/** Catalog families organize browsing; only a concrete available variant starts placement. */
function PurchaseMenu({ cards, faction, match, onChoose, rules }: PickerProps) {
  const catalog = useSessionCatalog();
  const [category, setCategory] = useState<UnitCategory>("rec");
  const [family, setFamily] = useState<string | null>(null);
  // The menu hangs from its family's card's top-left corner, measured
  // after every render from the card itself (whatever chose the family),
  // inside the picker's border, where absolute positions start.
  const flyout = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const menu = flyout.current;
    const picker = menu?.closest<HTMLElement>(".hud-purchase-picker");
    const card = picker?.querySelector<HTMLElement>(`[data-family="${family}"]`);
    if (!menu || !picker || !card) return;
    const at = card.getBoundingClientRect();
    const box = picker.getBoundingClientRect();
    menu.style.left = `${at.left - box.left - picker.clientLeft}px`;
    menu.style.top = `${at.top - box.top - picker.clientTop}px`;
  });
  // Hover intent, as a web menu has it: while a menu is open, crossing
  // another card on the way into the menu does not close it. Moving onto
  // another family switches only after a pause; reaching the menu cancels.
  const pendingHover = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cancelHover = () => {
    if (pendingHover.current) clearTimeout(pendingHover.current);
    pendingHover.current = null;
  };
  const hover = (id: string | null, menuOpen: boolean) => {
    cancelHover();
    const show = () => setFamily(id);
    if (menuOpen && id !== family) pendingHover.current = setTimeout(show, HOVER_INTENT_MS);
    else show();
  };
  const available = cards.filter((c) => c.roster.factions.includes(faction));
  const families = new Map<string, UnitCard[]>();
  for (const card of available.filter((c) => c.roster.category === category)) {
    const group = families.get(card.family) ?? [];
    group.push(card);
    families.set(card.family, group);
  }
  const chosen = families.get(family ?? "") ?? [];
  const finished = match.phase === "finished";
  return (
    <section
      className="hud-panel hud-purchase-picker"
      aria-label="Faction units"
      onMouseLeave={() => hover(null, false)}
    >
      <div className="hud-purchase-faction">
        {{ us: "U.S.", europe: "EUROPE", eastern: "EASTERN" }[faction]}
      </div>
      <div className="hud-purchase-categories" role="tablist" aria-label="Unit categories">
        {CATEGORIES.map((c) => (
          <button
            key={c}
            type="button"
            role="tab"
            className="hud-menu-choice"
            aria-selected={c === category}
            onClick={() => {
              setCategory(c);
              cancelHover();
              setFamily(null);
            }}
          >
            {c.toUpperCase()}
          </button>
        ))}
      </div>
      <div className="hud-purchase-families" role="tabpanel" aria-label={category.toUpperCase()}>
        {[...families].map(([id, variants]) => {
          // Every card has its generated silhouette, keyed by card id: a
          // unit type's from its model, a disabled card's from its source.
          const unavailable = variants.every((v) => v.disabled_reason !== null);
          const shown = variants.find((v) => v.disabled_reason === null) ?? variants[0];
          const firstAvailable = variants.find(
            (v) =>
              finished === false &&
              v.disabled_reason === null &&
              v.cost <= match.credits &&
              match.occupiedSlots < match.maxUnits,
          );
          return (
            <button
              key={id}
              type="button"
              className={`hud-army-card hud-purchase-family${unavailable ? " unavailable" : firstAvailable ? "" : " short"}`}
              aria-label={variants[0].roster.family_name}
              aria-pressed={id === family}
              data-family={id}
              onMouseEnter={() => hover(id, chosen.length > 0)}
              onFocus={() => hover(id, false)}
              onClick={() => {
                cancelHover();
                setFamily(id);
                if (firstAvailable) onChoose(firstAvailable.id);
              }}
            >
              <UnitFace kind={shown.id}>
                <Price cost={Math.min(...variants.map((v) => v.cost))} credits={match.credits} />
              </UnitFace>
            </button>
          );
        })}
      </div>
      {/* The hovered family's info cards, one per variant (one for a
          family of one), stacked in a hover menu standing on its card,
          over the picker, so the picker never changes size: the deck's
          own info panel, with the price. Clicking a card buys it. The
          menu's transparent foot bridges the gap for the pointer;
          leaving the picker closes it. */}
      {chosen.length > 0 && (
        <div className="hud-purchase-flyout" ref={flyout} onMouseEnter={cancelHover}>
          <div
            className="hud-purchase-stack"
            role="group"
            aria-label={`${chosen[0].roster.family_name} variants`}
          >
            {/* The dearest on top, the cheapest nearest the pointer. */}
            {[...chosen]
              .sort((a, b) => b.cost - a.cost)
              .map((card) => (
                <button
                  key={card.id}
                  type="button"
                  className="hud-panel hud-card hud-purchase-info"
                  disabled={
                    finished ||
                    card.disabled_reason !== null ||
                    card.cost > match.credits ||
                    match.occupiedSlots >= match.maxUnits
                  }
                  aria-label={`${card.roster.variant} — ${card.cost} credits${card.disabled_reason !== null ? " — Unavailable" : ""}`}
                  onClick={() => onChoose(card.id)}
                >
                  <InfoPanel panel={purchasePanel(catalog, card, rules)} />
                  <span className="hud-purchase-info-foot">
                    {card.disabled_reason !== null && <small>Unavailable</small>}
                    <Price cost={card.cost} credits={match.credits} />
                  </span>
                </button>
              ))}
          </div>
        </div>
      )}
    </section>
  );
}
