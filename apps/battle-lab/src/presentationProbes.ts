import type { Vec3 } from "math";
import type { ScenePresentation } from "@web/battle/sim/presentation";

/** Read-only diagnostics of what the presentation (`current`) drew last
 *  frame, used by browser scenes to inspect its models, effects and props. */
export function presentationProbes(current: () => ScenePresentation | null) {
  return {
    /** The static pieces the last drawn frame drew beside the posed units (a
     *  cook-off's moving wreck and its debris): each one's appearance, state
     *  and how far its motion lowers it, metres. */
    cookOffPieces: () => current()?.probe().cookOffPieces ?? [],
    /** The soldiers the last drawn frame lays as static corpses. */
    lying: () => current()?.probe().lying ?? [],
    /** Where the last drawn frame draws the muzzle of the shown side's
     *  vehicle `id`'s mount `mount`, where its flash goes; null when no rig
     *  draws it. */
    drawnMuzzle: (id: number, mount: number): Vec3 | null =>
      current()?.drawnMuzzle(id, mount) ?? null,
    /** Combat effects: running, drawn last frame, dropped, and the last tick noted. */
    effects: () => current()?.probe().effects ?? null,
    /** The instances the effect pass drew last frame: shape (`SHAPE`), the
     *  point it sits on (a streak's tail), its size, and a glow's rays. */
    effectInstances: () => current()?.probe().effectInstances ?? [],
    /** The lights the effects offered last frame: where, how far, their
     *  colour × intensity, and what cast each. */
    castLights: () => current()?.probe().castLights ?? [],
    /** What the props draw (`ScenePresentation.propStats`): the bodies drawn,
     *  those a building draws, those no model draws, the models placed and
     *  those drawing what the side knows. */
    structureBodies: () => current()?.propStats() ?? null,
    /** Every prop model the side draws: appearance, state and placement. */
    structures: () => current()?.propModels() ?? [],
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
