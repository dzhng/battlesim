import type { ReactNode } from "react";
import { useSessionCatalog } from "../catalog/context";
import { cardIcon, unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";

/** A unit's vertical card face, the same in the army deck and the
 *  reinforcements picker: its role over its silhouette, then what this card
 *  says of it (an owned unit's strength, a purchase's name and price). `kind`
 *  is a unit type, or a purchase card that is not one yet (a planned unit:
 *  its silhouette, no role). */
export function UnitFace({ kind, children }: { kind: string; children: ReactNode }) {
  const { units } = useSessionCatalog();
  const role = units.has(kind) ? unitIcons(units.type(kind)).role : null;
  return (
    <>
      {role && <Icon path={role} className="hud-unit-role" />}
      <Icon path={cardIcon(kind)} className="hud-unit-silhouette" />
      {children}
    </>
  );
}

/** An owned unit's strength, the army card's footer. */
export function StrengthMeter({ name, strength }: { name: string; strength: number }) {
  return (
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
  );
}
