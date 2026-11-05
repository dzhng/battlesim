import type { PresentationLayer } from "@web/battle/benchmark/visualPresentation";

/** What the reel's viewport is always handed: the battle, without the
 *  side's fog of war or its indicators. */
const REEL_LAYERS: readonly PresentationLayer[] = [
  "sky",
  "terrain",
  "water",
  "grass",
  "scenery",
  "props",
  "structures",
  "buildings",
  "models",
  "effects",
  "scars",
];

/** What the graphics test draws, from its URL, each independently: the
 *  side's fog of war only with `?fog=1`, and the battle view's indicators
 *  (contacts, orders, zone, border; `overlays`) unless `?indicators=0`; and
 *  the layers that names. */
export function reelDrawing(params: URLSearchParams) {
  const fog = params.get("fog") === "1";
  const indicators = params.get("indicators") !== "0";
  const layers: readonly PresentationLayer[] = [
    ...REEL_LAYERS,
    ...(fog ? (["fog"] as const) : []),
    ...(indicators ? (["overlays"] as const) : []),
  ];
  return { fog, indicators, layers };
}
