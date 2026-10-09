import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { SkirmishView, OwnUnitView } from "../sim/observation";
import { useSessionCatalog } from "../catalog/context";
import { StrengthMeter, UnitFace } from "./unitFace";
import { InfoPanel } from "./infoPanel";
import { ownPanel, unitStrength, type PanelRules } from "./panelRows";
import { CommandBar, unitName } from "./readouts";

/** Own observations populate the army row; selection remains the input owner's. */
export function ArmyDeck({
  own,
  selected,
  onSelect,
  rules,
  control,
  captions,
  reinforcements,
  pending = [],
}: {
  own: readonly OwnUnitView[];
  selected: readonly number[];
  onSelect: (ids: number[]) => void;
  rules: PanelRules;
  control?: Parameters<typeof CommandBar>[0]["control"];
  captions: ReactNode;
  reinforcements?: ReactNode;
  pending?: SkirmishView["pending"];
}) {
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
              return (
                <button
                  type="button"
                  key={unit.id}
                  className="hud-army-card"
                  style={cardOffset(index)}
                  data-unit={unit.id}
                  aria-label={name}
                  aria-pressed={selected.includes(unit.id)}
                  aria-describedby={detail?.id === unit.id ? tooltipId : undefined}
                  onClick={(event) =>
                    onSelect(
                      event.shiftKey
                        ? selected.includes(unit.id)
                          ? selected.filter((id) => id !== unit.id)
                          : [...selected, unit.id]
                        : [unit.id],
                    )
                  }
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
                </button>
              );
            })}
            {pending.map((unit, index) => {
              const name = units.type(unit.kind).name;
              return (
                <button
                  key={`pending-${unit.id}`}
                  type="button"
                  className="hud-army-card hud-army-pending"
                  style={cardOffset(own.length + index)}
                  disabled
                  aria-label={`${name} — Will be deployed`}
                  title={`${name} — ${unit.blocked ? "Entry blocked" : "Will be deployed"}`}
                >
                  <UnitFace kind={unit.kind}>
                    <StrengthMeter name={name} strength={1} />
                  </UnitFace>
                </button>
              );
            })}
          </div>
          {control && <CommandBar control={control} hintHost={hintHost} />}
        </footer>
      )}
    </div>
  );
}
