// Shared Rust scenes render sheet tiles; canvas composition owns labels and layout.

import { vec3, type Vec3 } from "math";
import { BrowserScene } from "@web/battle/render/scene";
import type { BrowserGpu } from "@wasm/renderer_browser";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import type { ImpostorAtlas } from "@packages/battle-renderer/src/models/impostor";
import { REST_ARTICULATION, type Articulation } from "@packages/scene-assets/src/articulation";
import type { ReferenceView } from "@packages/scene-assets/src/references";
import { farPoseBounds } from "@packages/scene-assets/src/pose";
import {
  TEXTURE_CHANNELS,
  type Bounds,
  type Bundle,
  type Side,
  type SkeletonClips,
  type TextureChannel,
} from "@packages/scene-assets/src/schema";
import { gameLight } from "../gameLight";
import { ScenePresentation } from "@web/battle/sim/presentation";
import { presentedCamera } from "../presentedFrame";
import { gameFogStyle } from "../gameFog";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "../gameOverlay";
import { gameModelDetail } from "../gameModels";
import { benchOverlay, benchWorld, drawnPose, posedSockets, type BenchPose } from "./benchWorld";
import { sideTint, type LoadedModel } from "./sources";
import {
  REFERENCE_CAMERAS,
  SURFACE_VIEWS,
  WORKBENCH_VIEWS,
  viewCamera,
  type SheetView,
} from "./views";

/** Tile size in pixels. */
export const TILE = 512;
const STRIP_TILE = 256;
const PHASES = 8;
const HEADER = 72;
const LABEL = 22;

export interface StripFrame {
  label: string;
  pose: BenchPose;
  view: SheetView;
}

export interface Strip {
  name: string;
  frames: StripFrame[];
}

type Appearance = Exclude<Bundle, SkeletonClips>;

/** What views frame: the model in its far pose (standing, at rest, intact),
 *  with its ruler when it is judged against one. The bundle's own bounds hold
 *  every pose it can reach — right for culling, too loose for judging (a
 *  tank's swept gun doubles its width). */
export function framingBounds(model: LoadedModel): Bounds {
  const bundle = model.installed.appearances.get(model.name)!.bundle;
  const skeleton =
    bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
  const far = farPoseBounds(bundle, skeleton);
  const ruler = model.body.ruler;
  if (!ruler) return far;
  return {
    min: vec3.min(vec3.create(), far.min, ruler.min),
    max: vec3.max(vec3.create(), far.max, ruler.max),
  };
}

/** Where the scale figure stands for `view`: beside the model, on the screen's
 *  left, clear of its bounds, so no view hides it behind the model. */
export function figureSpot(bounds: Bounds, view: SheetView): [number, number] {
  const yaw = viewCamera(view, bounds).yaw;
  // The eye sits at `yaw` round the target: screen-right is (sin yaw, −cos yaw).
  const rx = Math.sin(yaw);
  const ry = -Math.cos(yaw);
  const cx = (bounds.min[0] + bounds.max[0]) / 2;
  const cy = (bounds.min[1] + bounds.max[1]) / 2;
  const reach =
    (Math.abs(rx) * (bounds.max[0] - bounds.min[0])) / 2 +
    (Math.abs(ry) * (bounds.max[1] - bounds.min[1])) / 2 +
    0.7;
  return [cx + rx * reach, cy + ry * reach];
}

/** The pose a sheet shows the model in: idle for a body, rest for a vehicle. */
export function sheetPose(bundle: Appearance, skeleton: SkeletonClips | null): BenchPose {
  if (bundle.kind === "skinned") {
    const clip = skeleton?.clips.find((c) => c.name === "idle") ?? skeleton?.clips[0];
    return { kind: "skinned", clip: clip?.name ?? "idle", phase: 0, blend: null };
  }
  if (bundle.kind === "articulated")
    return { kind: "articulated", articulation: { ...REST_ARTICULATION } };
  return { kind: "static", state: bundle.states[0].name };
}

const articulated = (a: Partial<Articulation>): BenchPose => ({
  kind: "articulated",
  articulation: { ...REST_ARTICULATION, ...a },
});

/** Animation and vehicle-motion strips sample each authored sequence. */
function sheetStrips(bundle: Appearance, skeleton: SkeletonClips | null): Strip[] {
  const phases = Array.from({ length: PHASES }, (_, k) => k / PHASES);
  if (bundle.kind === "skinned")
    return (skeleton?.clips ?? []).map((clip) => ({
      name: clip.name,
      frames: phases.map((p) => {
        // A one-shot clip's strip ends on its last frame.
        const phase = clip.loop ? p : p * (PHASES / (PHASES - 1));
        return {
          label: `${clip.name} ${phase.toFixed(2)}`,
          pose: { kind: "skinned", clip: clip.name, phase, blend: null },
          // Strides and falls read side-on.
          view: "left",
        };
      }),
    }));
  if (bundle.kind === "articulated") {
    const deg = Math.PI / 180;
    return [
      {
        name: "articulation",
        frames: phases.map((p, k) => ({
          label: `turret ${k * 45}° gun ${(-8 + 28 * p).toFixed(0)}°`,
          pose: articulated({
            turret_yaw: k * 45 * deg,
            gun_pitch: (-8 + 28 * p) * deg,
            hmg_yaw: -k * 60 * deg,
            hmg_pitch: 20 * Math.sin(k) * deg,
          }),
          view: "q-front",
        })),
      },
      {
        name: "running-gear",
        frames: phases.map((_, k) => ({
          label: `travel ${(k * 0.25).toFixed(2)} m`,
          pose: articulated({ travel_l: k * 0.25, travel_r: k * 0.25 }),
          view: "left",
        })),
      },
      {
        name: "deploy",
        frames: phases.map((p) => {
          const deploy = p * (PHASES / (PHASES - 1));
          return {
            label: `deploy ${deploy.toFixed(2)}`,
            pose: articulated({ deploy }),
            view: "q-front",
          };
        }),
      },
    ];
  }
  return [
    {
      name: "states",
      frames: bundle.states.map((s) => ({
        label: s.name,
        pose: { kind: "static", state: s.name },
        view: "q-front",
      })),
    },
  ];
}

export interface SheetResult {
  contact: HTMLCanvasElement;
  strips: { name: string; canvas: HTMLCanvasElement }[];
  /** Close views and each texture channel's part (`renderSurface`). */
  surface: HTMLCanvasElement;
  /** The texture preview, or null for an untextured appearance. */
  textures: HTMLCanvasElement | null;
  stats: Record<string, unknown>;
}

/** A detached canvas and canonical presentation own one independent sheet scene. */
export class SheetRenderer {
  private constructor(
    private readonly presentation: ScenePresentation,
    readonly frame: BrowserScene,
  ) {}

  static async create(gpu: BrowserGpu, model: LoadedModel) {
    const presentation = new ScenePresentation();
    try {
      const world = benchWorld(null);
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = TILE;
      presentation.setGround(world.terrain.input, world.scenery);
      presentation.setShadow(gameLight, world.terrain.mesh);
      presentation.frame(
        0,
        presentedCamera({
          camera3d: viewCamera("q-front", framingBounds(model)),
          width: TILE,
          height: TILE,
        }),
        { selected: [], xray: false, ghosts: [] },
        0,
      );
      const frame = await BrowserScene.create({
        gpu,
        canvas,
        presentation,
        world,
        settings: {
          light: gameLight,
          fog: gameFogStyle,
          paint: gamePaint,
          xrayMinHiddenFraction: gameXrayMinHiddenFragmentFraction,
        },
        detail: JSON.parse(gameModelDetail),
        overlayGlow: gameOverlayGlow,
        appearances: model.installed,
        models: [{ appearance: model.name, x: 0, y: 0, z: 0, yaw: 0, pose: { kind: "rest" } }],
        instances: [],
        redraw: () => {},
        failed: (error) => {
          throw error;
        },
      });
      return new SheetRenderer(presentation, frame);
    } catch (error) {
      presentation.free();
      throw error;
    }
  }

  /** One tile as RGBA pixels. */
  async tile(
    model: LoadedModel,
    pose: BenchPose,
    view: SheetView,
    frameOn: Bounds,
    marks: { hitBox: boolean; sockets: boolean } = { hitBox: true, sockets: true },
    tint?: Vec3,
  ): Promise<ImageData> {
    const bundle = model.installed.appearances.get(model.name)!.bundle;
    const skeleton =
      bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
    const instance: ModelInstance = {
      appearance: model.name,
      x: 0,
      y: 0,
      z: 0,
      yaw: 0,
      pose: drawnPose(bundle, skeleton, pose),
      tint,
    };
    this.frame.setModels([instance]);
    const scale = view.startsWith("battle")
      ? 2.5
      : Math.max(1, vec3.distance(frameOn.min, frameOn.max) / 3);
    this.frame.setOverlay(
      benchOverlay(instance, model.body, posedSockets(bundle, skeleton, pose), marks, scale),
    );
    const world = benchWorld(figureSpot(frameOn, view));
    this.frame.setSkirtAndProps(world.skirt_and_props);
    const camera = viewCamera(view, frameOn);
    this.presentation.setShadow(gameLight, world.terrain.mesh);
    this.frame.flush();
    this.presentation.frame(
      0,
      presentedCamera({ camera3d: camera, width: TILE, height: TILE }),
      { selected: [], xray: false, ghosts: [] },
      this.frame.staticGeneration,
    );
    let drawn = false;
    for (let attempt = 0; attempt < 3; attempt++) {
      if (this.frame.render()) {
        drawn = true;
        break;
      }
      if (attempt < 2) await new Promise<void>((resolve) => setTimeout(resolve, 16));
    }
    if (!drawn)
      throw new Error(`SheetSurfaceUnavailable: ${model.name} (${view}) after 3 attempts`);
    const { rgba, width, height } = await this.frame.readPixels();
    return new ImageData(new Uint8ClampedArray(rgba), width, height);
  }

  dispose() {
    this.frame.dispose();
    this.presentation.free();
  }
}

function canvas(width: number, height: number) {
  const c = document.createElement("canvas");
  c.width = width;
  c.height = height;
  const g = c.getContext("2d")!;
  g.fillStyle = "#15171c";
  g.fillRect(0, 0, width, height);
  return { c, g };
}

function paste(
  g: CanvasRenderingContext2D,
  image: ImageData,
  x: number,
  y: number,
  size: number,
  label: string,
) {
  const tile = document.createElement("canvas");
  tile.width = image.width;
  tile.height = image.height;
  tile.getContext("2d")!.putImageData(image, 0, 0);
  g.drawImage(tile, x, y, size, size);
  g.fillStyle = "rgba(16, 18, 22, 0.72)";
  g.fillRect(x, y, size, LABEL);
  g.fillStyle = "#e8ebef";
  g.font = "14px system-ui, sans-serif";
  g.fillText(label, x + 6, y + 15);
}

function describe(model: LoadedModel, bundle: Appearance): string {
  const tiers =
    model.stats?.tiers.map((t) => t.triangles).join(" / ") ??
    (bundle.kind === "skinned" ? bundle.tiers.map((t) => t.indices.length / 3).join(" / ") : "");
  const m = (b: Bounds) =>
    vec3
      .subtract(vec3.create(), b.max, b.min)
      .map((v) => v.toFixed(2))
      .join(" × ");
  return `${model.name} · ${model.unit} (${bundle.kind}) · triangles ${tiers} · size ${m(framingBounds(model))} m (every pose ${m(bundle.bounds)})`;
}

/** The contact sheet, strips and stats of one loaded model. */
export async function renderSheet(
  gpu: BrowserGpu,
  model: LoadedModel,
  impostor: ImpostorAtlas | null,
  side: Side = "blue",
): Promise<SheetResult> {
  const renderer = await SheetRenderer.create(gpu, model);
  const tint = sideTint(model, side);
  try {
    const entry = model.installed.appearances.get(model.name)!;
    const bundle = entry.bundle;
    const skeleton =
      bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
    const pose = sheetPose(bundle, skeleton);
    const framing = framingBounds(model);
    const findings = model.findings.flatMap((f) => f.findings);
    const errors = findings.filter((f) => f.severity === "error").length;

    const cols = 4;
    const { c: contact, g } = canvas(cols * TILE, HEADER + 2 * TILE);
    g.fillStyle = "#e8ebef";
    g.font = "bold 22px system-ui, sans-serif";
    g.fillText(`${describe(model, bundle)}${tint ? ` · ${side}` : ""}`, 12, 30);
    g.font = "15px system-ui, sans-serif";
    g.fillStyle = errors ? "#ff9d8f" : "#a9d8a0";
    g.fillText(
      `${errors} error(s), ${findings.length - errors} warning(s)${findings.length ? ": " + [...new Set(findings.map((f) => f.code))].join(", ") : ""} · grey figure 1.8 m · magenta: ${model.body.label} · RGB: sockets`,
      12,
      56,
    );
    for (const [i, view] of WORKBENCH_VIEWS.entries()) {
      const image = await renderer.tile(model, pose, view, framing, undefined, tint);
      paste(g, image, (i % cols) * TILE, HEADER + Math.floor(i / cols) * TILE, TILE, view);
    }

    const strips: SheetResult["strips"] = [];
    for (const strip of sheetStrips(bundle, skeleton)) {
      const { c, g: sg } = canvas(strip.frames.length * STRIP_TILE, 34 + STRIP_TILE);
      sg.fillStyle = "#e8ebef";
      sg.font = "bold 16px system-ui, sans-serif";
      sg.fillText(`${model.name} · ${strip.name}`, 8, 22);
      for (const [k, frame] of strip.frames.entries()) {
        // A one-shot body clip (the fall) frames every pose; loops frame the stand.
        const pose = frame.pose;
        const oneShot =
          pose.kind === "skinned" && !skeleton?.clips.find((c) => c.name === pose.clip)?.loop;
        const on = bundle.kind === "skinned" && oneShot ? bundle.bounds : framing;
        const image = await renderer.tile(
          model,
          frame.pose,
          frame.view,
          on,
          { hitBox: true, sockets: false },
          tint,
        );
        paste(sg, image, k * STRIP_TILE, 34, STRIP_TILE, frame.label);
      }
      strips.push({ name: strip.name, canvas: c });
    }

    const surface = await renderSurface(renderer, model, pose, framing, tint);

    const stats = {
      appearance: model.name,
      source: model.source,
      unit: model.unit,
      kind: bundle.kind,
      tiers: model.stats?.tiers ?? null,
      joints: bundle.kind === "skinned" ? bundle.joints.length : undefined,
      nodes: bundle.kind === "articulated" ? bundle.nodes.length : undefined,
      clips: skeleton?.clips.map((c) => ({
        name: c.name,
        loop: c.loop,
        duration: c.duration,
        frames: c.frames,
      })),
      bounds: bundle.bounds,
      far_pose_bounds: farPoseBounds(bundle, skeleton),
      findings: model.findings,
      load_ms: Math.round(model.loadMs),
      frame: renderer.frame.stats().models,
      impostor: impostor
        ? {
            hash: impostor.hash,
            width: impostor.width,
            height: impostor.height,
            spec: impostor.spec,
          }
        : null,
    };
    return { contact, strips, surface, textures: textureSheet(model.name, bundle), stats };
  } finally {
    renderer.dispose();
  }
}

/** Channel switches the surface sheet compares, each against all on. */
const CHANNEL_TILES: { label: string; channels: Partial<Record<TextureChannel, boolean>> }[] = [
  { label: "surface-front · no albedo texture", channels: { albedo: false } },
  { label: "surface-front · no normal map", channels: { normal: false } },
  { label: "surface-front · no ORM", channels: { orm: false } },
];

/** The surface sheet: close views with every texture channel, the same with
 *  none, one channel off at a time (what each texture adds), and the battle
 *  views as a player sees them. No marks. */
async function renderSurface(
  renderer: SheetRenderer,
  model: LoadedModel,
  pose: BenchPose,
  framing: Bounds,
  tint: Vec3 | undefined,
): Promise<HTMLCanvasElement> {
  const cols = 3;
  const { c, g } = canvas(cols * TILE, HEADER + 3 * TILE);
  g.fillStyle = "#e8ebef";
  g.font = "bold 22px system-ui, sans-serif";
  g.fillText(`${model.name} · surface: close views and texture channels`, 12, 30);
  const marks = { hitBox: false, sockets: false };
  const tiles: {
    label: string;
    view: SheetView;
    channels: Partial<Record<TextureChannel, boolean>>;
  }[] = [
    ...SURFACE_VIEWS.map((view) => ({ label: view, view, channels: {} })),
    { label: "surface-front · no textures", view: "surface-front", channels: NO_CHANNELS },
    ...CHANNEL_TILES.map((t) => ({ ...t, view: "surface-front" as const })),
    ...(["battle-near", "battle-mid", "battle-far"] as const).map((view) => ({
      label: `${view} · no marks`,
      view,
      channels: {},
    })),
  ];
  try {
    for (const [i, tile] of tiles.entries()) {
      renderer.frame.setTextureChannels({ ...ALL_CHANNELS, ...tile.channels });
      const image = await renderer.tile(model, pose, tile.view, framing, marks, tint);
      paste(g, image, (i % cols) * TILE, HEADER + Math.floor(i / cols) * TILE, TILE, tile.label);
    }
  } finally {
    renderer.frame.setTextureChannels(ALL_CHANNELS);
  }
  return c;
}

/** One row of the reference sheet: a reference view, and the image that shows
 *  it (a data URL), or null where the library has none. */
export interface ReferenceRow {
  view: ReferenceView;
  image: string | null;
  /** The file, its source, author and licence, and how many more show the
   *  view; with no image, why it is missing. */
  label: string;
  generated: boolean;
}

/** The reference sheet: each reference view's studio view of the model, with
 *  no marks, beside the reference that shows it. A view no reference shows is
 *  marked missing, a generated one generated. */
export async function renderReferenceSheet(
  gpu: BrowserGpu,
  model: LoadedModel,
  rows: ReferenceRow[],
  title: string,
  side: Side = "blue",
): Promise<HTMLCanvasElement> {
  const wide = Math.round(TILE * 1.5);
  const { c, g } = canvas(TILE + wide, HEADER + rows.length * TILE);
  g.fillStyle = "#e8ebef";
  g.font = "bold 22px system-ui, sans-serif";
  g.fillText(`${model.name} · ${title}`, 12, 30);
  g.font = "15px system-ui, sans-serif";
  g.fillText("left: the model, no marks · right: the reference of the same view", 12, 56);
  const renderer = await SheetRenderer.create(gpu, model);
  try {
    const bundle = model.installed.appearances.get(model.name)!.bundle;
    const skeleton =
      bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
    const pose = sheetPose(bundle, skeleton);
    const framing = framingBounds(model);
    const tint = sideTint(model, side);
    for (const [i, row] of rows.entries()) {
      const y = HEADER + i * TILE;
      const camera = REFERENCE_CAMERAS[row.view];
      if (camera) {
        const marks = { hitBox: false, sockets: false };
        const image = await renderer.tile(model, pose, camera, framing, marks, tint);
        paste(g, image, 0, y, TILE, `${row.view} · ${camera}`);
      } else banner(g, 0, y, TILE, `${row.view} · no model view`, "#8a9099");
      if (row.image) {
        const picture = await createImageBitmap(await (await fetch(row.image)).blob());
        const scale = Math.min(wide / picture.width, TILE / picture.height);
        const w = picture.width * scale;
        const h = picture.height * scale;
        g.drawImage(picture, TILE + (wide - w) / 2, y + (TILE - h) / 2, w, h);
        picture.close();
        g.fillStyle = "rgba(16, 18, 22, 0.72)";
        g.fillRect(TILE, y + TILE - LABEL, wide, LABEL);
        g.fillStyle = "#e8ebef";
        g.font = "13px system-ui, sans-serif";
        g.fillText(row.label, TILE + 6, y + TILE - 6, wide - 12);
        if (row.generated)
          banner(g, TILE, y, wide, "GENERATED: layout and proportions only", "#f2b84b");
      } else banner(g, TILE, y, wide, row.label, "#ff9d8f");
    }
  } finally {
    renderer.dispose();
  }
  return c;
}

/** A label strip across the top of a cell, in `colour`. */
function banner(
  g: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  text: string,
  colour: string,
) {
  g.fillStyle = "rgba(16, 18, 22, 0.85)";
  g.fillRect(x, y, width, LABEL + 6);
  g.fillStyle = colour;
  g.font = "bold 15px system-ui, sans-serif";
  g.fillText(text, x + 6, y + 18);
}

const ALL_CHANNELS = { albedo: true, normal: true, orm: true };
const NO_CHANNELS = { albedo: false, normal: false, orm: false };

/** The texture preview: each textured material's channels as the bundle
 *  carries them (albedo over its wear threshold, normal, ORM over the tint
 *  mask), with the material's name. */
export function textureSheet(name: string, bundle: Appearance): HTMLCanvasElement | null {
  const textured = bundle.materials.filter((m) => m.textures && Object.keys(m.textures).length);
  if (!textured.length) return null;
  const cell = 160;
  const columns = TEXTURE_CHANNELS.length * 2;
  const { c, g } = canvas(columns * cell, HEADER + textured.length * (cell + LABEL));
  g.fillStyle = "#e8ebef";
  g.font = "bold 20px system-ui, sans-serif";
  g.fillText(`${name} · textures (colour, alpha per channel)`, 12, 30);
  g.font = "13px system-ui, sans-serif";
  g.fillText(
    "albedo · wear threshold · normal · — · ORM (occlusion, roughness, metalness) · tint mask",
    12,
    56,
  );
  textured.forEach((m, row) => {
    const y = HEADER + row * (cell + LABEL);
    g.fillStyle = "#e8ebef";
    g.fillText(`${m.name} (${Object.keys(m.textures!).join(", ")})`, 6, y + 15);
    TEXTURE_CHANNELS.forEach((channel, k) => {
      const index = m.textures![channel];
      if (index === undefined) return;
      const t = bundle.textures[index];
      for (const alpha of [false, true]) {
        const rgba = new Uint8ClampedArray(t.levels[0].length);
        for (let p = 0; p < rgba.length; p += 4) {
          const a = t.levels[0][p + 3];
          for (let ch = 0; ch < 3; ch++) rgba[p + ch] = alpha ? a : t.levels[0][p + ch];
          rgba[p + 3] = 255;
        }
        const tile = document.createElement("canvas");
        tile.width = t.width;
        tile.height = t.height;
        tile.getContext("2d")!.putImageData(new ImageData(rgba, t.width, t.height), 0, 0);
        g.drawImage(tile, (k * 2 + (alpha ? 1 : 0)) * cell, y + LABEL, cell, cell);
      }
    });
  });
  return c;
}

/** An RGBA atlas as a canvas, over a checker so coverage reads. */
export function atlasCanvas(width: number, height: number, rgba: Uint8Array): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = width;
  c.height = height;
  c.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);
  return c;
}
