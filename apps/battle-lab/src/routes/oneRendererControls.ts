/** Identical live diagnostic inputs for both production renderer controls. */
import { FRAME_VIEWS, type BattleFrame, type FrameView } from "@packages/battle-renderer/src/scene";

export function comparisonControls() {
  const query = new URLSearchParams(window.location.search);
  const view = query.get("frameView") ?? "final";
  if (!FRAME_VIEWS.includes(view as FrameView)) throw new Error(`unknown frame view ${view}`);
  const shown = (key: string) => query.get(key) !== "0";
  return {
    view: view as FrameView,
    paint: shown("paint"),
    roadWear: shown("roadWear"),
    fieldTexture: shown("fieldTexture"),
    castLights: shown("castLights"),
    grass: shown("grass"),
    trees: shown("trees"),
    buildings: shown("buildings"),
    xrayCoverage: shown("xrayCoverage"),
  };
}

export function applyTypeGpuControls(frame: BattleFrame) {
  const controls = comparisonControls();
  frame.setView(controls.view);
  frame.setPaintShown(controls.paint);
  frame.setRoadWearShown(controls.roadWear);
  frame.setFieldTextureShown(controls.fieldTexture);
  frame.setCastLightsShown(controls.castLights);
  frame.setTreesShown(controls.trees);
  frame.setBuildingsShown(controls.buildings);
  frame.setXrayCoverageEnabled(controls.xrayCoverage);
  frame.grassProbes.suppress(!controls.grass);
}

/** Source-pixel evidence is captured before either device can be destroyed. */
export async function captureInspectorViews(
  canvas: HTMLCanvasElement,
  draw: (view: FrameView) => void,
  drain: () => Promise<unknown>,
) {
  if (new URLSearchParams(window.location.search).get("inspectViews") !== "1") return;
  const original = comparisonControls().view;
  const before = canvas.toDataURL();
  const images: Partial<Record<FrameView, string>> = {};
  try {
    for (const view of FRAME_VIEWS) {
      draw(view);
      await drain();
      images[view] = canvas.toDataURL();
    }
  } finally {
    draw(original);
    await drain();
  }
  canvas.dataset.inspectorViews = JSON.stringify(images);
  canvas.dataset.inspectorRestored = String(before === canvas.toDataURL());
}
