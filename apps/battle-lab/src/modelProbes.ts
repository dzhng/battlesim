import type { ScenePresentation } from "@web/battle/sim/presentation";

/** Read-only model diagnostics used by browser scenes to inspect model rows. */
export function modelProbes(current: () => ScenePresentation | null) {
  return {
    /** Every drawn model's muzzles in the world, where the presentation
     *  draws them (a soldier's crossfaded and kicked, a hull's articulated
     *  and rocked), with each model's origin to select the firing body before
     *  its attachment is measured. */
    muzzleSockets: () => {
      const probe = current()?.probe();
      if (!probe) return [];
      const soldiers = probe.soldiers.flatMap((m) =>
        m.muzzle
          ? [{ appearance: m.appearance, origin: m.position, name: "muzzle", at: m.muzzle }]
          : [],
      );
      const vehicles = probe.vehicles.flatMap((m) =>
        m.muzzles.map(({ name, at }) => ({
          appearance: m.appearance,
          origin: m.position,
          name,
          at,
        })),
      );
      return [...soldiers, ...vehicles];
    },
    /** The vehicles as last posed: appearance, placement and articulation. */
    vehicles: () => current()?.probe().vehicles ?? [],
    drawnModels: () => current()?.probe().drawnModels ?? [],
  };
}
