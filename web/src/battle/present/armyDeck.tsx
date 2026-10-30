import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { carrierOf, type PendingPurchaseView, type OwnUnitView } from "../sim/observationView";
import { useSessionCatalog } from "../catalog/context";
import { StrengthMeter, UnitFace } from "./unitFace";
import { stateIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import { InfoPanel } from "./infoPanel";
import { ownPanel, unitStrength, type PanelRules } from "./panelRows";
import { unitName } from "./readouts";
import { CommandBar, type ArmyCommandState } from "./commandBar";
import { useFeedValue, type FeedSource } from "@web/feed";

/** Own observations populate the army row; selection remains the input owner's,
 *  and a plain click on an already selected card asks the page to centre on it. */
export function ArmyDeck({
  state,
  selected,
  onSelect,
  onCenter,
  rules,
  captions,
  reinforcements,
  pending = [],
  tick = 0,
}: {
  state: FeedSource<ArmyCommandState>;
  selected: readonly number[];
  onSelect: (ids: number[]) => void;
  /** A plain click on a card already in `selected`: show this unit on the map. */
  onCenter?: (unit: OwnUnitView) => void;
  rules: PanelRules;
  captions: ReactNode;
  reinforcements?: ReactNode;
  pending?: PendingPurchaseView[];
  /** The observed tick, which a waiting aircraft's countdown runs from. */
  tick?: number;
}) {
  const { own, control } = useFeedValue(state);
  const lower = useRef<HTMLDivElement>(null);
  const deck = useRef<HTMLElement>(null);
  const [hintHost, setHintHost] = useState<HTMLDivElement | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const [focused, setFocused] = useState<number | null>(null);
  const [dismissed, setDismissed] = useState<number | null>(null);
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
  const count = own.length + pending.length;
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
  const capacity = Math.max(1, Math.floor((availableWidth - 10 + 4) / 68));
  const rows = Math.max(1, Math.ceil(count / capacity));
  const columns = Math.max(1, Math.ceil(count / rows));
  const lastRow = count % columns;
  const cardOffset = (index: number) =>
    lastRow && index >= count - lastRow
      ? { transform: `translateX(${(columns - lastRow) * 34}px)` }
      : undefined;
  const active = hovered ?? focused;
  const detail = active === dismissed ? undefined : own.find((unit) => unit.id === active);
  const alignDetail = (button: HTMLButtonElement) => {
    // Both popups anchor to the deck; captions above it must not inflate the gap.
    const box = button.getBoundingClientRect();
    setDetailX(box.left + box.width / 2);
    setDetailY(deck.current!.getBoundingClientRect().top);
    setDismissed(null);
  };
  return (
    <div className="hud-lower" ref={lower}>
      {captions}
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
      {(own.length > 0 || pending.length > 0 || reinforcements) && (
        <footer
          ref={deck}
          className="hud-panel hud-bar hud-bottom hud-army-deck"
          data-occludes-readouts
        >
          {reinforcements}
          <div
            className="hud-army"
            data-unit-count={count}
            data-army-rows={rows}
            style={{ gridTemplateColumns: `repeat(${columns}, 64px)` }}
            role="group"
            aria-label="Your units"
          >
            {own.map((unit, index) => {
              const name = `${unitName(units, unit)} #${unit.id}`;
              const strength = unitStrength(units, unit);
              // A squad a carrier holds is still the player's, but can't be
              // ordered (D10): its card is dimmed and marked with the seat it
              // rides in, and picks its carrier.
              const host = carrierOf(unit);
              const carrier = host === null ? undefined : own.find((u) => u.id === host);
              const picks = carrier?.id ?? unit.id;
              const selectedHere = selected.includes(picks);
              return (
                <button
                  type="button"
                  key={unit.id}
                  className="hud-army-card"
                  style={cardOffset(index)}
                  data-unit={unit.id}
                  data-aboard={carrier ? "true" : undefined}
                  aria-label={
                    carrier ? `${name}, aboard ${unitName(units, carrier)} #${carrier.id}` : name
                  }
                  aria-pressed={selectedHere}
                  aria-describedby={detail?.id === unit.id ? tooltipId : undefined}
                  onClick={(event) => {
                    onSelect(
                      event.shiftKey
                        ? selectedHere
                          ? selected.filter((id) => id !== picks)
                          : [...selected, picks]
                        : [picks],
                    );
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
                  // Mouse selection keeps details tied to hover; Tab still offers focus details.
                  onMouseDown={(event) => event.preventDefault()}
                  onMouseEnter={(event) => {
                    alignDetail(event.currentTarget);
                    setHovered(unit.id);
                  }}
                  onMouseLeave={() => {
                    setHovered(null);
                    const button = lower.current!.querySelector<HTMLButtonElement>(
                      `button[data-unit="${focused}"]`,
                    );
                    if (button) alignDetail(button);
                  }}
                  onFocus={(event) => {
                    setHovered(null);
                    event.currentTarget.scrollIntoView({ block: "nearest", inline: "nearest" });
                    alignDetail(event.currentTarget);
                    setFocused(unit.id);
                  }}
                  onBlur={() => setFocused(null)}
                  onKeyDown={(event) => {
                    if (event.key === "Escape") setDismissed(active);
                  }}
                >
                  <UnitFace kind={unit.kind}>
                    <StrengthMeter name={name} strength={strength} />
                  </UnitFace>
                  {carrier && <Icon path={stateIcon("seats")} className="hud-army-aboard" />}
                </button>
              );
            })}
            {pending.map((unit, index) => {
              const name = units.type(unit.kind).name;
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
              return (
                <button
                  key={`pending-${unit.id}`}
                  type="button"
                  className={`hud-army-card hud-army-pending${eta !== null ? " hud-army-arriving" : ""}`}
                  style={cardOffset(own.length + index)}
                  disabled
                  aria-label={`${name} — ${state}`}
                  title={`${name} — ${state}`}
                >
                  <UnitFace kind={unit.kind}>
                    {eta !== null ? (
                      <span className="hud-army-eta">
                        <span>Arriving in</span> <b>{`${eta}\u2009s`}</b>
                      </span>
                    ) : (
                      <StrengthMeter name={name} strength={1} />
                    )}
                  </UnitFace>
                </button>
              );
            })}
          </div>
          {control && <CommandBar state={state} hintHost={hintHost} />}
        </footer>
      )}
    </div>
  );
}
