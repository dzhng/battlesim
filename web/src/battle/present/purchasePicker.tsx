import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { Faction, UnitCard } from "@packages/scene-assets/src/units";
import { hudIcon } from "@packages/scene-assets/src/icons";
import type { Vec2 } from "math";
import { triangle2 } from "math/shapes";
import { isTypingTarget } from "../input/keyGuard";
import { useSessionCatalog } from "../catalog/context";
import { Icon } from "./icons";
import { purchasePanel, type PanelRules } from "./panelRows";
import { PurchaseInfo, ShopCard } from "./purchaseCard";
import {
  cardPurchasable,
  carrierSections,
  enterChoice,
  filterCards,
  shopOrder,
  UNIT_GROUPS,
  unitGroup,
  type BuyFacts,
  type UnitGroup,
} from "./purchaseFilter";
import {
  ON_FOOT,
  optionCarrier,
  optionPurchasable,
  TransportMenu,
  transportOptions,
} from "./transportMenu";
import type { PlayerView, SkirmishView } from "../sim/observationView";

/** A request to show one exact card in the store, from a unit card's
 *  right-click. `serial` changes on every request, so asking for the same
 *  unit twice opens it twice. */
export interface PurchaseLookup {
  card: string;
  serial: number;
}

interface PickerProps {
  cards: readonly UnitCard[];
  faction: Faction;
  match: SkirmishView;
  player?: PlayerView;
  /** Starts placing `variant`; a squad bought in a ride names its `carrier`. */
  onChoose: (variant: string, carrier?: string) => void;
  /** What a purchase's info card is built with, as the deck's are. */
  rules: PanelRules;
  /** Whether the shop's key answers: off while a menu or result covers it. */
  enabled?: boolean;
  /** A unit card was right-clicked: open the store on this exact variant. */
  lookup?: PurchaseLookup | null;
  /** The request was taken: the page clears it, so a remount cannot replay it. */
  onLookupHandled?: () => void;
}
/** How long an open transport menu holds while the pointer heads into it
 *  across another card. Past it, the card the pointer stopped on takes over;
 *  a pointer moving any other way switches at once. */
const MENU_AIM_MS = 300;
/** Pointer positions kept to read its heading: few enough to follow a turn,
 *  enough to steady a shaky hand. */
const HEADING_SAMPLES = 3;

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
 *  while open, so every plain opening starts fresh: first group, empty
 *  filter. A lookup instead opens on its exact card. */
export function PurchasePicker({
  match,
  player,
  enabled = true,
  lookup,
  onLookupHandled,
  ...menu
}: PickerProps) {
  const viewingPlayer = player ?? match.players[0];
  const [open, setOpen] = useState(false);
  const [reveal, setReveal] = useState<PurchaseLookup | null>(null);
  const seenLookup = useRef(0);
  const button = useRef<HTMLButtonElement>(null);
  // A unit card's right-click is one request: open the store on that exact
  // card, and leave it there. Closing clears it, so the button opens fresh.
  useEffect(() => {
    if (!lookup || lookup.serial === seenLookup.current) return;
    seenLookup.current = lookup.serial;
    onLookupHandled?.();
    setReveal(lookup);
    setOpen(true);
  }, [lookup, onLookupHandled]);
  const close = () => {
    setOpen(false);
    setReveal(null);
  };
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
  if (!viewingPlayer) return null;
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
          onClick={() => {
            if (open) close();
            else {
              setReveal(null);
              setOpen(true);
            }
          }}
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
          key={reveal?.serial ?? "browse"}
          reveal={reveal}
          enabled={enabled}
          match={match}
          player={viewingPlayer}
          onChoose={(...choice) => {
            menu.onChoose(...choice);
            close();
          }}
          onClose={() => {
            close();
            button.current?.focus();
          }}
        />
      )}
    </div>
  );
}

interface MenuProps extends Omit<PickerProps, "player"> {
  player: PlayerView;
  /** The exact card a unit card's right-click asked for, if any. */
  reveal?: PurchaseLookup | null;
  /** Escape (the field's once its filter is empty) closes the menu and
   *  returns focus to the button. */
  onClose: () => void;
}

/** One card per variant, each under its own group's tab; clicking one starts
 *  placing exactly that variant. Hovering a squad opens its transport menu. */
function PurchaseMenu({
  cards,
  faction,
  match,
  player,
  onChoose,
  rules,
  onClose,
  enabled,
  reveal,
}: MenuProps) {
  const catalog = useSessionCatalog();
  const available = cards.filter((c) => c.roster.factions.includes(faction));
  // A planned card has no type: it is neither a squad nor a carrier.
  const typed = (id: string) => catalog.units.has(id);
  const groupOf = (c: UnitCard) => unitGroup(c, typed(c.id) ? catalog.units.type(c.id) : null);
  // A lookup starts on the exact card's group and shows that one card, with
  // its info card open. Its filter holds the full name so the state explains
  // itself; the id is what keeps the result exact.
  const revealed = reveal ? (available.find((c) => c.id === reveal.card) ?? null) : null;
  const [group, setGroup] = useState<UnitGroup>(revealed ? groupOf(revealed) : UNIT_GROUPS[0].id);
  const [hovered, setHovered] = useState<string | null>(revealed?.id ?? null);
  // An explicitly hovered or keyboard-selected transport option, for its squad.
  const [pick, setPick] = useState<{ squad: string; option: string } | null>(null);
  const [query, setQuery] = useState(
    revealed ? `${revealed.roster.family_name} ${revealed.roster.variant}` : "",
  );
  /** The exact card a lookup holds, until the player edits the filter. */
  const [exact, setExact] = useState<string | null>(revealed?.id ?? null);
  const search = useRef<HTMLInputElement>(null);
  // The card a heading-in pointer crossed, waiting to take over the menu.
  const crossed = useRef<string | null>(null);
  const crossTimer = useRef<number | undefined>(undefined);
  const trail = useRef<Vec2[]>([]);
  const transport = useRef<HTMLElement>(null);
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
  useEffect(() => () => window.clearTimeout(crossTimer.current), []);
  const facts: BuyFacts = {
    credits: player.credits,
    unitCount: player.unitCount,
    maxUnits: player.cap,
    finished: match.phase === "finished",
  };
  const searching = query.trim() !== "" || exact !== null;
  // A search spans the roster; browsing shows the cards under the open tab.
  // A lookup is the one exact card, by id rather than by a name another
  // variant might share.
  const squadSize = (c: UnitCard) => (typed(c.id) ? catalog.units.squadSize(c.id) : 0);
  const shown = shopOrder(
    exact
      ? available.filter((c) => c.id === exact)
      : searching
        ? filterCards(available, query)
        : available.filter((c) => groupOf(c) === group),
  );
  const hoveredCard = shown.find((c) => c.id === hovered) ?? null;
  // A squad on sale has a transport menu: the hovered one's, or with nothing
  // hovered, the one squad a search narrows to.
  const sole = searching && shown.length === 1 ? shown[0] : null;
  const candidate = hoveredCard ?? sole;
  const squad =
    candidate && squadSize(candidate) > 0 && candidate.disabled_reason === null ? candidate : null;
  const sections = squad
    ? carrierSections(
        squadSize(squad),
        available,
        (c) => (typed(c.id) ? catalog.units.seats(c.id) : 0),
        groupOf,
      )
    : [];
  // Pointing at the base card previews buying the squad on foot.
  const option =
    squad && pick?.squad === squad.id
      ? pick.option
      : squad && squad === hoveredCard
        ? ON_FOOT
        : null;
  // A card without a transport menu shows its one info card.
  const info = squad ? null : hoveredCard;
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
  // Buys a transport option, when it can be bought now.
  const buy = (rider: UnitCard, option: string) => {
    const carrier = optionCarrier(sections, option);
    if (!optionPurchasable(rider, carrier, facts)) return;
    if (carrier) onChoose(rider.id, carrier.id);
    else onChoose(rider.id);
  };
  // While a transport menu is open the arrow keys step its highlight, the
  // camera's no longer: on foot, then each section cheapest first, round.
  const step = useRef<(by: 1 | -1) => void>(() => {});
  step.current = (by) => {
    if (!squad) return;
    const options = transportOptions(sections);
    const at = option === null ? (by > 0 ? -1 : options.length) : options.indexOf(option);
    const next = options[(at + by + options.length) % options.length];
    setPick({ squad: squad.id, option: next });
  };
  const menuOpen = enabled && squad !== null;
  useEffect(() => {
    if (!menuOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
      event.preventDefault();
      event.stopPropagation();
      step.current(event.key === "ArrowDown" ? 1 : -1);
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [menuOpen]);
  // Enter buys the sole match; a squad's, in the highlighted ride or on foot.
  // With several shown, it no-ops.
  const enter = () => {
    if (squad && squad === sole) buy(squad, option ?? ON_FOOT);
    else {
      const choice = searching ? enterChoice(shown, facts) : null;
      if (choice) onChoose(choice.id);
    }
  };
  // A crossed card stops waiting: the pointer reached the menu or moved on.
  const cancelCrossing = () => {
    window.clearTimeout(crossTimer.current);
    crossed.current = null;
  };
  // Returning to a base card clears a ride preview, including in its own menu.
  const point = (id: string | null) => {
    cancelCrossing();
    setPick(null);
    if (id === hovered) return;
    setHovered(id);
  };
  // Whether the pointer is heading into the open transport menu: where it is
  // now lies in the triangle from where it was a few moves back to the
  // menu's bottom corners. The menu stands on its card, so heading in is
  // heading up; a pointer that hasn't moved heads nowhere.
  const headingIn = () => {
    const box = transport.current?.getBoundingClientRect();
    const from = trail.current[0];
    const at = trail.current[trail.current.length - 1];
    if (!box || !from || !at || (from[0] === at[0] && from[1] === at[1])) return false;
    return triangle2.containsPoint(from, [box.left, box.bottom], [box.right, box.bottom], at);
  };
  // Another card under a pointer heading into the open transport menu waits
  // for the pointer to stop there or turn away; anything else follows at once.
  const hoverCard = (id: string) => {
    if (squad && squad.id === hovered && id !== hovered && headingIn()) {
      window.clearTimeout(crossTimer.current);
      crossed.current = id;
      crossTimer.current = window.setTimeout(() => point(id), MENU_AIM_MS);
    } else point(id);
  };
  // Every move reads the pointer's heading; one that turns away from the
  // menu hands it to the card it crossed.
  const move = (x: number, y: number) => {
    trail.current = [...trail.current.slice(1 - HEADING_SAMPLES), [x, y]];
    if (crossed.current && !headingIn()) point(crossed.current);
  };
  // A tab is shown as soon as the pointer or focus reaches it; a click does
  // the same, for touch. Passing back over the shown tab changes nothing.
  const showGroup = (g: UnitGroup) => {
    if (searching || g === group) return;
    setGroup(g);
    point(null);
  };
  return (
    <section
      className="hud-panel hud-purchase-picker"
      aria-label="Faction units"
      onMouseMove={(event) => move(event.clientX, event.clientY)}
      onMouseLeave={() => {
        trail.current = [];
        point(null);
      }}
    >
      <div
        className="hud-purchase-cards"
        role="tabpanel"
        aria-label={searching ? "Search results" : UNIT_GROUPS.find((g) => g.id === group)?.label}
      >
        {shown.map((card) => (
          <ShopCard
            key={card.id}
            face={card}
            label={card.name}
            cost={card.cost}
            payable={card.cost <= facts.credits}
            purchasable={cardPurchasable(card, facts)}
            pressed={card.id === hovered}
            describedBy={card === info ? infoId : undefined}
            data={{ "data-card": card.id }}
            onHover={() => hoverCard(card.id)}
            onChoose={() => onChoose(card.id)}
          />
        ))}
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
        <label className="hud-purchase-search" data-lookup={exact ? "true" : undefined}>
          <Icon path={hudIcon("search")} />
          <input
            ref={search}
            type="text"
            aria-label="Search units"
            placeholder="SEARCH"
            autoComplete="off"
            spellCheck={false}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setExact(null);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                enter();
              }
              if (event.key === "Escape") {
                // Keep the page's own Escape (the pause menu) out of a field
                // the player is using: clear first, then close.
                event.preventDefault();
                if (searching) {
                  setQuery("");
                  setExact(null);
                  point(null);
                } else onClose();
              }
            }}
          />
        </label>
      </div>
      {/* The hovered card's info card, standing on its card over the picker,
          so the picker never changes size. */}
      {info && (
        <div className="hud-purchase-flyout" ref={flyout}>
          <PurchaseInfo
            id={infoId}
            card={info}
            panel={purchasePanel(catalog, info, rules)}
            credits={facts.credits}
            short={!cardPurchasable(info, facts)}
          />
        </div>
      )}
      {squad && (
        <TransportMenu
          key={squad.id}
          squad={squad}
          sections={sections}
          facts={facts}
          rules={rules}
          option={option}
          onOption={(option) => setPick({ squad: squad.id, option })}
          onBuy={(option) => buy(squad, option)}
          frame={transport}
          onEnter={cancelCrossing}
        />
      )}
    </section>
  );
}
