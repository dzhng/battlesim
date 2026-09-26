// Model sheets: every view needed to judge a model, rendered by the
// production battle frame (a second `BattleFrame` on the viewport's device, at
// a fixed tile size) and laid out with labels on a 2D canvas.
//
// - The contact sheet: the eight named views in one pose, with the scale
//   figure, the hit box and the socket gizmos.
// - Strips (spike 03's shape): eight phases per clip for a body; for a
//   vehicle, articulation (turret, gun, HMG), running gear and deploy.
// - Stats: tiers, joints or nodes, clips, bounds, findings, frame numbers.

import { vec3 } from "math";
import { createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import type { BattleFrame } from "@packages/battle-renderer/src/scene";
import type { ModelInstance, ModelPose } from "@packages/battle-renderer/src/models/modelInstances";
import type { ImpostorAtlas } from "@packages/battle-renderer/src/models/impostor";
import { REST_ARTICULATION, type Articulation } from "@packages/scene-assets/src/articulation";
import { farPoseBounds } from "@packages/scene-assets/src/pose";
import type { Bounds, Bundle, SkeletonClips } from "@packages/scene-assets/src/schema";
import { villageLight } from "../villageLight";
import { villageFogGeometry } from "../villageFog";
import { benchOverlay, benchWorld, posedSockets } from "./benchWorld";
import type { LoadedModel } from "./sources";
import { WORKBENCH_VIEWS, viewCamera, type WorkbenchView } from "./views";

/** Tile size in pixels; a multiple of 64 so a row of texels is 256-aligned. */
export const TILE = 512;
const STRIP_TILE = 256;
const PHASES = 8;
const HEADER = 72;
const LABEL = 22;

export interface StripFrame {
  label: string;
  pose: ModelPose;
  view: WorkbenchView;
}

export interface Strip {
  name: string;
  frames: StripFrame[];
}

type Appearance = Exclude<Bundle, SkeletonClips>;

/** What views frame: the model in its far pose (standing, at rest, intact).
 *  The bundle's own bounds hold every pose it can reach — right for culling,
 *  too loose for judging (a tank's swept gun doubles its width). */
export function framingBounds(model: LoadedModel): Bounds {
  const bundle = model.installed.appearances.get(model.name)!.bundle;
  const skeleton =
    bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
  return farPoseBounds(bundle, skeleton);
}

/** Where the scale figure stands for `view`: beside the model, on the screen's
 *  left, clear of its bounds, so no view hides it behind the model. */
export function figureSpot(bounds: Bounds, view: WorkbenchView): [number, number] {
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
export function sheetPose(bundle: Appearance, skeleton: SkeletonClips | null): ModelPose {
  if (bundle.kind === "skinned") {
    const clip = skeleton?.clips.find((c) => c.name === "idle") ?? skeleton?.clips[0];
    return { kind: "skinned", clip: clip?.name ?? "idle", phase: 0, blend: null };
  }
  if (bundle.kind === "articulated")
    return { kind: "articulated", articulation: { ...REST_ARTICULATION } };
  return { kind: "static", state: bundle.states[0].name };
}

const articulated = (a: Partial<Articulation>): ModelPose => ({
  kind: "articulated",
  articulation: { ...REST_ARTICULATION, ...a },
});

/** Spike 03's strips: eight phases of each clip, or of each vehicle motion. */
export function sheetStrips(bundle: Appearance, skeleton: SkeletonClips | null): Strip[] {
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
  stats: Record<string, unknown>;
}

/** Renders tiles with a battle frame of its own on the viewport's device. */
export class SheetRenderer {
  private target: GPUTexture;
  private read: GPUBuffer;

  private constructor(
    private readonly device: GPUDevice,
    private readonly format: GPUTextureFormat,
    readonly frame: BattleFrame,
  ) {
    this.target = device.createTexture({
      label: "sheet-tile",
      size: [TILE, TILE],
      format,
      usage: 0x10 | 0x01, // RENDER_ATTACHMENT | COPY_SRC
    });
    this.read = device.createBuffer({
      label: "sheet-read",
      size: TILE * TILE * 4,
      usage: 0x01 | 0x08,
    });
  }

  static async create(device: GPUDevice, format: GPUTextureFormat, model: LoadedModel) {
    const frame = await createBattleFrame(device, format, {
      light: villageLight,
      fogGeometry: villageFogGeometry,
      world: benchWorld(null),
      instances: [],
      width: TILE,
      height: TILE,
    });
    await frame.setAppearances(model.installed);
    return new SheetRenderer(device, format, frame);
  }

  /** One tile as RGBA pixels. */
  async tile(
    model: LoadedModel,
    pose: ModelPose,
    view: WorkbenchView,
    frameOn: Bounds,
    marks: { hitBox: boolean; sockets: boolean } = { hitBox: true, sockets: true },
  ): Promise<ImageData> {
    const bundle = model.installed.appearances.get(model.name)!.bundle;
    const skeleton =
      bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
    const instance: ModelInstance = { appearance: model.name, x: 0, y: 0, z: 0, yaw: 0, pose };
    this.frame.setModels([instance]);
    const scale = view.startsWith("battle")
      ? 2.5
      : Math.max(1, vec3.distance(frameOn.min, frameOn.max) / 3);
    this.frame.setOverlay(
      benchOverlay(instance, model.body, posedSockets(bundle, skeleton, pose), marks, scale),
    );
    this.frame.setWorld(benchWorld(figureSpot(frameOn, view)));
    const camera = viewCamera(view, frameOn);
    this.frame.render(this.target.createView(), { camera3d: camera, width: TILE, height: TILE });
    const encoder = this.device.createCommandEncoder();
    encoder.copyTextureToBuffer(
      { texture: this.target },
      { buffer: this.read, bytesPerRow: TILE * 4 },
      [TILE, TILE],
    );
    this.device.queue.submit([encoder.finish()]);
    await this.read.mapAsync(1);
    const bytes = new Uint8ClampedArray(this.read.getMappedRange().slice(0));
    this.read.unmap();
    if (this.format.startsWith("bgra"))
      for (let i = 0; i < bytes.length; i += 4) {
        const b = bytes[i];
        bytes[i] = bytes[i + 2];
        bytes[i + 2] = b;
      }
    return new ImageData(bytes, TILE, TILE);
  }

  dispose() {
    this.frame.dispose();
    this.target.destroy();
    this.read.destroy();
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
  device: GPUDevice,
  format: GPUTextureFormat,
  model: LoadedModel,
  impostor: ImpostorAtlas | null,
): Promise<SheetResult> {
  const renderer = await SheetRenderer.create(device, format, model);
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
    g.fillText(describe(model, bundle), 12, 30);
    g.font = "15px system-ui, sans-serif";
    g.fillStyle = errors ? "#ff9d8f" : "#a9d8a0";
    g.fillText(
      `${errors} error(s), ${findings.length - errors} warning(s)${findings.length ? ": " + [...new Set(findings.map((f) => f.code))].join(", ") : ""} · grey figure 1.8 m · magenta: ${model.body.label} · RGB: sockets`,
      12,
      56,
    );
    for (const [i, view] of WORKBENCH_VIEWS.entries()) {
      const image = await renderer.tile(model, pose, view, framing);
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
        const image = await renderer.tile(model, frame.pose, frame.view, on, {
          hitBox: true,
          sockets: false,
        });
        paste(sg, image, k * STRIP_TILE, 34, STRIP_TILE, frame.label);
      }
      strips.push({ name: strip.name, canvas: c });
    }

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
    return { contact, strips, stats };
  } finally {
    renderer.dispose();
  }
}

/** An RGBA atlas as a canvas, over a checker so coverage reads. */
export function atlasCanvas(width: number, height: number, rgba: Uint8Array): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = width;
  c.height = height;
  c.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);
  return c;
}
