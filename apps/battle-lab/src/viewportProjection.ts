import { vec3 } from "math";
import {
  createGpuMat4,
  createProjectedPoint,
  projectPoint,
  viewProjMatrix,
} from "@packages/renderer-core/src/camera3d";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import type { Project } from "@web/battle/present/readouts";

const _projector_world = vec3.create();
const _projector_point = createProjectedPoint();

/** Project anchors through the submitted view with one layout read per frame. */
export function viewportProjector(view: ViewportCamera, rect: DOMRect): Project {
  const viewProj = viewProjMatrix(createGpuMat4(), liveCamera(view));
  return (x, y, z) => {
    vec3.set(_projector_world, x, y, z);
    const { ndc, clipW } = projectPoint(_projector_point, viewProj, _projector_world);
    if (clipW <= 0) return null;
    return [
      rect.left + (ndc[0] * 0.5 + 0.5) * rect.width,
      rect.top + (0.5 - ndc[1] * 0.5) * rect.height,
    ];
  };
}
