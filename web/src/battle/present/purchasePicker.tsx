import { useRef, useState } from "react";
import type { Faction, UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import { UnitFace } from "./unitFace";
import type { SkirmishView } from "../sim/observation";
interface PickerProps {
  cards: readonly UnitCard[];
  faction: Faction;
  match: SkirmishView;
  onChoose: (variant: string) => void;
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
/** Catalog families organize browsing; only a concrete available variant starts placement. */
export function PurchasePicker({ cards, faction, match, onChoose }: PickerProps) {
  const [open, setOpen] = useState(false);
  const [category, setCategory] = useState<UnitCategory>("rec");
  const [family, setFamily] = useState<string | null>(null);
  // Where the hovered family's card stands in the picker: its menu hangs
  // from the card's top-left corner.
  const [anchor, setAnchor] = useState({ left: 0, top: 0 });
  // Hover intent, as a web menu has it: while a menu is open, crossing
  // another card on the way into the menu does not close it. Moving onto
  // another family switches only after a pause; reaching the menu cancels.
  const pendingHover = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cancelHover = () => {
    if (pendingHover.current) clearTimeout(pendingHover.current);
    pendingHover.current = null;
  };
  const hover = (id: string | null, card: HTMLElement | null, menuOpen: boolean) => {
    cancelHover();
    const show = () => {
      setFamily(id);
      if (!card) return;
      const at = card.getBoundingClientRect();
      const picker = card.closest(".hud-purchase-picker")!.getBoundingClientRect();
      setAnchor({ left: at.left - picker.left, top: at.top - picker.top });
    };
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
        <section
          className="hud-panel hud-purchase-picker"
          aria-label="Faction units"
          onMouseLeave={() => hover(null, null, false)}
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
          <div
            className="hud-purchase-families"
            role="tabpanel"
            aria-label={category.toUpperCase()}
          >
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
                  onMouseEnter={(event) => hover(id, event.currentTarget, chosen.length > 1)}
                  onFocus={(event) => hover(id, event.currentTarget, false)}
                  onClick={() => {
                    cancelHover();
                    setFamily(id);
                    if (firstAvailable) {
                      onChoose(firstAvailable.id);
                      setOpen(false);
                    }
                  }}
                >
                  <UnitFace kind={shown.id}>
                    <Price
                      cost={Math.min(...variants.map((v) => v.cost))}
                      credits={match.credits}
                    />
                  </UnitFace>
                </button>
              );
            })}
          </div>
          {/* A family of variants lists them in a hover menu standing on its
              card, over the picker, so the picker never changes size. The
              menu's transparent foot bridges the gap, so the pointer can
              climb into it; leaving the picker closes it. One variant opens
              nothing. */}
          {chosen.length > 1 && (
            <div className="hud-purchase-flyout" style={anchor} onMouseEnter={cancelHover}>
              <div className="hud-panel hud-purchase-flyout-panel">
                <span className="hud-purchase-flyout-name">{chosen[0].roster.family_name}</span>
                <div
                  className="hud-purchase-variants"
                  role="group"
                  aria-label={`${chosen[0].roster.family_name} variants`}
                >
                  {chosen.map((card) => (
                    <button
                      key={card.id}
                      type="button"
                      className="hud-menu-choice"
                      disabled={
                        finished ||
                        card.disabled_reason !== null ||
                        card.cost > match.credits ||
                        match.occupiedSlots >= match.maxUnits
                      }
                      aria-label={`${card.roster.variant} — ${card.cost} credits${card.disabled_reason !== null ? " — Unavailable" : ""}`}
                      onClick={() => {
                        onChoose(card.id);
                        setOpen(false);
                      }}
                    >
                      <span>{card.roster.variant}</span>
                      <Price cost={card.cost} credits={match.credits} />
                      {card.disabled_reason !== null && <small>Unavailable</small>}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}
        </section>
      )}
    </div>
  );
}
