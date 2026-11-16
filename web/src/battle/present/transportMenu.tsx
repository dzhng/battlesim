/** A squad's transport menu: over its card in the shop, the squad on foot
 *  and every carrier that seats it, by shop tab; the option the player points
 *  at (or steps to with the arrow keys) shows what it buys above the menu. */
import { useId, useLayoutEffect, useRef } from "react";
import type { UnitCard } from "@packages/scene-assets/src/units";
import { Price, PurchaseInfo, ShopCard } from "./purchaseCard";
import {
  cardPurchasable,
  pairPurchasable,
  type BuyFacts,
  type CarrierSection,
} from "./purchaseFilter";
import { useSessionCatalog } from "../catalog/context";
import { purchasePairPanels, purchasePanel, type PanelRules } from "./panelRows";

/** The menu's first option: the squad bought alone. Every other option is a
 *  carrier's card id. */
export const ON_FOOT = "on-foot";

/** The menu's options in the order the arrow keys step through them: on
 *  foot, then each section's carriers as they stand. */
export const transportOptions = (sections: readonly CarrierSection[]): string[] => [
  ON_FOOT,
  ...sections.flatMap((s) => s.carriers.map((c) => c.id)),
];

/** The carrier an option names; null on foot. */
export const optionCarrier = (sections: readonly CarrierSection[], option: string) =>
  sections.flatMap((s) => s.carriers).find((c) => c.id === option) ?? null;

/** Whether `option` can be bought now: the squad alone, or with that ride. */
export function optionPurchasable(
  squad: UnitCard,
  carrier: UnitCard | null,
  facts: BuyFacts,
): boolean {
  return carrier ? pairPurchasable(squad, carrier, facts) : cardPurchasable(squad, facts);
}

/** A position inside the picker's border, where absolute positions start. */
function within(picker: HTMLElement, rect: DOMRect) {
  const box = picker.getBoundingClientRect();
  return {
    left: rect.left - box.left - picker.clientLeft,
    top: rect.top - box.top - picker.clientTop,
  };
}

/** `left` kept inside the picker for an element `width` wide. */
const clampLeft = (picker: HTMLElement, left: number, width: number) =>
  Math.max(0, Math.min(left, picker.clientWidth - width));

export function TransportMenu({
  squad,
  sections,
  facts,
  rules,
  option,
  onOption,
  onBuy,
  onEnter,
}: {
  squad: UnitCard;
  sections: readonly CarrierSection[];
  facts: BuyFacts;
  rules: PanelRules;
  /** The highlighted option, null until the player shows intent. */
  option: string | null;
  onOption: (option: string) => void;
  /** Buys `option`, when it can be bought now. */
  onBuy: (option: string) => void;
  /** The pointer reached the menu. */
  onEnter: () => void;
}) {
  const catalog = useSessionCatalog();
  const infoId = useId();
  const menu = useRef<HTMLDivElement>(null);
  const stack = useRef<HTMLDivElement>(null);
  const carrier = option ? optionCarrier(sections, option) : null;
  const pair = carrier ? pairPurchasable(squad, carrier, facts) : false;
  const panels = carrier ? purchasePairPanels(catalog, squad, carrier, rules) : null;
  const ids = carrier ? `${infoId}-carrier ${infoId}-squad ${infoId}-total` : infoId;
  // The menu stands on its squad's card, its left edge at the card's; the
  // option's info stands on the whole menu, its left edge at the option's.
  // Both stay inside the picker's width. Measured after every render.
  useLayoutEffect(() => {
    const wrap = menu.current;
    const picker = wrap?.closest<HTMLElement>(".hud-purchase-picker");
    const card = picker?.querySelector(`.hud-purchase-cards [data-card="${squad.id}"]`);
    if (!wrap || !picker || !card) return;
    const at = within(picker, card.getBoundingClientRect());
    const top = at.top - wrap.offsetHeight;
    wrap.style.left = `${clampLeft(picker, at.left, wrap.offsetWidth)}px`;
    wrap.style.top = `${top}px`;
    const info = stack.current;
    const chosen = option && wrap.querySelector(`[data-option="${option}"]`);
    if (!info || !chosen) return;
    const left = within(picker, chosen.getBoundingClientRect()).left;
    info.style.left = `${clampLeft(picker, left, info.offsetWidth)}px`;
    info.style.top = `${top - info.offsetHeight}px`;
  });
  // An option's card: the squad's own face on foot, a ride's face priced as
  // what it adds, its price payable when the credits cover the two.
  const card = (id: string, ride: UnitCard | null) => {
    const face = ride ?? squad;
    return (
      <ShopCard
        key={id}
        face={face}
        label={ride ? ride.name : "On foot"}
        cost={face.cost}
        payable={(ride?.cost ?? 0) + squad.cost <= facts.credits}
        add={ride !== null}
        purchasable={optionPurchasable(squad, ride, facts)}
        pressed={id === option}
        describedBy={id === option ? ids : undefined}
        data={{ "data-option": id }}
        onHover={() => onOption(id)}
        onChoose={() => onBuy(id)}
      />
    );
  };
  return (
    <>
      {/* The wrapper's foot bridges the gap down to the squad's card, so the
          pointer never leaves the menu on its way in. */}
      <div className="hud-transport" ref={menu} onMouseEnter={onEnter}>
        <section className="hud-panel hud-transport-menu" aria-label="Transport">
          <div className="hud-transport-section" role="group" aria-label="On foot">
            <span className="hud-transport-label" aria-hidden>
              On foot
            </span>
            <div className="hud-transport-cards">{card(ON_FOOT, null)}</div>
          </div>
          {sections.map(({ group, label, carriers }) => (
            <div key={group} className="hud-transport-section" role="group" aria-label={label}>
              <span className="hud-transport-label" aria-hidden>
                {label}
              </span>
              <div className="hud-transport-cards">{carriers.map((c) => card(c.id, c))}</div>
            </div>
          ))}
        </section>
      </div>
      {option && (
        <div className="hud-transport-stack" ref={stack}>
          {carrier && panels ? (
            <>
              <PurchaseInfo
                id={`${infoId}-carrier`}
                card={carrier}
                panel={panels.carrier}
                credits={facts.credits}
                short={!pair}
              />
              <PurchaseInfo
                id={`${infoId}-squad`}
                card={squad}
                panel={panels.squad}
                credits={facts.credits}
                short={!pair}
              />
              <div
                id={`${infoId}-total`}
                className="hud-panel hud-purchase-total"
                aria-label="Total"
              >
                <span>Total</span>
                {/* The sum reads as the stack above it: carrier, then squad. */}
                <span className="hud-purchase-sum">
                  {carrier.cost} + {squad.cost} =
                </span>
                <Price
                  cost={squad.cost + carrier.cost}
                  payable={squad.cost + carrier.cost <= facts.credits}
                />
              </div>
            </>
          ) : (
            <PurchaseInfo
              id={infoId}
              card={squad}
              panel={purchasePanel(catalog, squad, rules)}
              credits={facts.credits}
              short={!cardPurchasable(squad, facts)}
            />
          )}
        </div>
      )}
    </>
  );
}
