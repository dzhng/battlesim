import { useMemo, useState } from "react";
import { CameraController } from "@packages/renderer-core/src/cameraController";
import type { CameraPresentation } from "@packages/renderer-core/src/cameraController";
import type {
  WorldExports,
  WorldLayout,
  WorldOverlay,
} from "@packages/battle-renderer/src/worldMesh";
import { buildTerrainSurface } from "@packages/battle-renderer/src/terrain/terrainSurface";
import { appendRemoteTerrain } from "./matchedWorld";
import { createViewMeasurement } from "./viewMeasurement";
import { gameBiome } from "../gameBiome";
import { APP_COMMIT } from "../buildIdentity";
import { gameLightFor } from "../gameLight";

/** Opt-in geometry control: one admitted local export, only remote terrain changes. */
export function useGeometryScaling(
  local: WorldExports | undefined,
  layout: WorldLayout | undefined,
  cameraConfig: CameraPresentation,
  overlay: WorldOverlay,
) {
  const enabled = window.location.pathname === "/labs/geometry-scaling";
  const [measurement] = useState(createViewMeasurement);
  const [arm, setArm] = useState({ copies: 0, elevation: 0 });
  const built = useMemo(() => {
    if (!enabled || !local || !layout) return null;
    const started = performance.now();
    const exports = appendRemoteTerrain(local, arm.copies, arm.elevation);
    const terrain = buildTerrainSurface(exports, layout, gameBiome, overlay);
    // Preserve the admitted material plots/height grid. Expanding that domain
    // re-seeds local fields. Receiver bounds still come from the COMPLETE mesh
    // in ScenePresentation.setShadow, never from this material input's box.
    terrain.input = { ...terrain.input, exports: local };
    return { terrain, constructionMs: performance.now() - started, exports };
  }, [enabled, local, layout, arm, overlay]);
  return {
    terrain: built?.terrain,
    pilot: enabled ? measurement.pilot : undefined,
    diagnostics:
      enabled && built
        ? {
            setScalingArm(copies: number, elevation: number) {
              // Validate before scheduling React, so a bad command rejects at its caller.
              if (![0, 1, 4, 16].includes(copies) || ![0, 100].includes(elevation))
                throw new Error("unsupported matched-world arm");
              setArm({ copies, elevation });
            },
            scaling: {
              ...arm,
              build: APP_COMMIT,
              constructionMs: built.constructionMs,
              vertices: built.exports.positions.length / 3,
              triangles: built.exports.indices.length / 3,
              exportBytes:
                built.exports.positions.byteLength +
                built.exports.indices.byteLength +
                built.exports.triangleSurfaces.byteLength,
              meshExtents: built.exports.extents,
              materialExtents: local!.extents,
              light: gameLightFor(window.location.search),
              cameraConfig,
              clock: 0,
              ticks: null,
              worker: "unavailable: static geometry fixture",
              populations: "unchanged local props, buildings and scenery; remote terrain only",
            },
            placeView(target: readonly [number, number], distance: number) {
              measurement.place({
                target,
                distance,
                yaw: -1.57,
                pitch: new CameraController(cameraConfig).pitchAt(distance),
              });
            },
            beginViewMeasurement: measurement.begin,
            viewMeasurement: measurement.read,
            viewFrame: measurement.current,
          }
        : {},
  };
}
