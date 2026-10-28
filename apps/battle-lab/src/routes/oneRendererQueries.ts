/** Real GPU snapshots while the live scene keeps rendering. */
import type { Renderer } from "@wasm/renderer_browser";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import { comparisonControls } from "./oneRendererControls";

interface PixelSnapshot {
  width: number;
  height: number;
  rgba: Uint8Array;
}

export function fogQueryPoints(presentation: ScenePresentation) {
  const eye = presentation.fogEyes()[0];
  if (!eye) throw new Error("fog recovery query needs a canonical eye");
  const [x, y, z] = eye.position;
  return [
    { position: [x, y, z] },
    ...[0.1, 0.5, 1.2].flatMap((share) =>
      [0, Math.PI / 2, Math.PI, (3 * Math.PI) / 2].map((angle) => ({
        position: [
          x + Math.cos(angle) * eye.range * share,
          y + Math.sin(angle) * eye.range * share,
          z,
        ],
      })),
    ),
  ];
}

export async function captureSharedQueries(
  renderer: Renderer,
  presentation: ScenePresentation,
  canvas: HTMLCanvasElement,
) {
  if (new URLSearchParams(window.location.search).get("readbacks") !== "1") return;
  canvas.dataset.queryBefore = canvas.toDataURL();
  const pending = renderer.read_rgba(false);
  const controls = comparisonControls();
  presentation.setRendererControls(renderer, { ...controls, view: "overlays-on-white" });
  presentation.render(renderer);
  const snapshot: PixelSnapshot = await pending;
  const image = document.createElement("canvas");
  image.width = snapshot.width;
  image.height = snapshot.height;
  image
    .getContext("2d")!
    .putImageData(
      new ImageData(new Uint8ClampedArray(snapshot.rgba), snapshot.width, snapshot.height),
      0,
      0,
    );
  canvas.dataset.queryReadback = image.toDataURL();
  presentation.setRendererControls(renderer, controls);
  presentation.render(renderer);
  await renderer.drain();
  canvas.dataset.queryRestored = String(canvas.dataset.queryBefore === canvas.toDataURL());
  const shape = renderer.fog_shape(
    Float32Array.from([
      3,
      2,
      1,
      0,
      3,
      2,
      1,
      Math.PI / 2,
      3,
      2,
      1,
      Math.PI,
      3,
      2,
      1,
      Math.PI / 4,
      3,
      2,
      1,
      (3 * Math.PI) / 4,
    ]),
  );
  // Render before awaiting: mapping must retain no wasm-bindgen scene borrow.
  presentation.render(renderer);
  canvas.dataset.queryShape = JSON.stringify(Array.from((await shape) as Float32Array));
}
