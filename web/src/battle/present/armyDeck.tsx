import { useId, useRef, useState, type ReactNode } from "react";
import type { OwnUnitView } from "../sim/observation";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
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
}: {
  own: readonly OwnUnitView[];
  selected: readonly number[];
  onSelect: (ids: number[]) => void;
  rules: PanelRules;
  control?: Parameters<typeof CommandBar>[0]["control"];
  captions: ReactNode;
}) {
  const lower = useRef<HTMLDivElement>(null);
  const [hintHost, setHintHost] = useState<HTMLDivElement | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  const [focused, setFocused] = useState<number | null>(null);
  const [dismissed, setDismissed] = useState<number | null>(null);
  const [detailX, setDetailX] = useState(0);
  const tooltipId = useId();
  const active = hovered ?? focused;
  const detail = active === dismissed ? undefined : own.find((unit) => unit.id === active);
  const alignDetail = (button: HTMLButtonElement) => {
    const box = button.getBoundingClientRect();
    const bounds = lower.current!.getBoundingClientRect();
    setDetailX(Math.max(180, Math.min(bounds.width - 180, box.left + box.width / 2 - bounds.left)));
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
            style={{ left: detailX }}
          >
            <InfoPanel
              panel={{ ...ownPanel(detail, own, rules), name: `${unitName(detail)} #${detail.id}` }}
            />
          </div>
        </div>
      )}
      <div className="hud-command-hint" ref={setHintHost} />
      {own.length > 0 && (
        <footer className="hud-panel hud-bar hud-bottom hud-army-deck" data-occludes-readouts>
          <div
            className="hud-army"
            role="group"
            aria-label="Your units"
            onScroll={(event) => {
              setHovered(null);
              const button = event.currentTarget.querySelector<HTMLButtonElement>(
                `button[data-unit="${focused}"]`,
              );
              if (button) alignDetail(button);
            }}
          >
            {own.map((unit) => {
              const { role, silhouette } = unitIcons(UNITS.type(unit.kind));
              const name = `${unitName(unit)} #${unit.id}`;
              const strength = unitStrength(unit);
              return (
                <button
                  type="button"
                  key={unit.id}
                  className="hud-army-card"
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
                  <span className="hud-army-id" aria-hidden="true">
                    {unit.id}
                  </span>
                  <Icon path={role} className="hud-army-role" />
                  <Icon path={silhouette} className="hud-army-silhouette" />
                  <span
                    className="hud-army-health"
                    role="meter"
                    aria-label={`${name} health`}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={Math.round(strength * 100)}
                  >
                    <span style={{ width: `${strength * 100}%` }} />
                  </span>
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
