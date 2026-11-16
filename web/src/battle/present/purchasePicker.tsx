import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { Faction, UnitCard } from "@packages/scene-assets/src/units";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { isTypingTarget } from "../input/keyGuard";
import { useSessionCatalog } from "../catalog/context";
import { Icon } from "./icons";
import { InfoPanel } from "./infoPanel";
import { purchasePanel, type PanelRules } from "./panelRows";
import {
  cardPurchasable,
  enterChoice,
  filterCards,
  shopOrder,
  UNIT_GROUPS,
  unitGroup,
  type BuyFacts,
  type UnitGroup,
} from "./purchaseFilter";
import { UnitFace } from "./unitFace";
import type { PlayerView, SkirmishView } from "../sim/observationView";
interface PickerProps {
  cards: readonly UnitCard[];
  faction: Faction;
  match: SkirmishView;
  player?: PlayerView;
  onChoose: (variant: string) => void;
  /** What a purchase's info card is built with, as the deck's are. */
  rules: PanelRules;
  /** Whether the shop's key answers: off while a menu or result covers it. */
  enabled?: boolean;
}
/** A price: gold when the credits can pay it, the "can't" red when they
 *  can't, and plain text for what is never sold, since no credits buy it. */
function Price({ cost, credits, unsold }: { cost: number; credits: number; unsold: boolean }) {
  const state = unsold ? " unsold" : cost > credits ? " too-dear" : "";
  return <span className={`hud-purchase-price${state}`}>{cost} CR</span>;
}

/** The shop's key. Tab, not a letter, so every letter stays a search
 *  character and no command key is spent: one key opens the shop, enters its
 *  search and leaves it again. */
const REINFORCEMENTS_KEY = { code: "Tab", label: "Tab" } as const;

/** A press of the key, with or without Shift, wherever it lands: the battle
 *  never walks focus onto a button. Ctrl, Cmd and Alt+Tab stay the browser's
 *  and the system's. */
const isShopTab = (event: KeyboardEvent) =>
  event.code === REINFORCEMENTS_KEY.code &&
  !event.ctrlKey &&
  !event.metaKey &&
  !event.altKey &&
  !event.repeat;

/** The Reinforcements button, its key and the credits. The menu exists only
 *  while open, so every opening starts fresh: first group, empty filter. */
export function PurchasePicker({ match, player, enabled = true, ...menu }: PickerProps) {
  const viewingPlayer = player ?? match.players[0];
  if (!viewingPlayer) return null;
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  // The key opens the shop; once it is open, the menu owns the key.
  useEffect(() => {
    if (!enabled || open) return;
    const onKey = (event: KeyboardEvent) => {
      if (!isShopTab(event) || isTypingTarget(event.target)) return;
      event.preventDefault();
      setOpen(true);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled, open]);
  return (
    <div className="hud-reinforcements" data-occludes-readouts>
      <div className="hud-purchase-controls">
        <button
          ref={button}
          type="button"
          className="hud-menu-choice"
          aria-label="Reinforcements"
          aria-keyshortcuts={REINFORCEMENTS_KEY.label}
          aria-expanded={open}
          title={`Reinforcements (${REINFORCEMENTS_KEY.label})`}
          onClick={() => setOpen(!open)}
        >
          Reinforcements
        </button>
        <span className="hud-credits" aria-label="Credits">
          {Math.floor(viewingPlayer.credits)} CR
        </span>
      </div>
      {open && (
        <PurchaseMenu
          {...menu}
          enabled={enabled}
          match={match}
          player={viewingPlayer}
          onChoose={(variant) => {
            menu.onChoose(variant);
            setOpen(false);
          }}
          onClose={() => {
            setOpen(false);
            button.current?.focus();
          }}
        />
      )}
    </div>
  );
}

interface MenuProps extends Omit<PickerProps, "player"> {
  player: PlayerView;
  /** Escape (the field's once its filter is empty) closes the menu and
   *  returns focus to the button. */
  onClose: () => void;
}

/** One card per variant, each under its own group's tab; clicking one starts
 *  placing exactly that variant. */
function PurchaseMenu({
  cards,
  faction,
  match,
  player,
  onChoose,
  rules,
  onClose,
  enabled,
}: MenuProps) {
  const catalog = useSessionCatalog();
  const [group, setGroup] = useState<UnitGroup>(UNIT_GROUPS[0].id);
  const [hovered, setHovered] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const search = useRef<HTMLInputElement>(null);
  const infoId = useId();
  // The field waits for the key, so game keys stay the game's while the shop
  // is open; the key enters the field and, pressed in it, leaves it. Escape
  // anywhere but the field (which clears itself first) closes the shop ahead
  // of the pause menu. A menu over the shop takes both keys.
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    if (!enabled) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const inField = event.target === search.current;
      if (isShopTab(event)) {
        event.preventDefault();
        if (inField) search.current?.blur();
        else search.current?.focus();
      } else if (event.key === "Escape" && !event.repeat && !inField) {
        event.preventDefault();
        close.current();
      }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [enabled]);
  const facts: BuyFacts = {
    credits: player.credits,
    unitCount: player.unitCount,
    maxUnits: player.cap,
    finished: match.phase === "finished",
  };
  const available = cards.filter((c) => c.roster.factions.includes(faction));
  const searching = query.trim() !== "";
  // A search spans the roster; browsing shows the cards under the open tab.
  const typeOf = (id: string) => (catalog.units.has(id) ? catalog.units.type(id) : null);
  const shown = shopOrder(
    searching
      ? filterCards(available, query)
      : available.filter((c) => unitGroup(c, typeOf(c.id)) === group),
  );
  const info = shown.find((c) => c.id === hovered);
  // The info card hangs from its card's top-left corner, measured after every
  // render from the card itself, inside the picker's border, where absolute
  // positions start.
  const flyout = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const menu = flyout.current;
    const picker = menu?.closest<HTMLElement>(".hud-purchase-picker");
    const card = picker?.querySelector<HTMLElement>(`[data-card="${hovered}"]`);
    if (!menu || !picker || !card) return;
    const at = card.getBoundingClientRect();
    const box = picker.getBoundingClientRect();
    menu.style.left = `${at.left - box.left - picker.clientLeft}px`;
    menu.style.top = `${at.top - box.top - picker.clientTop}px`;
  });
  // Enter buys the sole match; with several shown, it no-ops.
  const enter = searching ? enterChoice(shown, facts) : null;
  // A tab is shown as soon as the pointer or focus reaches it; a click does
  // the same, for touch. Passing back over the shown tab changes nothing.
  const showGroup = (g: UnitGroup) => {
    if (searching || g === group) return;
    setGroup(g);
    setHovered(null);
  };
  return (
    <section
      className="hud-panel hud-purchase-picker"
      aria-label="Faction units"
      onMouseLeave={() => setHovered(null)}
    >
      <div
        className="hud-purchase-cards"
        role="tabpanel"
        aria-label={searching ? "Search results" : UNIT_GROUPS.find((g) => g.id === group)?.label}
      >
        {shown.map((card) => {
          const unsold = card.disabled_reason !== null;
          const purchasable = cardPurchasable(card, facts);
          return (
            <button
              key={card.id}
              type="button"
              className={`hud-army-card hud-purchase-card${unsold ? " unavailable" : purchasable ? "" : " short"}`}
              aria-label={card.name}
              aria-disabled={!purchasable}
              aria-pressed={card.id === hovered}
              aria-describedby={card.id === hovered ? infoId : undefined}
              data-card={card.id}
              // A click never takes the text cursor out of the field.
              onMouseDown={(event) => event.preventDefault()}
              onMouseEnter={() => setHovered(card.id)}
              onFocus={() => setHovered(card.id)}
              onClick={() => {
                setHovered(card.id);
                if (purchasable) onChoose(card.id);
              }}
            >
              {/* Every card has its generated silhouette, keyed by card id: a
                  unit type's from its model, a disabled card's from its source. */}
              <UnitFace kind={card.id}>
                <Price cost={card.cost} credits={facts.credits} unsold={unsold} />
              </UnitFace>
            </button>
          );
        })}
        {searching && shown.length === 0 && <p className="hud-purchase-empty">NO MATCH</p>}
      </div>
      {/* The tabs stand under the cards, so a card's info card, standing on
          it, never covers them. The filter stands at the row's right, beside
          the tabs, and dims them while it owns the search. */}
      <div className="hud-purchase-filter">
        <div className="hud-purchase-groups" role="tablist" aria-label="Unit groups">
          {UNIT_GROUPS.map(({ id, label }) => (
            <button
              key={id}
              type="button"
              role="tab"
              className="hud-menu-choice"
              disabled={searching}
              aria-selected={!searching && id === group}
              onMouseDown={(event) => event.preventDefault()}
              onMouseEnter={() => showGroup(id)}
              onFocus={() => showGroup(id)}
              onClick={() => showGroup(id)}
            >
              {label}
            </button>
          ))}
        </div>
        <label className="hud-purchase-search">
          <Icon path={hudIcon("search")} />
          <input
            ref={search}
            type="text"
            aria-label="Search units"
            placeholder="SEARCH"
            autoComplete="off"
            spellCheck={false}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && enter) {
                event.preventDefault();
                onChoose(enter.id);
              }
              if (event.key === "Escape") {
                // Keep the page's own Escape (the pause menu) out of a field
                // the player is using: clear first, then close.
                event.preventDefault();
                if (query) setQuery("");
                else onClose();
              }
            }}
          />
        </label>
      </div>
      {/* The hovered card's info card, standing on its card over the picker,
          so the picker never changes size: the deck's own info panel, with
          the price. It only shows; the pointer passes through it to the
          cards beneath. */}
      {info && (
        <div className="hud-purchase-flyout" ref={flyout}>
          <div
            id={infoId}
            role="tooltip"
            aria-label={info.name}
            className={`hud-panel hud-card hud-purchase-info${cardPurchasable(info, facts) ? "" : " short"}`}
          >
            <InfoPanel panel={purchasePanel(catalog, info, rules)} />
            <span className="hud-purchase-info-foot">
              {info.disabled_reason !== null && <small>Unavailable</small>}
              <Price
                cost={info.cost}
                credits={facts.credits}
                unsold={info.disabled_reason !== null}
              />
            </span>
          </div>
        </div>
      )}
    </section>
  );
}
