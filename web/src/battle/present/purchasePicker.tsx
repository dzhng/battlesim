import { useState } from "react";
import type { Faction, UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import type { SkirmishView } from "../sim/observation";
interface PickerProps {
  cards: readonly UnitCard[];
  faction: Faction;
  match: SkirmishView;
  onChoose: (variant: string) => void;
  onReady: () => void;
  onCancelPending: (id: number) => void;
}
const CATEGORIES: readonly UnitCategory[] = ["rec", "inf", "veh", "sup", "hel", "air"];
/** Catalog families organize browsing; only a concrete available variant starts placement. */
export function PurchasePicker({
  cards,
  faction,
  match,
  onChoose,
  onReady,
  onCancelPending,
}: PickerProps) {
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
        <span aria-label="Unit slots">
          {match.occupiedSlots}/{match.maxUnits}
        </span>
        {match.phase === "preparation" && (
          <button
            type="button"
            className="hud-menu-choice"
            disabled={match.ready[0]}
            onClick={onReady}
          >
            {match.ready[0] ? "Ready" : "Ready for battle"}
          </button>
        )}
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
              const named = variants.find((c) => UNITS.has(c.id));
              const silhouette = named && unitIcons(UNITS.type(named.id)).silhouette;
              return (
                <button
                  key={id}
                  type="button"
                  className="hud-purchase-family"
                  aria-label={variants[0].roster.family_name}
                  aria-pressed={id === family}
                  onClick={() => setFamily(id)}
                >
                  {silhouette && <Icon path={silhouette} />}
                  <span>{variants[0].roster.family_name}</span>
                  <span className="hud-purchase-price">
                    {Math.min(...variants.map((v) => v.cost))} CR
                  </span>
                  {variants.every((v) => v.disabled_reason !== null) && <small>Unavailable</small>}
                </button>
              );
            })}
          </div>
          {chosen.length > 0 && (
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
                  <span>{card.cost} CR</span>
                  {card.disabled_reason !== null && <small>Unavailable</small>}
                </button>
              ))}
            </div>
          )}
        </section>
      )}
      {match.pending.length > 0 && (
        <div className="hud-purchase-pending" role="group" aria-label="Incoming units">
          {match.pending.map((p) => (
            <button
              type="button"
              key={p.id}
              className="hud-menu-choice"
              disabled={finished}
              onClick={() => onCancelPending(p.id)}
              aria-label={`Cancel incoming ${available.find((c) => c.id === p.kind)?.name ?? p.kind}`}
            >
              <span>{available.find((c) => c.id === p.kind)?.name ?? p.kind}</span>
              <small>{p.blocked ? "Entry blocked" : "Incoming"} · Cancel</small>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
