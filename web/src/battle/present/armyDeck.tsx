import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { carrierOf, type PendingPurchaseView, type OwnUnitView } from "../sim/observationView";
import { useSessionCatalog } from "../catalog/context";
import { StrengthMeter, UnitFace } from "./unitFace";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import { InfoPanel } from "./infoPanel";
import { ownPanel, reserveRow, unitStrength, type PanelRules } from "./panelRows";
import { unitName } from "./readouts";
import { CommandBar, type ArmyCommandState } from "./commandBar";
import { Feed, useFeedValue, type FeedSource } from "@web/feed";
import { GROUP_KEYS, type ControlGroups, type GroupKey } from "../input/controlGroups";
import { deckSlots } from "./deckLayout";
import { playBlink, type CardBlink } from "./fireBlink";

/** The unit a card's click selects: a rider's carrier, since a squad a
 *  carrier holds can't be ordered (D10), else the unit itself. */
function pickOf(unit: OwnUnitView, own: readonly OwnUnitView[]): number {
  const host = carrierOf(unit);
  return host !== null && own.some((u) => u.id === host) ? host : unit.id;
}

const NO_GROUPS: ControlGroups = new Map();
const NO_BLINKS: FeedSource<ReadonlyMap<number, CardBlink>> = new Feed(new Map());

const sameSet = (a: readonly number[], b: readonly number[]) =>
  a.length === b.length && a.every((id) => b.includes(id));

/** Own observations populate the army row; selection remains the input owner's.
 *  A click selects one card, Shift-click the run of cards from the last one
 *  clicked (the anchor), and Cmd- or Ctrl-click adds or removes one. A plain
 *  click on an already selected card asks the page to centre on it. Hovering
 *  a card hovers its unit, as the pointer over the unit would. A right-click
 *  asks the page to find that exact unit in the reinforcements store.
 *  Control groups lead the deck in key order, each under a bracket
 *  (`deckLayout.ts`) with its key, and its × while the pointer is over it.
 *  A lost unit's card holds its place, emptied and out of reach, while its
 *  red blink runs (`fireBlink.ts`). */
export function ArmyDeck({
  state,
  selected,
  onSelect,
  onCenter,
  onLookup,
  onHover,
  rules,
  contactLines,
  reinforcements,
  pending = [],
  tick = 0,
  groups = NO_GROUPS,
  onDeleteGroup,
  blinks: blinkFeed = NO_BLINKS,
}: {
  state: FeedSource<ArmyCommandState>;
  selected: readonly number[];
  onSelect: (ids: number[]) => void;
  /** A plain click on a card already in `selected`: show this unit on the map. */
  onCenter?: (unit: OwnUnitView) => void;
  /** A right-click on a card: find this exact unit in the reinforcements store. */
  onLookup?: (unit: OwnUnitView) => void;
  /** The unit the pointer is over by its card (a rider's, its carrier, as
   *  its click picks), or null when it leaves the card. */
  onHover?: (unit: number | null) => void;
  rules: PanelRules;
  contactLines: ReactNode;
  reinforcements?: ReactNode;
  pending?: PendingPurchaseView[];
  /** The observed tick, which a waiting aircraft's countdown runs from. */
  tick?: number;
  /** The player's control groups (`useUnitControl`). */
  groups?: ControlGroups;
  /** A group's × was clicked. */
  onDeleteGroup?: (key: GroupKey) => void;
  /** Each unit's latest fire blink (`fireBlink.ts`). */
  blinks?: FeedSource<ReadonlyMap<number, CardBlink>>;
}) {
  const { own, control } = useFeedValue(state);
  const blinks = useFeedValue(blinkFeed);
  const lower = useRef<HTMLDivElement>(null);
  // The card a run starts from, and the selection the deck last asked for:
  // once the selection changes any other way, the run starts over.
  const anchor = useRef<number | null>(null);
  // The cards last drawn, each with its group: a lost unit's card is drawn
  // from here, where it was, while its blink runs.
  const drawn = useRef<{ unit: OwnUnitView; group: GroupKey | null }[]>([]);
  const asked = useRef<readonly number[] | null>(null);
  const select = (ids: number[], from: number | null) => {
    asked.current = ids;
    if (from !== null) anchor.current = from;
    onSelect(ids);
  };
  const deck = useRef<HTMLElement>(null);
  const [hintHost, setHintHost] = useState<HTMLDivElement | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const [hoveredGroup, setHoveredGroup] = useState<GroupKey | null>(null);
  const [detailX, setDetailX] = useState(0);
  const [detailY, setDetailY] = useState(0);
  const tooltipId = useId();
  const detailCard = useRef<HTMLDivElement>(null);
  // Centred on its card, but never past the viewport's edge (8 px margin).
  useLayoutEffect(() => {
    const card = detailCard.current;
    if (!card) return;
    const half = card.offsetWidth / 2;
    const x = Math.min(Math.max(detailX, 8 + half), window.innerWidth - 8 - half);
    card.style.left = `${x}px`;
    lower.current!.style.setProperty("--hud-army-detail-height", `${card.offsetHeight}px`);
  });
  const { units } = useSessionCatalog();
  // A squad bought aboard a carrier waits as two cards: the carrier, then the
  // squad marked aboard it, as a rider on the map is.
  const waiting = pending.flatMap((purchase) =>
    purchase.carrier
      ? [
          { purchase, kind: purchase.carrier, aboard: null, key: `${purchase.id}-carrier` },
          { purchase, kind: purchase.kind, aboard: purchase.carrier, key: `${purchase.id}` },
        ]
      : [{ purchase, kind: purchase.kind, aboard: null, key: `${purchase.id}` }],
  );
  const present = new Set(own.map((u) => u.id));
  const lingering = drawn.current.flatMap((card, index) =>
    !present.has(card.unit.id) && blinks.get(card.unit.id)?.lost ? [{ ...card, index }] : [],
  );
  const [availableWidth, setAvailableWidth] = useState(() =>
    typeof window === "undefined" ? 1280 : window.innerWidth,
  );
  useLayoutEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    const node = lower.current;
    if (!node) return;
    const observer = new ResizeObserver(([entry]) => setAvailableWidth(entry.contentRect.width));
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  // Groups first, 1 to 9 then 0, each in army order; then the rest.
  const groupOf = new Map<number, GroupKey>();
  for (const key of GROUP_KEYS) for (const id of groups.get(key) ?? []) groupOf.set(id, key);
  const cards = [
    ...GROUP_KEYS.flatMap((key) => own.filter((u) => groupOf.get(u.id) === key)),
    ...own.filter((u) => !groupOf.has(u.id)),
  ];
  for (const { unit, group, index } of lingering) {
    cards.splice(Math.min(index, cards.length), 0, unit);
    if (group) groupOf.set(unit.id, group);
  }
  useLayoutEffect(() => {
    drawn.current = cards.map((unit) => ({ unit, group: groupOf.get(unit.id) ?? null }));
  });
  const unitCount = own.length + waiting.length;
  const count = cards.length + waiting.length;
  const capacity = Math.max(1, Math.floor((availableWidth - 10 + 4) / 68));
  const rows = Math.max(1, Math.ceil(count / capacity));
  const slots = deckSlots(
    [...cards.map((u) => groupOf.get(u.id) ?? null), ...waiting.map(() => null)],
    rows,
  );
  /** A card in its grid cell, with its group's bracket, key and ×. */
  const slot = (key: string | number, index: number, children: ReactNode, blink?: CardBlink) => {
    const { row, column, group, sides, keyTab, deleteTab } = slots[index];
    return (
      <div
        key={key}
        className="hud-army-slot"
        style={{ gridRow: row + 1, gridColumn: column + 1 }}
        data-group={group ?? undefined}
        data-bracket={sides.length ? sides.join(" ") : undefined}
        onMouseEnter={() => setHoveredGroup(group)}
      >
        {blink && (
          // Behind the card's face, so its strength meter and symbols stay
          // clean; a new serial is a new element, so the animation restarts.
          <span
            key={blink.serial}
            ref={playBlink}
            className="hud-army-blink"
            data-colour={blink.colour}
            aria-hidden="true"
          />
        )}
        {children}
        {keyTab && (
          <span className="hud-army-group-key" aria-hidden="true">
            {group}
          </span>
        )}
        {deleteTab && group && hoveredGroup === group && onDeleteGroup && (
          <button
            type="button"
            className="hud-army-group-delete"
            aria-label={`Delete group ${group}`}
            title={`Delete group ${group}`}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onDeleteGroup(group)}
          >
            <Icon path={hudIcon("close")} />
          </button>
        )}
      </div>
    );
  };
  const detail = own.find((unit) => unit.id === hovered);
  const alignDetail = (button: HTMLButtonElement) => {
    // Both popups anchor to the deck; contact lines above it must not inflate the gap.
    const box = button.getBoundingClientRect();
    setDetailX(box.left + box.width / 2);
    setDetailY(deck.current!.getBoundingClientRect().top);
  };
  return (
    <div className="hud-lower" ref={lower}>
      {contactLines}
      {detail && (
        <div className="hud-army-detail-host">
          <div
            className="hud-panel hud-card hud-army-detail"
            role="tooltip"
            id={tooltipId}
            data-unit={detail.id}
            data-occludes-readouts
            ref={detailCard}
            style={{ left: detailX, top: detailY }}
          >
            <InfoPanel
              panel={{
                ...ownPanel(units, detail, own, rules),
                name: `${unitName(units, detail)} #${detail.id}`,
              }}
            />
          </div>
        </div>
      )}
      <div className="hud-command-hint" ref={setHintHost} />
      {(cards.length > 0 || pending.length > 0 || reinforcements) && (
        <footer
          ref={deck}
          className="hud-panel hud-bar hud-bottom hud-army-deck"
          data-occludes-readouts
        >
          <div className="hud-army-header">
            {reinforcements}
            <span className="hud-army-count" aria-label="Total units">
              {unitCount} {unitCount === 1 ? "UNIT" : "UNITS"}
            </span>
          </div>
          <div
            className="hud-army"
            data-unit-count={count}
            data-army-rows={rows}
            style={{ gridTemplateRows: `repeat(${rows}, auto)` }}
            role="group"
            aria-label="Your units"
            onMouseLeave={() => setHoveredGroup(null)}
          >
            {cards.map((unit, index) => {
              const name = `${unitName(units, unit)} #${unit.id}`;
              const lost = !present.has(unit.id);
              const strength = lost ? 0 : unitStrength(units, unit);
              // A squad a carrier holds is still the player's, but can't be
              // ordered (D10): its card is dimmed and marked with the seat it
              // rides in, and picks its carrier.
              const host = carrierOf(unit);
              const carrier = host === null ? undefined : own.find((u) => u.id === host);
              // One state mark per card: a riding squad's seats and a unit on
              // its way home already own the corner, so a reserve waits for a
              // plain card. The mark is the unit panel's own reserve row.
              const mark =
                !lost && !carrier && !unit.withdrawing ? reserveRow(units, unit, rules) : null;
              const picks = pickOf(unit, own);
              const selectedHere = selected.includes(picks);
              const group = groupOf.get(unit.id);
              return slot(
                unit.id,
                index,
                <button
                  type="button"
                  className="hud-army-card"
                  data-unit={unit.id}
                  disabled={lost}
                  data-aboard={carrier ? "true" : undefined}
                  data-withdrawing={unit.withdrawing ? "true" : undefined}
                  aria-label={
                    (lost
                      ? `${name}, lost`
                      : carrier
                        ? `${name}, aboard ${unitName(units, carrier)} #${carrier.id}`
                        : unit.withdrawing
                          ? `${name}, returning to base`
                          : name) +
                    (mark ? `, ${mark.word.toLowerCase()}` : "") +
                    (group ? `, group ${group}` : "")
                  }
                  aria-pressed={selectedHere}
                  aria-describedby={detail?.id === unit.id ? tooltipId : undefined}
                  onClick={(event) => {
                    const from = cards.findIndex((u) => u.id === anchor.current);
                    const run =
                      event.shiftKey &&
                      from >= 0 &&
                      asked.current &&
                      sameSet(asked.current, selected);
                    if (run) {
                      const [a, b] = from < index ? [from, index] : [index, from];
                      const picked = cards.slice(a, b + 1).map((u) => pickOf(u, own));
                      select([...new Set(picked)], null);
                    } else if (event.metaKey || event.ctrlKey) {
                      select(
                        selectedHere ? selected.filter((id) => id !== picks) : [...selected, picks],
                        unit.id,
                      );
                    } else select([picks], unit.id);
                    // A double-click lands here twice: the first selects the card,
                    // the second sees it selected and centres. Modified clicks keep
                    // their selection meaning and never move the camera.
                    if (
                      selectedHere &&
                      !event.shiftKey &&
                      !event.ctrlKey &&
                      !event.metaKey &&
                      !event.altKey
                    )
                      // A rider's card picks its carrier, so centre on the
                      // selected unit rather than the passenger.
                      onCenter?.(carrier ?? unit);
                  }}
                  // A click leaves keyboard focus where it was, so Enter and
                  // Space stay the game's rather than clicking the card again.
                  onMouseDown={(event) => event.preventDefault()}
                  // The browser's menu would cover the store's own lookup.
                  onContextMenu={(event) => {
                    if (!onLookup) return;
                    event.preventDefault();
                    onLookup(unit);
                  }}
                  onMouseEnter={(event) => {
                    if (lost) return;
                    alignDetail(event.currentTarget);
                    setHovered(unit.id);
                    onHover?.(picks);
                  }}
                  onMouseLeave={() => {
                    setHovered(null);
                    onHover?.(null);
                  }}
                >
                  <UnitFace kind={unit.kind}>
                    <StrengthMeter name={name} strength={strength} />
                  </UnitFace>
                  {carrier && <Icon path={stateIcon("seats")} className="hud-army-aboard" />}
                  {unit.withdrawing && (
                    <Icon path={stateIcon("withdrawing")} className="hud-army-withdrawing" />
                  )}
                  {mark && <Icon path={mark.icon} className="hud-army-reserve" title={mark.word} />}
                </button>,
                blinks.get(unit.id),
              );
            })}
            {waiting.map(({ purchase: unit, kind, aboard, key }, index) => {
              const name = aboard
                ? `${units.type(kind).name}, aboard ${units.type(aboard).name}`
                : units.type(kind).name;
              // A bought aircraft waiting off the map counts down to its
              // arrival (D42); anything else waits on the queue.
              const eta =
                unit.arrivalTick > 0
                  ? Math.max(0, Math.ceil((unit.arrivalTick - tick) / rules.tick_hz))
                  : null;
              const state =
                eta !== null
                  ? `Arriving in ${eta} s`
                  : unit.blocked
                    ? "Entry blocked"
                    : "Will be deployed";
              return slot(
                `pending-${key}`,
                cards.length + index,
                <button
                  type="button"
                  className={`hud-army-card hud-army-pending${eta !== null ? " hud-army-arriving" : ""}`}
                  disabled
                  data-aboard={aboard ? "true" : undefined}
                  aria-label={`${name} — ${state}`}
                  title={`${name} — ${state}`}
                >
                  <UnitFace kind={kind}>
                    {eta !== null ? (
                      <span className="hud-army-eta">
                        <span>Arriving in</span> <b>{`${eta}\u2009s`}</b>
                      </span>
                    ) : (
                      <StrengthMeter name={name} strength={1} />
                    )}
                  </UnitFace>
                  {aboard && <Icon path={stateIcon("seats")} className="hud-army-aboard" />}
                </button>,
              );
            })}
          </div>
          {control && <CommandBar state={state} hintHost={hintHost} />}
        </footer>
      )}
    </div>
  );
}
