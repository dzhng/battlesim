// Appearance-owned admission for prepared static cards. Other surface
// kinds retain meshes; the two runtime atlases cannot represent their materials.
import type { InstalledAppearance } from "./loader";

export function staticCardPose(a: InstalledAppearance) {
  if (
    !(a.farCardPx && Number.isFinite(a.farCardPx) && a.farCardPx > 0) ||
    a.bundle.kind !== "static" ||
    !a.footprint
  )
    return null;
  const state = a.bundle.states[0];
  if (
    state?.name !== "default" ||
    a.bundle.materials.some((m) => m.coverage.kind !== "opaque" || m.interior)
  )
    return null;
  return { pose: { kind: "static" as const, state: state.name }, bounds: state.bounds };
}
