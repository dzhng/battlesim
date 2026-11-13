// What the rules derive from a weapon row (`derived` on the catalog's
// rows, `contract::weapons::Derived`), shown read-only. It is no gameplay
// field: the allowlist never names it, so it is never drafted, and the
// resolver refuses it as an authored key.
import type { AltitudeLayer, WeaponDerived } from "../../../packages/scene-assets/src/units";
import { isObject } from "./fields";
import type { JsonObject } from "./protocol";

const LAYERS: Record<AltitudeLayer, string> = {
  ground: "Ground range",
  low_air: "Low-air range",
  high_air: "High-air range",
};

export interface DerivedFact {
  label: string;
  value: string;
  /** A layer it leaves alone. */
  none?: boolean;
}

/** The facts a resolved weapon row publishes under `derived`, as the editor
 *  shows them, with every layer named, engaged or not; none for a row that
 *  publishes none. */
export function derivedFacts(row: JsonObject): DerivedFact[] {
  if (!isObject(row.derived)) return [];
  const derived = row.derived as unknown as WeaponDerived;
  return [
    {
      label: "Fires at what it cannot hurt",
      value: derived.fires_regardless ? "Yes, at any identified enemy" : "No",
    },
    ...(Object.keys(LAYERS) as AltitudeLayer[]).map((layer) => {
      const reach = derived.reach.find((r) => r.layer === layer);
      return {
        label: LAYERS[layer],
        value: reach
          ? [
              `${reach.range_m} m`,
              reach.only && `${reach.only.join(", ")} only`,
              reach.intercepts && "also shoots down incoming rounds",
            ]
              .filter(Boolean)
              .join(" · ")
          : "Does not engage",
        none: !reach,
      };
    }),
  ];
}

/** The saved row's derived facts; a draft changes them only once saved. */
export function DerivedFacts({ row }: { row: JsonObject }) {
  const facts = derivedFacts(row);
  if (!facts.length) return null;
  return (
    <section className="me-derived">
      <h5>
        Derived by the rules · read-only <span>updates on save</span>
      </h5>
      <dl>
        {facts.map((fact) => (
          <div key={fact.label}>
            <dt>{fact.label}</dt>
            <dd className={fact.none ? "me-derived-none" : undefined}>{fact.value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
