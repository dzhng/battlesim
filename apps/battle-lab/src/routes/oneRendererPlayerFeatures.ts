/** Matched player-mark inputs over the captured production street. */
import {
  MeshBuilder,
  groundAnnulus,
  groundStrip,
  type Mesh,
} from "@packages/battle-renderer/src/mesh";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { LabModel } from "@web/battle/sim/presentationModels";
import { modelAssetsOf } from "@web/battle/render/modelAssets";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { gameModelDetail } from "../gameModels";
import { REST_ARTICULATION } from "@packages/scene-assets/src/articulation";

export function playerFeatureInputs(own: readonly OwnUnitView[], eye: readonly number[]) {
  const shown = new URLSearchParams(window.location.search).get("playerFeatures") === "1";
  if (!shown) return null;
  const unit = [...own].sort(
    (a, b) =>
      Math.hypot(a.position[0] - eye[0], a.position[1] - eye[1]) -
      Math.hypot(b.position[0] - eye[0], b.position[1] - eye[1]),
  )[0];
  if (!unit) throw new Error("player feature capture requires an own unit");
  const [x, y, z] = unit.position;
  const painted = new MeshBuilder();
  groundAnnulus(painted, [x, y], 3.8, 4.1, {
    z: () => z,
    segments: 48,
    colorIn: [0.15, 0.75, 1, 0.9],
    colorOut: [0.15, 0.75, 1, 0.9],
  });
  const marching = new MeshBuilder();
  groundStrip(marching, [x - 6, y + 2], [x + 12, y + 2], 0.25, [0.2, 1, 0.45, 0.8], {
    z: () => z,
    dashed: true,
    step: 2,
  });
  const march = marching.build();
  for (let i = 0; i < march.length; i += 10) {
    march[i + 3] = (march[i] - x) * 0.1;
    march[i + 4] = 1;
    march[i + 5] = 0.8;
  }
  const pointer = new MeshBuilder();
  groundAnnulus(pointer, [x + 7, y - 2], 1.5, 1.7, {
    z: () => z,
    segments: 32,
    colorIn: [1, 0.65, 0.1, 0.9],
  });
  const opaque = new MeshBuilder();
  opaque.segment([x - 5, y - 3, z + 0.15], [x + 10, y - 3, z + 0.15], 0.08, [0.9, 0.75, 0.2, 1]);
  const translucent = new MeshBuilder();
  translucent.mark([x + 4, y, z + 1.5], 0.65, [1, 0.15, 0.2, 0.6]);
  const overlay: {
    opaque: Mesh;
    translucent: Mesh;
    painted: Mesh;
    paintedMarching: Mesh;
  } = {
    opaque: opaque.build(),
    translucent: translucent.build(),
    painted: painted.build(),
    paintedMarching: march,
  };
  return { selected: [unit.id], overlay, pointer: pointer.build() };
}

/** Exercise external poses through the same canonical frame in both controls. */
export function installLabFixture(
  presentation: ScenePresentation,
  installed: InstalledAppearances,
  at: readonly number[],
) {
  if (new URLSearchParams(window.location.search).get("labModels") !== "1") return;
  const vehicle = [...installed.appearances].find(
    ([, value]) => value.bundle.kind === "articulated",
  );
  if (!vehicle) throw new Error("lab fixture needs an installed articulated model");
  const [name] = vehicle;
  const [x, y, z] = at;
  const models: LabModel[] = [
    {
      appearance: name,
      x: x - 6,
      y,
      z,
      yaw: 0.7,
      tier: 0,
      tint: [0.5, 0.7, 1],
      pose: { kind: "rest" },
    },
    {
      appearance: name,
      x: x + 6,
      y,
      z,
      yaw: -0.4,
      tier: 1,
      tint: [1, 0.5, 0.4],
      pose: {
        kind: "articulated",
        articulation: { ...REST_ARTICULATION, turret_yaw: 0.8 },
      },
    },
    {
      appearance: name,
      x,
      y: y + 8,
      z,
      yaw: 0,
      tier: 0,
      pose: { kind: "rest" },
      ghost: [0.15, 0.8, 1, 0.45],
    },
  ];
  presentation.installLabAssets(modelAssetsOf(installed, [name]), JSON.parse(gameModelDetail));
  presentation.setLabModels(models);
}
