// What the scenes measure of a colour: how light it is, and its hue apart
// from that. A pixel is a displayed sRGB triple, 0 to 255.
import { pixel } from "./_png.mjs";

/** An sRGB channel (0 to 255) as linear light, 0 to 1. */
export const linear = (v) => {
  const c = v / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};

/** Rec. 709's weighted sum of a colour: a linear colour's relative
 *  luminance, a displayed one's luma. */
export const rec709 = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

/** A displayed pixel's relative luminance, 0 to 1. */
export const luminance = ([r, g, b]) =>
  0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);

/** CIELAB a* and b* of a linear colour: its hue and chroma, apart from how
 *  light it is. Two colours differ in hue by their distance in this plane. */
export function chroma([r, g, b]) {
  const f = (t) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116);
  const x = f((0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.9505),
    y = f(0.2126 * r + 0.7152 * g + 0.0722 * b),
    z = f((0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.089);
  return [500 * (x - y), 200 * (y - z)];
}

/** The mean displayed colour and luminance of `png` over `pixels`. */
export function meanColour(png, pixels) {
  const sum = [0, 0, 0];
  let light = 0,
    count = 0;
  for (const [x, y] of pixels) {
    const rgb = pixel(png, x, y);
    rgb.forEach((v, k) => (sum[k] += v));
    light += luminance(rgb);
    count++;
  }
  return { count, rgb: sum.map((v) => v / count), luminance: light / count };
}

/** How far a colour leans from blue toward red, as a share of its red. */
export const warmth = ([r, , b]) => (r - b) / r;

/** The middle of `values` (of an even count, the upper of the two). */
export const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
