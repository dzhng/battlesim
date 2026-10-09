import { useState } from "react";
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

const CATEGORIES: readonly UnitCategory[] = ["rec", "inf", "veh", "sup", "hel", "air"];
/** Catalog families organize browsing; only a concrete available variant starts placement. */
export function PurchasePicker({ cards, faction, match, onChoose }: PickerProps) {
  const [open, setOpen] = useState(false);
  const [category, setCategory] = useState<UnitCategory>("rec");
  const [family, setFamily] = useState<string | null>(null);
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
        <section className="hud-panel hud-purchase-picker" aria-label="Faction units">
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
                  onMouseEnter={() => setFamily(id)}
                  onFocus={() => setFamily(id)}
                  onClick={() => {
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
          {/* The army's cards carry no names: the family under the pointer
              names itself here, with its variants. The row keeps its room
              empty, so showing it never moves the cards. */}
          <div className="hud-purchase-detail">
            {chosen.length > 0 && (
              <span className="hud-purchase-detail-name">{chosen[0].roster.family_name}</span>
            )}
            {chosen.length === 1 && (
              <span className="hud-purchase-single">
                <Price cost={chosen[0].cost} credits={match.credits} />
                {chosen[0].disabled_reason !== null && <small>Unavailable</small>}
              </span>
            )}
            {chosen.length > 1 && (
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
            )}
          </div>
        </section>
      )}
    </div>
  );
}
