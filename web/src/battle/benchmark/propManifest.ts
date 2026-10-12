// The `props` section of the `native-visual-presentation` manifest: what a
// second client needs besides the map to stand a side's buildings as the
// browser does: which map props are each building's parts, and the prop
// types a gutted building's parts become. The props themselves, and what the
// side knows of them, are the presentation's (`crates/presentation`), which
// both clients run.
// Plain Node runs the writer, so this imports nothing the renderer owns.
import { hasKeys } from "./presentationCapture.ts";

/** The manifest's `props`. */
export interface PropsManifest {
  /** Per building of `buildings`, its parts' map prop ids; and the prop
   *  types a gutted building's parts become (`buildingRemains` `shells`). */
  buildings: { parts: number[][]; shells: string[] };
}

const text = (value: unknown) => typeof value === "string" && value.length > 0;
const whole = (value: unknown) => Number.isInteger(value) && (value as number) >= 0;

/** `value` as the manifest's `props`, or a refusal naming the part at fault. */
export function validateProps(value: unknown): PropsManifest {
  if (!hasKeys(value, ["buildings"]))
    throw new Error("visual presentation props need the buildings");
  const { buildings } = value;
  if (
    !hasKeys(buildings, ["parts", "shells"]) ||
    !Array.isArray(buildings.parts) ||
    !buildings.parts.every((p) => Array.isArray(p) && p.every(whole)) ||
    new Set(buildings.parts.flat()).size !== buildings.parts.flat().length ||
    !Array.isArray(buildings.shells) ||
    !buildings.shells.every(text)
  )
    throw new Error(
      "visual presentation props need each building's distinct part props and the gutted shells' prop types",
    );
  return value as unknown as PropsManifest;
}
