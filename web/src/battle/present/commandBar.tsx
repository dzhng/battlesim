/** Selection commands and capability availability, from the side's own observation. */
import { useId, useState } from "react";
import { createPortal } from "react-dom";
import { useSessionCatalog } from "../catalog/context";
import type { OwnUnitView } from "../sim/observationView";
import type { useUnitControl } from "../input/useUnitControl";
import { useFeedValue, type FeedSource } from "@web/feed";
import type { CommandMode } from "../input/pointerIntent";
import { CommandBindings } from "../input/commandBindings";
import { reach, type ReachCommand } from "../input/commandReach";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";

interface CommandProps {
  icon: string;
  label: string;
  pressed?: boolean;
  disabled?: boolean;
  emphasis?: boolean;
  onClick: () => void;
}

function CommandButton({
  icon,
  label,
  pressed,
  disabled,
  emphasis,
  onClick,
  hintHost,
}: CommandProps & { hintHost?: HTMLElement | null }) {
  const id = useId();
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [hintCenter, setHintCenter] = useState(0);
  const alignHint = (button: HTMLButtonElement) => {
    if (!hintHost) return;
    const owner = button.getBoundingClientRect();
    // The empty hint host is hidden; its fixed layout parent still has bounds.
    const lower = hintHost.parentElement!.getBoundingClientRect();
    setHintCenter(owner.left + owner.width / 2 - lower.left);
  };
  const shown = (hovered || focused) && !dismissed;
  const tooltip = shown ? (
    <span
      className="ro-cmd-tooltip"
      role="tooltip"
      id={id}
      style={hintHost ? { left: hintCenter } : undefined}
    >
      {label}
    </span>
  ) : null;
  return (
    <button
      type="button"
      className={`ro-cmd${emphasis ? " ro-cmd-emphasis" : ""}`}
      aria-label={label}
      aria-describedby={shown ? id : undefined}
      aria-pressed={pressed}
      disabled={disabled}
      onClick={onClick}
      onMouseEnter={(event) => {
        alignHint(event.currentTarget);
        setHovered(true);
        setDismissed(false);
      }}
      onMouseLeave={() => setHovered(false)}
      onFocus={(event) => {
        alignHint(event.currentTarget);
        setFocused(true);
        setDismissed(false);
      }}
      onBlur={() => setFocused(false)}
      onKeyDown={(event) => {
        if (event.key === "Escape") setDismissed(true);
      }}
    >
      <Icon path={icon} className="ro-cmd-icon" />
      {hintHost ? createPortal(tooltip, hintHost) : tooltip}
    </button>
  );
}

export type CommandControl = Pick<
  ReturnType<typeof useUnitControl>,
  | "mode"
  | "setMode"
  | "selectedUnits"
  | "stop"
  | "togglePolicy"
  | "toggleDeployment"
  | "exitBuilding"
  | "pressUnload"
> & { refund?: () => void; toggleAfterburner?: () => void };

export interface ArmyCommandState {
  control?: CommandControl;
  /** Every own unit: the passengers Unload reaches carriers by. */
  own: readonly OwnUnitView[];
}

export function CommandBar({
  state,
  hintHost,
}: {
  hintHost?: HTMLElement | null;
  state: FeedSource<ArmyCommandState>;
}) {
  const { units, weapons } = useSessionCatalog();
  const { control, own } = useFeedValue(state);
  if (!control) return null;
  const selected = control.selectedUnits;
  const n = selected.length;
  const command = (c: CommandProps) => <CommandButton {...c} hintHost={hintHost} />;
  const key = (command: keyof typeof CommandBindings) => `(${CommandBindings[command].label})`;
  // Capability actions apply to eligible units; movement applies to everyone.
  const [armed, deployers, inside, unloaders] = (
    ["attack", "deploy", "exit_building", "unload"] as ReachCommand[]
  ).map((c) => reach(c, selected, units, own));
  const hold = n > 0 && selected.every((u) => u.engagement === "return_fire_only");
  const packing = deployers.every((u) => u.deployment?.target === "deployed");
  const aircraft = selected.filter((u) => u.fuelS != null);
  const artillery = armed.some((u) =>
    units
      .type(u.kind)
      .mounts.some((mount) =>
        mount.weapons.some((weapon) => weapons[weapon]?.trajectory === "indirect"),
      ),
  );
  /** Movement stays available; an unsupported targeted attack is disabled. */
  const mode = (m: CommandMode, icon: string, label: string, available = n > 0, emphasis = false) =>
    command({
      icon,
      label,
      pressed: control.mode === m,
      disabled: !available,
      emphasis,
      onClick: () => control.setMode(m),
    });
  return (
    <div className="ro-commands" role="toolbar" aria-label="Commands">
      {mode("attack_move", hudIcon("attack_move"), `Attack-move ${key("attack_move")}`)}
      {mode("reverse_move", hudIcon("reverse"), `Reverse ${key("reverse_move")}`)}
      {mode(
        "attack_ground",
        hudIcon("attack_ground"),
        `Attack ground ${key("attack_ground")}`,
        armed.length > 0,
        artillery,
      )}
      {mode("fast_move", hudIcon("fast_move"), "Fast move (double right-click)")}
      {command({
        icon: hudIcon("stop"),
        label: `Stop ${key("stop")}`,
        disabled: n === 0,
        onClick: control.stop,
      })}
      {command({
        icon: hudIcon(hold ? "hold_fire" : "fire_at_will"),
        label: `${hold ? "Return fire only" : "Fire at will"} ${key("toggle_fire_policy")}`,
        pressed: hold,
        disabled: n === 0,
        onClick: control.togglePolicy,
      })}
      {command({
        icon: stateIcon(deployers.length > 0 && packing ? "pack" : "deploy"),
        label: `${deployers.length > 0 && packing ? "Pack" : "Deploy"} ${key("toggle_deployment")}`,
        disabled: deployers.length === 0,
        onClick: control.toggleDeployment,
      })}
      {command({
        icon: stateIcon(
          aircraft.length > 0 && aircraft.every((u) => u.afterburner) ? "afterburner" : "fuel",
        ),
        label: `${aircraft.length > 0 && aircraft.every((u) => u.afterburner) ? "Disable" : "Enable"} afterburners ${key("toggle_afterburner")}`,
        pressed: aircraft.length > 0 && aircraft.every((u) => u.afterburner),
        disabled: aircraft.length === 0,
        onClick: control.toggleAfterburner ?? (() => {}),
      })}
      {control.refund &&
        command({
          icon: stateIcon("withdrawing"),
          label: "Refund · Return to base",
          disabled: n === 0,
          onClick: control.refund,
        })}
      {command({
        icon: hudIcon("unload"),
        label: `Unload ${key("unload")}`,
        pressed: control.mode === "unload",
        disabled: unloaders.length === 0,
        onClick: control.pressUnload,
      })}
      {command({
        icon: hudIcon("leave_building"),
        label: "Leave building",
        disabled: inside.length === 0,
        onClick: control.exitBuilding,
      })}
    </div>
  );
}
