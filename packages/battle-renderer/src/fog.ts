/** Side-visible fog inputs and diagnostic readbacks; Rust owns GPU preparation. */
import type { Presentation } from "@wasm/renderer_browser";
/** One unseen look, as the fixture writes it. */
export interface FogStyle {
  /** Brightness kept, 0–1: the look's last multiplier. */
  dim: number;
  /** 0–1: how far the colour moves to the night tint at its own luminance. */
  cool: number;
  /** The night tint's hue (rgb); rescaled to unit luminance, so it tints
   *  without brightening. */
  tint: [number, number, number];
  /** Saturation kept before cooling: 0 grey, 1 the lit colour's own. */
  saturation: number;
  /** A glow of the night tint added over all unseen, at this HDR luminance:
   *  fog reads as a luminous veil, not a dark mass, and the veil lifts a sun
   *  shadow inside fog toward the fog around it. 0 off. */
  veil: number;
  /** Screen-space lines ruled across unseen pixels. */
  lines: {
    /** Brightness change on a line: 0.3 is 30% brighter, −0.3 darker; 0 off. */
    strength: number;
    /** A line's least brightening, in the night tint at this HDR luminance:
     *  keeps the lines as plain in a shadow as on lit ground, so a shadow
     *  inside fog reads as the same fog, darker, not as a second layer. */
    floor: number;
    /** Distance between lines, device pixels. */
    spacing_px: number;
    /** Line width, device pixels (anti-aliased). */
    width_px: number;
    /** The lines' angle, degrees counter-clockwise from screen horizontal. */
    angle_deg: number;
  };
  /** Device pixels over which the unseen side fades in from the boundary;
   *  0 is a hard edge. Seen pixels never change. */
  edge_softness: number;
  /** A line along the boundary, on the seen side: a sun shadow has none. */
  rim: {
    /** Device pixels; 0 draws none. */
    width_px: number;
    /** Display sRGB, 0–1: drawn after post, so exactly this colour. */
    color: [number, number, number];
    /** Opacity over the lit world, 0–1. */
    alpha: number;
  };
}

/** `presentation.fog`: named unseen looks and the one drawn. */
export interface FogPresentation {
  style: string;
  styles: Record<string, FogStyle>;
}
/** A point to probe: ground (at the target height above it) or a face. */
export interface FogProbeInput {
  position: readonly [number, number, number];
  /** Zero for ground. */
  normal?: readonly [number, number, number];
}

/** Presentation decides what fog is drawn from. */
export type FogSource = Pick<
  Presentation,
  "prepare_fog" | "resend_fog" | "rebuild_fog" | "fog_stats" | "fog_occluders" | "pages"
>;

/** A structure that takes fog whole, as the lab reads it back. */
export interface FogOccluder {
  x: number;
  y: number;
  yaw: number;
  hx: number;
  hy: number;
  base: number;
  top: number;
}

/** `Presentation.fog_occluders`' seven numbers a box, as boxes. */
export function occluderBoxes(n: Float64Array): FogOccluder[] {
  const boxes: FogOccluder[] = [];
  for (let i = 0; i < n.length; i += 7) {
    const [x, y, yaw, hx, hy, base, top] = n.subarray(i, i + 7);
    boxes.push({ x, y, yaw, hx, hy, base, top });
  }
  return boxes;
}

export interface FogProbes {
  probe(points: readonly FogProbeInput[]): Promise<Uint8Array>;
  probeShape(rows: Float32Array): Promise<Float32Array>;
  tileCounts(): Promise<Uint32Array>;
  rebuildAll(): void;
  wholes(): Promise<{ boxes: readonly FogOccluder[]; seen: Uint8Array }>;
}
