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

/** How the reel shows the battle view's unit info cards: not at all, as a
 *  player sees them at rest (compact), or open, as holding Space opens them. */
export type ReelCards = "off" | "compact" | "open";

/** What the graphics test draws, from its URL, each independently: the
 *  side's fog of war only with `?fog=1`; the battle view's indicators
 *  (contacts, orders, zone, border; `overlays`) unless `?indicators=0`; its
 *  unit info cards (`cards`) unless `?cards=0`, open with `?cards=open`;
 *  and the layers that names. */
export function reelDrawing(params: URLSearchParams) {
  const fog = params.get("fog") === "1";
  const indicators = params.get("indicators") !== "0";
  const asked = params.get("cards");
  const cards: ReelCards = asked === "0" ? "off" : asked === "open" ? "open" : "compact";
  const layers: readonly PresentationLayer[] = [
    ...REEL_LAYERS,
    ...(fog ? (["fog"] as const) : []),
    ...(indicators ? (["overlays"] as const) : []),
    ...(cards !== "off" ? (["cards"] as const) : []),
  ];
  return { fog, indicators, cards, layers };
}
