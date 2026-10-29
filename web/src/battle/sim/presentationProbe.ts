/** Diagnostic observation contract for labs inspecting canonical presentation. */
import type { Articulation } from "@packages/scene-assets/src/articulation";

type P3 = [number, number, number];

/** The last frame as the labs' scenes probe it. */
export interface PresentationProbe {
  effects: { live: number; instances: number; dropped: number; sources: number; tick: number };
  effectInstances: { shape: number; at: P3; to: P3; rgb: P3; size: number; rays: number }[];
  castLights: { at: P3; radius: number; rgb: P3; cause: string }[];
  lying: number[];
  cookOffPieces: { appearance: string; state: string; sunk: number }[];
  soldiers: {
    appearance: string;
    position: P3;
    yaw: number;
    clip: string;
    phase: number;
    /** His weapon's muzzle where his body draws it (crossfaded and kicked),
     *  or null with no weapon up. */
    muzzle: P3 | null;
  }[];
  vehicles: {
    appearance: string;
    position: P3;
    yaw: number;
    tint: P3 | null;
    articulation: Articulation;
    /** Each mount's muzzle where the hull draws it (articulated and rocked),
     *  by its muzzle node's name. */
    muzzles: { name: string; at: P3 }[];
  }[];
}
