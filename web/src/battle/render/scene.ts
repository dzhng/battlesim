/** Browser scene lifetime and transport. Rust owns all GPU rendering policy. */
import { Renderer, type BrowserGpu } from "@wasm/renderer_browser";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { TextureChannel } from "@packages/scene-assets/src/schema";
import type { WorldLayers, WorldMeshes, SceneInstance } from "@packages/battle-renderer/src/scene";
import type { ImpostorAtlas, ImpostorSpec } from "@packages/battle-renderer/src/models/impostor";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type { FogStyle } from "@packages/battle-renderer/src/fog";
import type { OverlayGlowStyle } from "@packages/battle-renderer/src/overlayStyle";
import type { PaintStyle } from "@packages/battle-renderer/src/paintStyle";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import type { ScenePresentation } from "../sim/presentation";
import type { LabModel, ModelDetail } from "../sim/presentationModels";
import { modelAssetsOf, type ModelAsset } from "./modelAssets";
import { proxyModels } from "./proxyModels";
import { SceneAppearances } from "./sceneAppearances";
import { loadEffectAtlasLayers } from "./effectAssets";
import { installSceneGeometry } from "./sceneGeometry";
import type { SceneStats, DrawnInstances, FrameView } from "./sceneStats";
import type { FogProbes } from "@packages/battle-renderer/src/fog";
import type { GrassProbes } from "@packages/battle-renderer/src/grass";
import type { SurfaceClass } from "@packages/battle-renderer/src/models/surfaceParts";

const EMPTY_MESH = new Float32Array(0);

export interface SceneControls {
  view?: FrameView;
  paint?: boolean;
  grass?: boolean;
  trees?: boolean;
  buildings?: boolean;
  roadWear?: boolean;
  fieldTexture?: boolean;
  castLights?: boolean;
  xrayCoverage?: boolean;
  models?: boolean;
  effects?: boolean;
  scars?: boolean;
  textureChannels?: [boolean, boolean, boolean];
  surfaces?: [boolean, boolean, boolean, boolean];
}

export interface SceneSettings {
  light: LightPresentation;
  fog: FogStyle;
  grassDensity?: number;
  paint?: PaintStyle;
  xrayMinHiddenFraction?: number;
  /** Refracted water, or its depth colour alone (the lowest tier). */
  waterDetail?: "full" | "depth";
}

export interface PixelSnapshot {
  width: number;
  height: number;
  rgba: Uint8Array;
}

export interface SceneOptions {
  gpu: BrowserGpu;
  canvas: HTMLCanvasElement;
  highCanvas?: HTMLCanvasElement;
  presentation: ScenePresentation;
  world: WorldLayers;
  settings: SceneSettings;
  detail: ModelDetail;
  overlayGlow: OverlayGlowStyle;
  appearances: InstalledAppearances | null;
  models: readonly LabModel[];
  instances: readonly SceneInstance[];
  redraw: () => void;
  failed: (error: unknown) => void;
}

/** Independent outputs share the page GPU. Each scene has one canonical
 * presentation and one live runtime, plus at most one candidate asset group. */
export class BrowserScene {
  private readonly admission: SceneAppearances;
  private authored: ModelAsset[];
  private proxies: ReturnType<typeof proxyModels>;
  private models: readonly LabModel[];
  private instances: readonly SceneInstance[];
  private desired: InstalledAppearances | null;
  private installed: InstalledAppearances | null;
  private activeWorld: WorldLayers;
  private desiredWorld: WorldLayers;
  private admittedWorld: ReturnType<ScenePresentation["world"]>;
  private readonly hasGrass: boolean;
  private controls: SceneControls = {};
  private overlayGlow: OverlayGlowStyle;
  private disposed = false;
  private readonly redraw: () => void;
  private grassRules: ReturnType<GrassProbes["rules"]>;
  readonly fogProbes: FogProbes;
  readonly grassProbes: GrassProbes;

  private constructor(
    readonly renderer: Renderer,
    readonly presentation: ScenePresentation,
    readonly canvas: HTMLCanvasElement,
    options: SceneOptions,
    authored: ModelAsset[],
    proxies: ReturnType<typeof proxyModels>,
    assets: readonly ModelAsset[],
    admittedWorld: ReturnType<ScenePresentation["world"]>,
  ) {
    this.authored = authored;
    this.overlayGlow = options.overlayGlow;
    this.redraw = options.redraw;
    const defaultGrass = options.world.terrain.biome.grass;
    this.grassRules = options.world.grass ? defaultGrass : null;
    this.proxies = proxies;
    this.models = options.models;
    this.instances = options.instances;
    this.desired = this.installed = options.appearances;
    this.activeWorld = this.desiredWorld = options.world;
    this.admittedWorld = admittedWorld;
    this.hasGrass = options.settings.grassDensity !== undefined;
    this.admission = new SceneAppearances(
      renderer,
      presentation,
      options.detail,
      options.redraw,
      options.failed,
      { assets, models: this.modelList() },
    );
    this.fogProbes = {
      probe: (points) => presentation.probeFog(renderer, points),
      probeShape: (rows) => renderer.fog_shape(rows),
      tileCounts: () => renderer.fog_tile_counts(),
      rebuildAll: () => presentation.presentedGround()?.rebuild_fog(),
      wholes: async () => {
        const boxes = presentation.fogOccluders();
        return { boxes, seen: await renderer.fog_whole_flags(boxes.length) };
      },
    };
    this.grassProbes = {
      suppress: (on) => this.setControls({ grass: !on }),
      retune: (rules) => {
        presentation.presentedGround()?.retune_grass(rules ? JSON.stringify(rules) : undefined);
        this.grassRules = rules ?? defaultGrass;
        void this.request();
      },
      rules: () => this.grassRules,
      counts: () => renderer.grass_counts(),
      clumps: () => presentation.grassClumps(renderer),
    };
  }

  static async create(options: SceneOptions): Promise<BrowserScene> {
    const { gpu, canvas, highCanvas, presentation, appearances, settings } = options;
    const renderer = await Renderer.new(gpu, canvas, JSON.stringify(settings), highCanvas);
    try {
      const authored = assetsFor(
        presentation,
        appearances,
        options.models,
        options.world,
        options.settings.grassDensity !== undefined,
      );
      const proxies = proxyModels(options.instances, appearances?.appearances.keys());
      const assets = [...authored, ...proxies.assets];
      // Initial admission is private until every asset is ready. Live changes
      // use the asset-only transaction below and retain the current output.
      presentation.installLabAssets(assets, options.detail);
      presentation.setLabModels([...options.models, ...proxies.models]);
      for (const asset of assets)
        renderer.load_bundle(asset.name, asset.bundle, asset.textureIds, asset.textureBytes);
      renderer.load_effect_atlas(await loadEffectAtlasLayers());
      installSceneGeometry(renderer, presentation, options.world);
      renderer.set_overlay_glow(options.overlayGlow.radius_px, options.overlayGlow.strength);
      presentation.installRenderer(renderer);
      const admittedWorld = presentation.world();
      await renderer.prepare_assets();
      renderer.set_gpu_timing(true);
      return new BrowserScene(
        renderer,
        presentation,
        canvas,
        options,
        authored,
        proxies,
        assets,
        admittedWorld,
      );
    } catch (error) {
      renderer.free();
      throw error;
    }
  }

  /** Authored static placement reads only the committed appearance generation. */
  appearances(): InstalledAppearances | null {
    return this.installed;
  }

  /** Reconcile one complete desired input tuple after canonical framing.
   * A topology replacement bypasses asset staging and rebuilds directly. */
  updateSources(
    world: WorldLayers,
    installed: InstalledAppearances | null,
    models: readonly LabModel[],
    instances: readonly SceneInstance[],
  ): "rebuild" | "reframe" | "current" {
    const previous = this.desiredWorld;
    if (
      world.terrain.mesh !== previous.terrain.mesh ||
      world.terrain.input !== previous.terrain.input ||
      world.backdropTerrain !== previous.backdropTerrain ||
      !!world.grass !== !!previous.grass
    )
      return "rebuild";
    const artChanged =
      installed !== this.desired ||
      world.scenery !== previous.scenery ||
      world.grass !== previous.grass;
    const modelsChanged = models !== this.models;
    const instancesChanged = instances !== this.instances;
    let proxiesChanged = false;
    if (instancesChanged || installed !== this.desired) {
      const next = proxyModels(instances, installed?.appearances.keys());
      proxiesChanged =
        next.assets.length !== this.proxies.assets.length ||
        next.assets.some((asset, i) => asset.name !== this.proxies.assets[i].name);
      if (!proxiesChanged) next.assets = this.proxies.assets;
      this.proxies = next;
    }
    this.models = models;
    this.instances = instances;
    this.desired = installed;
    this.desiredWorld = world;
    const sourcesChanged =
      artChanged ||
      (modelsChanged &&
        !sameNames(sourceNames(this.presentation, models, this.hasGrass), this.authored));
    if (sourcesChanged)
      this.authored = assetsFor(this.presentation, installed, models, world, this.hasGrass);
    if (sourcesChanged || proxiesChanged) void this.request();
    else if (modelsChanged || instancesChanged) {
      this.admission.models(this.modelList());
      return "reframe";
    }
    return "current";
  }

  setModels(models: readonly LabModel[]): void {
    this.models = models;
    // React source publication and model props may arrive in either order.
    // A new-name request stays desired until its source generation arrives.
    if (models.some((model) => !this.desired?.appearances.has(model.appearance))) return;
    const names = sourceNames(this.presentation, models, this.hasGrass);
    if (!sameNames(names, this.authored)) {
      this.authored = assetsFor(
        this.presentation,
        this.desired,
        models,
        this.desiredWorld,
        this.hasGrass,
      );
      void this.request();
      return;
    }
    this.admission.models(this.modelList());
  }

  setInstances(instances: readonly SceneInstance[]): void {
    this.instances = instances;
    const next = proxyModels(instances, this.desired?.appearances.keys());
    const same =
      next.assets.length === this.proxies.assets.length &&
      next.assets.every((asset, i) => asset.name === this.proxies.assets[i].name);
    if (same) next.assets = this.proxies.assets;
    this.proxies = next;
    if (same) this.admission.models(this.modelList());
    else void this.request();
  }

  /** Apply ready appearance work before publishing the next canonical frame. */
  flush(): boolean {
    if (!this.worldCurrent()) return false;
    const changed = this.admission.flush();
    if (changed) {
      this.installed = this.desired;
      this.presentation.adoptRendererAppearanceSource(this.installed, this.desiredWorld.scenery);
      this.admittedWorld = this.presentation.world();
    }
    if (!changed && !this.admission.current()) return false;
    const propsChanged = this.activeWorld.props !== this.desiredWorld.props;
    if (propsChanged) this.setWorldProps(this.desiredWorld.props);
    this.activeWorld = this.desiredWorld;
    return changed || propsChanged;
  }

  world(): WorldLayers {
    return this.activeWorld;
  }

  /** An external canonical reload requires complete GPU re-admission. */
  worldCurrent(): boolean {
    return this.presentation.world() === this.admittedWorld;
  }

  get staticGeneration(): number {
    return this.renderer.static_generation();
  }

  render(): boolean {
    return !this.disposed && this.presentation.render(this.renderer) !== null;
  }

  current(): boolean {
    return (
      this.worldCurrent() && this.admission.current() && this.activeWorld === this.desiredWorld
    );
  }

  inheritControls(previous: BrowserScene): void {
    this.setControls(previous.controls);
    this.setOverlayGlow(previous.overlayGlow);
  }

  resize(): void {
    this.presentation.resizeRenderer(this.renderer, this.canvas);
  }

  setOverlay(meshes: WorldMeshes): void {
    this.renderer.set_overlays(
      meshes.opaque,
      meshes.translucent,
      meshes.painted ?? EMPTY_MESH,
      meshes.paintedMarching ?? EMPTY_MESH,
    );
  }

  setWorldProps(mesh: Mesh): void {
    this.renderer.set_world_props(mesh);
  }

  setPointerMarks(mesh: Mesh): void {
    this.renderer.set_pointer_marks(mesh);
  }

  setControls(controls: SceneControls): void {
    this.controls = { ...this.controls, ...controls };
    this.presentation.setRendererControls(this.renderer, this.controls);
    this.redraw();
  }

  setTextureChannels(channels: Partial<Record<TextureChannel, boolean>>): void {
    const current = this.controls.textureChannels ?? [true, true, true];
    this.setControls({
      textureChannels: [
        channels.albedo ?? current[0],
        channels.normal ?? current[1],
        channels.orm ?? current[2],
      ],
    });
  }

  setSurfaceShown(surface: SurfaceClass, on: boolean): void {
    const surfaces: [boolean, boolean, boolean, boolean] = [
      ...(this.controls.surfaces ?? [true, true, true, true]),
    ];
    const index = { opaque: 0, room: 1, cutout: 2, blended: 3 }[surface];
    surfaces[index] = on;
    this.setControls({ surfaces });
  }

  setFogStyle(style: FogStyle): void {
    this.renderer.set_fog_style(JSON.stringify(style));
  }

  setOverlayGlow(style: OverlayGlowStyle): void {
    this.overlayGlow = style;
    this.renderer.set_overlay_glow(style.radius_px, style.strength);
  }

  readPixels(): Promise<PixelSnapshot> {
    return this.renderer.read_rgba(false);
  }

  bakeImpostor(name: string, spec?: ImpostorSpec): Promise<ImpostorAtlas> {
    const row = this.presentation.appearanceRows().findIndex((row) => row.name === name);
    if (row < 0) return Promise.reject(new Error(`appearance ${name} is not committed`));
    return this.renderer.bake_impostor(row, JSON.stringify(spec ?? null));
  }

  settled(): Promise<boolean> {
    return this.admission.settled();
  }
  resetGpuTiming(): void {
    this.renderer.reset_gpu_timing();
  }
  drain(): Promise<void> {
    return this.renderer.drain();
  }
  stats(): SceneStats {
    return JSON.parse(this.renderer.frame_stats()) as SceneStats;
  }
  drawnInstances(): DrawnInstances {
    return JSON.parse(this.renderer.drawn_instances()) as DrawnInstances;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.admission.dispose();
    this.renderer.free();
  }

  private modelList(): readonly LabModel[] {
    return [...this.models, ...this.proxies.models];
  }

  private request(): Promise<boolean> {
    return this.admission.request([...this.authored, ...this.proxies.assets], this.modelList());
  }
}

/** Source admission follows the canonical world and current requests. A loaded
 * game catalog may contain far more art than this scene can ever draw. */
function sourceNames(
  presentation: ScenePresentation,
  models: readonly LabModel[],
  grass: boolean,
): string[] {
  const names = new Set(presentation.canonicalAppearanceRows().map((row) => row.name));
  for (const model of models) names.add(model.appearance);
  const ground = presentation.presentedGround();
  if (ground && grass)
    for (const name of JSON.parse(ground.grass_kinds()) as string[]) names.add(name);
  return [...names].sort();
}

function assetsFor(
  presentation: ScenePresentation,
  installed: InstalledAppearances | null,
  models: readonly LabModel[],
  world: WorldLayers,
  grass: boolean,
): ModelAsset[] {
  const sources = [world.scenery?.appearances, grass ? world.grass : null].filter(
    (source): source is NonNullable<typeof source> => source != null,
  );
  return modelAssetsOf(installed, sourceNames(presentation, models, grass), sources);
}

function sameNames(names: readonly string[], assets: readonly ModelAsset[]): boolean {
  return names.length === assets.length && names.every((name, i) => name === assets[i].name);
}

/** CPU export keeps GPU snapshots independent of the live scene lifetime. */
export async function pixelsPng(snapshot: Promise<PixelSnapshot>): Promise<Blob> {
  const { width, height, rgba } = await snapshot;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  canvas
    .getContext("2d")!
    .putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);
  return new Promise((resolve, reject) =>
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error("could not encode renderer pixels as PNG"));
    }),
  );
}
