/** The shop's two faces of a card for sale, shared by the picker's grid and a
 *  squad's transport menu: the card itself (the army deck's vertical card,
 *  with its price) and its info card (the deck's info panel, with the price
 *  at its foot). */
import type { UnitCard } from "@packages/scene-assets/src/units";
import { InfoPanel } from "./infoPanel";
import type { Panel } from "./panelRows";
import { UnitFace } from "./unitFace";

/** A price: gold when the credits can pay it, the "can't" red when they
 *  can't, and plain text for what is never sold, since no credits buy it.
 *  `add` reads it as a cost on top of another, a ride's on its squad's. */
export function Price({
  cost,
  payable,
  unsold = false,
  add = false,
}: {
  cost: number;
  payable: boolean;
  unsold?: boolean;
  add?: boolean;
}) {
  const state = unsold ? " unsold" : payable ? "" : " too-dear";
  return (
    <span className={`hud-purchase-price${state}`}>
      {add && "+"}
      {cost} CR
    </span>
  );
}

/** A card for sale: `face`'s role and silhouette over its price. It can be
 *  hovered whether or not it can be bought now (`aria-disabled` says it
 *  can't), so the player can still read what it is; a click buys only while
 *  `purchasable`. */
export function ShopCard({
  face,
  label,
  cost,
  payable,
  add,
  purchasable,
  pressed,
  describedBy,
  data,
  onHover,
  onChoose,
}: {
  face: UnitCard;
  label: string;
  cost: number;
  payable: boolean;
  add?: boolean;
  purchasable: boolean;
  pressed: boolean;
  describedBy?: string;
  /** The card's `data-*` attributes, what the shop finds it by. */
  data: Record<`data-${string}`, string>;
  onHover: () => void;
  onChoose: () => void;
}) {
  const unsold = face.disabled_reason !== null;
  return (
    <button
      type="button"
      className={`hud-army-card hud-purchase-card${unsold ? " unavailable" : purchasable ? "" : " short"}`}
      aria-label={label}
      aria-disabled={!purchasable}
      aria-pressed={pressed}
      aria-describedby={describedBy}
      {...data}
      // A click never takes the text cursor out of the search field.
      onMouseDown={(event) => event.preventDefault()}
      onMouseEnter={onHover}
      onFocus={onHover}
      onClick={() => {
        onHover();
        if (purchasable) onChoose();
      }}
    >
      {/* Every card has its generated silhouette, keyed by card id: a unit
          type's from its model, a disabled card's from its source. */}
      <UnitFace kind={face.id}>
        <Price cost={cost} payable={payable} unsold={unsold} add={add} />
      </UnitFace>
    </button>
  );
}

/** A card's info card: the deck's own info panel (`panel`, the card's
 *  unit as it arrives), with the price at its foot. `short` dims its words
 *  when it can't be bought now (never its plate: it stands over the shop).
 *  It only shows: the pointer passes through it to the cards beneath. */
export function PurchaseInfo({
  id,
  card,
  panel,
  credits,
  short,
}: {
  id?: string;
  card: UnitCard;
  panel: Panel;
  credits: number;
  short: boolean;
}) {
  const unsold = card.disabled_reason !== null;
  return (
    <div
      id={id}
      role="tooltip"
      aria-label={card.name}
      className={`hud-panel hud-card hud-purchase-info${short ? " short" : ""}`}
    >
      <InfoPanel panel={panel} />
      <span className="hud-purchase-info-foot">
        {unsold && <small>Unavailable</small>}
        <Price cost={card.cost} payable={card.cost <= credits} unsold={unsold} />
      </span>
    </div>
  );
}
