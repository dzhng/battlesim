/** The unit info cards as the live `ReadoutLayer` laid them out this frame,
 *  read back from its page as primitives a second client can draw: each
 *  shown card's backing, fills, words and icons relative to the card, where
 *  the card stands, its opacity (the contact fade, the close-camera fade
 *  `cardZoomOpacity`, a far view's softening) and its leader line. Nothing
 *  is decided here: what has a card, what it says, where it goes and how it
 *  fades are the readout layer's, from the side's own observation; this
 *  only reads the page that drew them. Words and icons are named by the
 *  font or the markup they were drawn with (`SpriteRequests`), so their
 *  pixels can come from the browser that laid them out. */

export type Rgba = [number, number, number, number];
export type Box = [number, number, number, number];

/** A text shadow, or a drop shadow's: its colour and CSS blur length. */
export interface ShadowSpec {
  color: Rgba;
  blur: number;
}

/** The face a run of words was drawn in: everything its glyphs' pixels
 *  depend on. */
export interface FontSpec {
  style: string;
  weight: string;
  /** CSS pixels. */
  size: number;
  family: string;
  /** Extra advance after each glyph, CSS pixels. */
  letterSpacing: number;
  color: Rgba;
  /** Painted under the glyphs, the first over the rest (`text-shadow`). */
  shadows: ShadowSpec[];
}

/** An icon as drawn: its markup with its computed paint inlined, its box,
 *  and the drop shadows its card gives it (CSS `filter`, inner first). */
export interface SvgSpec {
  markup: string;
  width: number;
  height: number;
  filter: string;
}

/** One primitive of a card, in CSS pixels from its top-left. */
export type Primitive =
  | { rect: { box: Box; fill: Rgba; border: Rgba; borderPx: number; blurPx: number } }
  | { text: { text: string; font: number; at: [number, number]; opacity: number } }
  | { sprite: { sprite: number; at: [number, number]; opacity: number } };

/** How a card's leader is stroked. */
export interface LeaderStyle {
  width: number;
  ink: Rgba;
  opacity: number;
  /** On and off lengths; empty for a whole line. */
  dash: number[];
  /** Drop shadows under it, the first over the second. */
  shadows: ShadowSpec[];
}

/** One shown card this frame. */
export interface CardPlacement {
  /** Whose card: the owner and the unit, enemy or contact id it hangs off
   *  (`own-12`, `enemy-21`, `contact-2 retiring`), as the page named it. */
  card: string;
  /** Its contents, and a key that changes whenever they could. */
  part: Primitive[];
  signature: string;
  /** Its top-left in CSS pixels. */
  at: [number, number];
  opacity: number;
  leader: {
    ring: [number, number, number];
    path: [number, number][];
    style: LeaderStyle;
  } | null;
}

/** Where words and icons get their pixels: an index for each face (noting
 *  the characters drawn in it) and each icon. */
export interface SpriteRequests {
  font(spec: FontSpec, text: string): number;
  svg(spec: SvgSpec): number;
}

const NONE: Rgba = [0, 0, 0, 0];

/** A computed colour (`rgb(…)`, `rgba(…)`, `transparent`) as straight sRGB. */
export function cssColor(value: string): Rgba {
  const inner = /rgba?\(([^)]*)\)/.exec(value)?.[1];
  if (!inner) return NONE;
  const [r, g, b, a = 1] = inner
    .split(/[\s,/]+/)
    .filter(Boolean)
    .map((part, i) => (i === 3 && part.endsWith("%") ? parseFloat(part) / 100 : parseFloat(part)));
  return [r / 255, g / 255, b / 255, a];
}

/** Splits a computed list at its top-level commas. */
function topLevel(list: string, separator: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < list.length; i++) {
    if (list[i] === "(") depth++;
    else if (list[i] === ")") depth--;
    else if (list[i] === separator && depth === 0) {
      out.push(list.slice(start, i));
      start = i + 1;
    }
  }
  out.push(list.slice(start));
  return out.map((s) => s.trim()).filter(Boolean);
}

/** One computed shadow (`rgba(…) 0px 0px 2px`): its colour and blur. */
function shadowOf(text: string): ShadowSpec {
  const color = /rgba?\([^)]*\)/.exec(text)?.[0] ?? "rgb(0, 0, 0)";
  const lengths = text.replace(color, "").trim().split(/\s+/).filter(Boolean).map(parseFloat);
  return { color: cssColor(color), blur: lengths[2] ?? 0 };
}

/** A computed `text-shadow` list. */
export function textShadows(value: string): ShadowSpec[] {
  return value === "none" ? [] : topLevel(value, ",").map(shadowOf);
}

/** The `drop-shadow(…)` functions of a computed `filter`, in order. */
export function dropShadows(value: string): string[] {
  if (value === "none") return [];
  return topLevel(value, " ").filter((f) => f.startsWith("drop-shadow("));
}

const dropShadowSpec = (fn: string): ShadowSpec => shadowOf(fn.slice("drop-shadow(".length, -1));

/** A leader's path data as `ReadoutLayer` writes it: a ring of radius r
 *  at the anchor (two arcs from its left), then the line to the card's
 *  near corner and along its edge. Anything else is refused: a new shape
 *  must be read here as well. */
export function leaderGeometry(d: string): {
  ring: [number, number, number];
  path: [number, number][];
} | null {
  if (!d.trim()) return null;
  if (d.replace(/[^A-Za-z]/g, "") !== "MaaMLL")
    throw new Error(`a card leader's path is not a ring and a line: ${d}`);
  const n = (d.match(/-?\d*\.?\d+(?:e[-+]?\d+)?/gi) ?? []).map(Number);
  if (n.length !== 22) throw new Error(`a card leader's path has ${n.length} numbers: ${d}`);
  const r = n[2];
  return {
    ring: [n[0] + r, n[1], r],
    path: [
      [n[16], n[17]],
      [n[18], n[19]],
      [n[20], n[21]],
    ],
  };
}

const fontAscents = new Map<string, number>();
let measure: CanvasRenderingContext2D | null = null;

/** A face's ascent above its baseline, in CSS pixels, as layout sees it. */
function ascent(font: string): number {
  let a = fontAscents.get(font);
  if (a === undefined) {
    measure ??= document.createElement("canvas").getContext("2d")!;
    measure.font = font;
    a = measure.measureText("H").fontBoundingBoxAscent;
    fontAscents.set(font, a);
  }
  return a;
}

const PAINT = [
  "fill",
  "stroke",
  "stroke-width",
  "stroke-dasharray",
  "stroke-dashoffset",
  "stroke-linecap",
  "stroke-linejoin",
  "fill-opacity",
  "stroke-opacity",
  "opacity",
  "color",
] as const;

/** `svg`'s markup with each element's computed paint inlined, sized to its
 *  box: drawn alone it looks as it does on the card. */
function inlinedSvg(svg: SVGSVGElement, width: number, height: number): string {
  const copy = svg.cloneNode(true) as SVGSVGElement;
  const source = [svg, ...svg.querySelectorAll("*")];
  const target = [copy, ...copy.querySelectorAll("*")];
  source.forEach((el, i) => {
    const style = getComputedStyle(el);
    const out = target[i];
    out.removeAttribute("class");
    out.removeAttribute("style");
    for (const property of PAINT) {
      // The card applies the icon's own opacity; its parts keep theirs.
      if (property === "opacity" && i === 0) continue;
      out.setAttribute(property, style.getPropertyValue(property));
    }
  });
  copy.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  copy.setAttribute("width", String(width));
  copy.setAttribute("height", String(height));
  return new XMLSerializer().serializeToString(copy);
}

const opacityOf = (style: CSSStyleDeclaration) => parseFloat(style.opacity || "1");

const fade = (c: Rgba, k: number): Rgba => [c[0], c[1], c[2], c[3] * k];
const px = (v: string) => parseFloat(v) || 0;

/** Where a card's primitives are measured from: its top-left. */
interface Origin {
  x: number;
  y: number;
}

/** `el`'s backing (`::before`), which paints under its card's contents. */
function backingOf(el: HTMLElement, k: number, o: Origin): Primitive | null {
  const before = getComputedStyle(el, "::before");
  const backing = cssColor(before.backgroundColor);
  if (before.content === "none" || before.content === "normal" || backing[3] <= 0) return null;
  const r = el.getBoundingClientRect();
  const blur = /blur\(([-\d.]+)px\)/.exec(before.filter)?.[1];
  return {
    rect: {
      box: [
        r.left - o.x + px(before.left),
        r.top - o.y + px(before.top),
        r.right - o.x - px(before.right),
        r.bottom - o.y - px(before.bottom),
      ],
      fill: fade(backing, k * opacityOf(before)),
      border: NONE,
      borderPx: 0,
      blurPx: blur ? parseFloat(blur) : 0,
    },
  };
}

/** `el`'s own fill and border (a pip, a progress bar), if it has either. */
function boxOf(
  el: HTMLElement,
  style: CSSStyleDeclaration,
  k: number,
  o: Origin,
): Primitive | null {
  const fill = cssColor(style.backgroundColor);
  const borderPx = style.borderTopStyle === "none" ? 0 : px(style.borderTopWidth);
  const border = cssColor(style.borderTopColor);
  if (fill[3] <= 0 && !(borderPx > 0 && border[3] > 0)) return null;
  const r = el.getBoundingClientRect();
  if (r.width <= 0 || r.height <= 0) return null;
  return {
    rect: {
      box: [r.left - o.x, r.top - o.y, r.right - o.x, r.bottom - o.y],
      fill: fade(fill, k),
      border: borderPx > 0 ? fade(border, k) : NONE,
      borderPx,
      blurPx: 0,
    },
  };
}

/** A run of words as its parent `style` draws it, on its baseline. */
function wordsOf(
  child: Text,
  style: CSSStyleDeclaration,
  k: number,
  o: Origin,
  sprites: SpriteRequests,
): Primitive | null {
  const words = child.data.replace(/\s+/g, " ").trim();
  if (!words) return null;
  const range = document.createRange();
  range.selectNodeContents(child);
  const r = range.getBoundingClientRect();
  if (r.width <= 0) return null;
  const size = parseFloat(style.fontSize);
  const spec: FontSpec = {
    style: style.fontStyle,
    weight: style.fontWeight,
    size,
    family: style.fontFamily,
    letterSpacing: style.letterSpacing === "normal" ? 0 : px(style.letterSpacing),
    color: cssColor(style.color),
    shadows: textShadows(style.textShadow),
  };
  const text = style.textTransform === "uppercase" ? words.toUpperCase() : words;
  const face = `${spec.style} ${spec.weight} ${size}px ${spec.family}`;
  return {
    text: {
      text,
      font: sprites.font(spec, text),
      at: [r.left - o.x, r.top - o.y + ascent(face)],
      opacity: k,
    },
  };
}

/** An icon (or a timer's ring) with the drop shadows its card gives it,
 *  its own first, up to the card `node`. */
function iconOf(
  svg: SVGSVGElement,
  node: HTMLElement,
  k: number,
  o: Origin,
  sprites: SpriteRequests,
): Primitive | null {
  const r = svg.getBoundingClientRect();
  const style = getComputedStyle(svg);
  if (r.width <= 0 || r.height <= 0 || style.display === "none") return null;
  const filters: string[] = [];
  for (let at: Element | null = svg; at && at !== node.parentElement; at = at.parentElement)
    filters.push(...dropShadows(getComputedStyle(at).filter));
  return {
    sprite: {
      sprite: sprites.svg({
        markup: inlinedSvg(svg, r.width, r.height),
        width: r.width,
        height: r.height,
        filter: filters.join(" "),
      }),
      at: [r.left - o.x, r.top - o.y],
      opacity: k * opacityOf(style),
    },
  };
}

/** One card's contents, from its panel element `node`: backings first,
 *  then fills, words and icons in document order. */
function cardPart(node: HTMLElement, sprites: SpriteRequests): Primitive[] {
  const box = node.getBoundingClientRect();
  const o: Origin = { x: box.left, y: box.top };
  const scrims: Primitive[] = [];
  const ink: Primitive[] = [];
  const keep = (list: Primitive[], p: Primitive | null) => {
    if (p) list.push(p);
  };
  const visit = (el: HTMLElement, opacity: number) => {
    const style = getComputedStyle(el);
    if (style.display === "none" || style.visibility === "hidden") return;
    const k = opacity * opacityOf(style);
    if (k <= 0) return;
    keep(scrims, backingOf(el, k, o));
    keep(ink, boxOf(el, style, k, o));
    for (const child of el.childNodes) {
      if (child instanceof Text) keep(ink, wordsOf(child, style, k, o, sprites));
      else if (child instanceof SVGSVGElement) keep(ink, iconOf(child, node, k, o, sprites));
      else if (child instanceof HTMLElement) visit(child, k);
    }
  };
  visit(node, 1);
  return [...scrims, ...ink];
}

/** The leader `path` of a card, as stroked. */
function leaderOf(path: SVGPathElement | null): CardPlacement["leader"] {
  const geometry = path && leaderGeometry(path.getAttribute("d") ?? "");
  if (!path || !geometry) return null;
  const style = getComputedStyle(path);
  const dash =
    style.strokeDasharray === "none" ? [] : style.strokeDasharray.split(/[\s,]+/).map(parseFloat);
  return {
    ...geometry,
    style: {
      width: parseFloat(style.strokeWidth),
      ink: cssColor(style.stroke),
      opacity: opacityOf(style),
      dash: dash.length >= 2 ? dash.slice(0, 2) : [],
      shadows: dropShadows(style.filter).map(dropShadowSpec),
    },
  };
}

/** A card's contents last read, while its markup stays the same. */
export type PartCache = WeakMap<HTMLElement, { signature: string; part: Primitive[] }>;

/** Every card the readout layer `layer` shows this frame, back to front:
 *  the ground and low-air plane, then the high-air plane, each in its
 *  paint order. A hidden card, or one faded to nothing, has none. */
export function readoutSnapshot(
  layer: HTMLElement,
  sprites: SpriteRequests,
  cache: PartCache,
): CardPlacement[] {
  const out: CardPlacement[] = [];
  for (const plane of layer.querySelectorAll<HTMLElement>(":scope > .ro-plane")) {
    const groups = [...plane.querySelectorAll<HTMLElement>(":scope > .ro-callout")]
      .map((group, order) => ({ group, order, z: Number(group.style.zIndex) || 0 }))
      .sort((a, b) => a.z - b.z || a.order - b.order);
    for (const { group } of groups) {
      const node = group.querySelector<HTMLElement>(":scope > .ro-unit");
      if (!node || node.style.display === "none") continue;
      const opacity = opacityOf(getComputedStyle(group));
      if (opacity <= 0) continue;
      // Everything its contents' look reads, but not where it stands (its
      // own inline transform).
      const signature = [
        group.className,
        group.dataset.overlapping ?? "",
        group.dataset.hovered ?? "",
        node.className,
        node.dataset.zoom ?? "",
        node.innerHTML,
      ].join("|");
      let held = cache.get(node);
      if (held?.signature !== signature) {
        held = { signature, part: cardPart(node, sprites) };
        cache.set(node, held);
      }
      const id = node.dataset.unit ?? node.dataset.enemy ?? node.dataset.contact ?? "";
      const box = node.getBoundingClientRect();
      out.push({
        card: `${node.dataset.owner}${group.dataset.ghost !== undefined ? "-ghost" : ""}-${id}${
          group.dataset.retiring !== undefined ? " retiring" : ""
        }`,
        part: held.part,
        signature,
        at: [box.left, box.top],
        opacity,
        leader: leaderOf(group.querySelector<SVGPathElement>("path.ro-leader")),
      });
    }
  }
  return out;
}
