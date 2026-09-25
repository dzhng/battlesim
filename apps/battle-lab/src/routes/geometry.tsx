import { useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  buildWorldMeshes,
  type WorldLayout,
  type WorldOverlay,
} from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import geometryMap from "@fixtures/geometry-lab.json";
import { LabViewport, type LabPick } from "../LabViewport";
import { useStaticWorld, type WorldView } from "../useStaticWorld";

interface Probe {
  point: [number, number, number];
  collider: string;
  surface: string;
  slope: number;
  forest: boolean;
  traversable: boolean;
}

export const GEOMETRY_CAMERA: Camera3DParams = {
  target: [170, 140, 0],
  distance: 420,
  pitch: 0.82,
  yaw: -1.25,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

/** Probe the authoritative surface under a camera ray. */
function probe(view: WorldView, layout: WorldLayout, ray: LabPick["ray"]): Probe | null {
  const hit = view.raycast(...ray.origin, ...ray.dir, 5000);
  if (hit.length === 0) return null;
  const [, x, y, z, , , , prop] = hit;
  const s = view.surface_at(x, y);
  const propKind =
    prop >= 0 ? layout.propKinds[view.props()[prop * layout.propStride + 1]] : "terrain";
  return {
    point: [x, y, z],
    collider: prop >= 0 ? `${propKind} #${prop}` : "terrain",
    surface: s.length ? layout.surfaceKinds[s[5]] : "out of bounds",
    slope: s[4] ?? 0,
    forest: s[6] === 1,
    traversable: s[7] === 1,
  };
}

export default function Geometry() {
  const world = useStaticWorld(geometryMap);
  const [overlay, setOverlay] = useState<WorldOverlay>("surface");
  const [showCanopy, setShowCanopy] = useState(true);
  const [probed, setProbed] = useState<Probe | null>(null);

  const meshes = useMemo(() => {
    if (!world) return null;
    const built = buildWorldMeshes(world.exports, world.layout, overlay);
    return showCanopy ? built : { ...built, translucent: new Float32Array(0) };
  }, [world, overlay, showCanopy]);

  const instances = useMemo<SceneInstance[]>(
    () =>
      probed
        ? [
            {
              kind: "marker",
              x: probed.point[0],
              y: probed.point[1],
              z: probed.point[2],
              yaw: 0,
              color: [1, 1, 1],
            },
          ]
        : [],
    [probed],
  );

  const diagnostics = useMemo(
    () =>
      world && {
        exports: world.exports,
        layout: world.layout,
        heightAt: (x: number, y: number) => world.view.height_at(x, y),
        probeRay: (ray: LabPick["ray"]) => probe(world.view, world.layout, ray),
        setOverlay,
        setShowCanopy,
      },
    [world],
  );

  if (!world || !meshes) return null;
  return (
    <>
      <LabViewport
        fixture="geometry"
        world={meshes}
        instances={instances}
        initialCamera={GEOMETRY_CAMERA}
        onPick={(pick) =>
          pick.button === "left" && setProbed(probe(world.view, world.layout, pick.ray))
        }
        diagnostics={diagnostics ?? undefined}
      />
      <aside className="lab-panel" data-testid="geometry-panel">
        <strong>Geometry</strong>
        <div className="lab-hint">
          Click probes the authoritative surface · middle‑drag orbit · arrows pan
        </div>
        <fieldset className="lab-field">
          <legend>Ground overlay</legend>
          {(["surface", "traversal"] as const).map((o) => (
            <label key={o}>
              <input
                type="radio"
                name="overlay"
                checked={overlay === o}
                onChange={() => setOverlay(o)}
              />
              {o === "surface"
                ? "Surface class"
                : `Traversable (slope < ${world.view.slope_cutoff_deg()}°, no water)`}
            </label>
          ))}
          <label>
            <input
              type="checkbox"
              checked={showCanopy}
              onChange={(e) => setShowCanopy(e.target.checked)}
            />
            Forest canopy volume
          </label>
        </fieldset>
        <div data-testid="probe">
          {probed ? (
            <>
              <div>
                Hit {probed.collider} at ({probed.point.map((v) => v.toFixed(2)).join(", ")})
              </div>
              <div>
                Surface: {probed.surface} · slope {probed.slope.toFixed(1)}° ·{" "}
                {probed.forest ? "forest · " : ""}
                {probed.traversable ? "traversable" : "blocked"}
              </div>
            </>
          ) : (
            "Click the world to probe."
          )}
        </div>
      </aside>
    </>
  );
}
