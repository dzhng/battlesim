import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";

/** Army cards use the same role, silhouette and strength slots for live and incoming units. */
export function ArmyCard({
  kind,
  name,
  strength,
}: {
  kind: string;
  name: string;
  strength: number;
}) {
  const { role, silhouette } = unitIcons(UNITS.type(kind));
  return (
    <>
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
    </>
  );
}
