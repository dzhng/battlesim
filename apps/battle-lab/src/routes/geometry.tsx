import generated from "@fixtures/generated-battle.json";
import { listMaps } from "@web/maps/catalogue";
import { fixtureMap } from "../fixtures";
import { useMemo, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  buildWorldLayers,
  type WorldLayout,
  type WorldOverlay,
} from "@packages/battle-renderer/src/worldMesh";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import type { MapDefinition } from "@web/maps/resolve";
import { SavedMap } from "../savedMaps";
import { LabViewport, type LabPick } from "../LabViewport";
import { useStandingBuildings, useStaticWorld, type WorldView } from "../useStaticWorld";
import { gameBiome } from "../gameBiome";
import { useMapAppearances } from "../gameAppearances";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";

interface Probe {
  point: [number, number, number];
  collider: string;
  surface: string;
  slope: number;
  forest: boolean;
  traversable: boolean;
}

const GEOMETRY_CAMERA: Camera3DParams = {
  target: [170, 140, 0],
  distance: 420,
  pitch: 0.82,
  yaw: -1.25,
  ...gameCamera.lens,
};

/** Probe the authoritative surface under a camera ray. */
function probe(
  view: WorldView,
  layout: WorldLayout,
  ray: LabPick["ray"],
  reach: number,
): Probe | null {
  const hit = view.raycast(...ray.origin, ...ray.dir, reach);
  if (hit.length === 0) return null;
  const [, x, y, z, , , , prop] = hit;
  const s = view.surface_at(x, y);
  const propKind =
    prop >= 0
      ? layout.propKinds[view.props()[prop * layout.propStride + layout.propFields.indexOf("kind")]]
      : "terrain";
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
  const id = new URLSearchParams(window.location.search).get("map") ?? fixtureMap("geometry");
  return <SavedMap id={id}>{(map) => <GeometryLab id={id} readMap={() => map} />}</SavedMap>;
}

// The reader keeps the map's large arrays out of React's changed-prop details.
function GeometryLab({ id, readMap }: { id: string; readMap: () => MapDefinition }) {
  const map = readMap();
  const generic = id !== fixtureMap("geometry");
  const label = generic ? (listMaps().find((entry) => entry.id === id)?.label ?? id) : "Geometry";
  const camera: Camera3DParams = generic
    ? {
        ...GEOMETRY_CAMERA,
        target: [map.size[0] / 2, map.size[1] / 2, 0],
        distance: Math.max(...map.size) * generated.camera.overview_span,
        pitch: generated.camera.overview_pitch,
      }
    : GEOMETRY_CAMERA;
  const cameraConfig = gameCamera.forMap(map.size);
  const probeReach = Math.max(5000, cameraConfig.zoom_max + Math.hypot(...map.size));
  const world = useStaticWorld(map);
  const [overlay, setOverlay] = useState<WorldOverlay>("surface");
  const [showTrees, setShowTrees] = useState(true);
  const buildings = useStandingBuildings(world);
  const appearances = useMapAppearances(buildings?.placed ?? null);
  const [probed, setProbed] = useState<Probe | null>(null);

  const meshes = useMemo(() => {
    if (!world || !appearances) return null;
    const built = buildWorldLayers(
      world.exports,
      world.layout,
      gameBiome,
      overlay,
      [],
      appearances,
    );
    return showTrees ? built : { ...built, scenery: null };
  }, [world, overlay, showTrees, appearances]);
  const worldFeed = useFeed(meshes);
  // The traversal view shows what blocks as boxes, the buildings' parts among them.
  const buildingsFeed = useFeed(overlay === "surface" ? buildings : null);

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
        mapId: id,
        mapSize: map.size,
        exports: world.exports,
        layout: world.layout,
        heightAt: (x: number, y: number) => world.view.height_at(x, y),
        probeRay: (ray: LabPick["ray"]) => probe(world.view, world.layout, ray, probeReach),
        setOverlay,
        setShowTrees,
      },
    [world, id, map.size, probeReach],
  );

  if (!world || !meshes) return null;
  return (
    <>
      <LabViewport
        fixture="geometry"
        world={worldFeed}
        buildings={buildingsFeed}
        appearances={appearances}
        instances={instances}
        initialCamera={camera}
        cameraConfig={cameraConfig}
        onPick={(pick) =>
          pick.button === "left" && setProbed(probe(world.view, world.layout, pick.ray, probeReach))
        }
        diagnostics={diagnostics ?? undefined}
      />
      <aside className="hud-panel lab-panel" data-testid="geometry-panel">
        <strong>{label}</strong>
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
              checked={showTrees}
              onChange={(e) => setShowTrees(e.target.checked)}
            />
            Trees (the forests' drawn canopy)
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
