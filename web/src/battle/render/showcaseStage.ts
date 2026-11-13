/** One unit at rest on a stage of its own, with no battle behind it. The
 *  presentation poses it with the battle's own driver
 *  (`presentation::presentation::showcase`), a squad stands as the simulation
 *  scatters it (`Showground`), and its idle loops are those a stationary
 *  battle vehicle starts. An aircraft stands parked: its rotors still, its
 *  engines off and silent. Each show stands on a presentation and a scene of
 *  its own, as a battle's do: no presentation outlives the scene it fed. */
import type { Vec3 } from "math";
import type { BrowserGpu } from "@wasm/renderer_browser";
import type { Showground } from "@wasm/game_wasm.js";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { CatalogView, Faction } from "@packages/scene-assets/src/units";
import type { WorldLayers, WorldScenery } from "@packages/battle-renderer/src/scene";
import {
  idleLoops,
  type IdleLoop,
  type SoundFrameOptions,
} from "@packages/battle-audio/src/soundFrame";
import {
  ScenePresentation,
  type PresentedCamera,
  type PresentedFrame,
  type ShowcaseInput,
} from "../sim/presentation";
import type { LabModel, PresentedAppearance } from "../sim/presentationModels";
import { fieldUnits, type RuntimeFiles } from "../sim/fielding";
import { BrowserScene, type SceneOptions } from "./scene";

/** The unit a stage shows: a kind, the faction whose look it wears, where it
 *  stands and which way it faces (radians counter-clockwise from +X). */
export interface StageUnit {
  kind: string;
  faction: Faction;
  at: Vec3;
  yaw: number;
}

/** What a shown unit is fielded and heard from: the session's rules (JSON)
 *  and unit catalog, and the battle's sound setup. */
export interface StageFielding {
  rules: string;
  catalog: CatalogView;
  sound: SoundFrameOptions;
}

/** The art one `show` draws: the installed appearances, and the runtime
 *  files they were loaded from, which the shown unit is fielded from. */
export interface StageArt {
  appearances: InstalledAppearances;
  runtime: RuntimeFiles;
}

/** How the stage's scene looks: the page's light, fog and paint, model
 *  detail and overlay glow. */
export type StageLook = Pick<SceneOptions, "settings" | "detail" | "overlayGlow">;

export interface StageOptions {
  gpu: BrowserGpu;
  canvas: HTMLCanvasElement;
  world: WorldLayers;
  /** Static models standing round the unit (a yard's props), as each
   *  show's art draws them. */
  models: (appearances: InstalledAppearances) => readonly LabModel[];
  /** The world's trees, hedgerows and dressing, as each show's art draws
   *  them; null draws none. */
  scenery: (appearances: InstalledAppearances) => WorldScenery | null;
  fielding: StageFielding;
  showground: Showground;
  look: StageLook;
  /** A failure the scene meets while it admits art. */
  failed: (error: unknown) => void;
}

const NO_UI = { selected: [], xray: false, ghosts: [] };

export class ShowcaseStage {
  private scene: BrowserScene | null = null;
  /** The last `show` asked for: an older one finishing late is dropped. */
  private asked = 0;
  private camera: PresentedCamera;
  private size: [number, number] = [0, 0];
  private disposed = false;
  private input: ShowcaseInput | null = null;

  /** The presentation the stage's scene draws from: the last show's to
   *  stand, kept until a newer show stands. */
  private presentation = new ScenePresentation();

  private constructor(private readonly o: StageOptions) {
    const { width, height } = o.canvas;
    this.camera = {
      eye: [-10, -10, 6],
      target: [0, 0, 1],
      up: [0, 0, 1],
      fovY: 0.8,
      near: 0.1,
      width,
      height,
    };
  }

  /** A stage with nothing on it yet: it draws once the first `show` is
   *  on stage. */
  static create(o: StageOptions): ShowcaseStage {
    return new ShowcaseStage(o);
  }

  /** Shows `unit` in place of the one before (null: the ground and models
   *  alone), drawn from `art`, its squad standing as a battle of seed `seed`
   *  fields it; resolves with the loops it idles with, once it is on stage.
   *  A show another supersedes resolves with none. */
  async show(unit: StageUnit | null, seed: number, art: StageArt): Promise<IdleLoop[]> {
    this.assertLive("show");
    const asked = ++this.asked;
    if (!unit) {
      this.input = null;
      await this.rebuild(asked, art.appearances, new ScenePresentation());
      return [];
    }
    const { rules, catalog, sound } = this.o.fielding;
    const fielded = await fieldUnits({
      rules,
      catalog,
      kinds: [unit.kind],
      factions: [unit.faction, unit.faction],
      runtime: art.runtime,
    });
    if (asked !== this.asked || this.disposed) return [];
    const types = (fielded.units as { types: Record<string, { hull: boolean }> }).types;
    const squad: Pick<ShowcaseInput, "soldiers" | "operators"> = types[unit.kind].hull
      ? { soldiers: [], operators: [] }
      : JSON.parse(this.o.showground.squad(unit.kind, seed));
    const [x, y, z] = unit.at;
    this.input = {
      kind: unit.kind,
      side: "blue",
      at: [x, y, z],
      yaw: unit.yaw,
      soldiers: squad.soldiers.map((s) => ({ ...s, at: [x + s.at[0], y + s.at[1]] })),
      operators: squad.operators,
    };
    const presentation = new ScenePresentation();
    try {
      presentation.setShowcase(fielded, this.input);
      await this.rebuild(asked, art.appearances, presentation);
    } catch (error) {
      // A show that never stands frees the presentation it made.
      if (presentation !== this.presentation) presentation.free();
      throw error;
    }
    return asked === this.asked ? idleLoops(sound, unit.kind) : [];
  }

  /** Draws the stage at presentation time `time` (seconds) from `camera`;
   *  false while the scene is still being built for the unit shown. */
  frame(time: number, camera: PresentedCamera): boolean {
    this.assertLive("frame");
    this.camera = camera;
    const scene = this.scene;
    if (!scene || !scene.worldCurrent()) return false;
    const { width, height } = this.o.canvas;
    if (width !== this.size[0] || height !== this.size[1]) {
      this.size = [width, height];
      scene.resize();
    }
    scene.flush();
    this.presentation.setShadow(this.o.look.settings.light, this.o.world.terrain.mesh);
    this.presentation.frame(time, camera, NO_UI, scene.staticGeneration);
    return scene.render();
  }

  /** The frame drawn last and the rows its batches name, for probes: valid
   *  until the next call into the module. */
  records(): { frame: PresentedFrame; rows: readonly PresentedAppearance[] } {
    this.assertLive("records");
    return { frame: this.presentation.lastFrame(), rows: this.presentation.appearanceRows() };
  }

  /** The unit on stage as the presentation stands it; null for none. */
  standing(): ShowcaseInput | null {
    return this.input;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.scene?.dispose();
    this.scene = null;
    this.presentation.free();
  }

  /** A scene built for show `asked` on `presentation`, drawing
   *  `appearances`; it replaces the stage's scene and presentation once it
   *  stands, and is dropped with its presentation if a newer show came. */
  private async rebuild(
    asked: number,
    appearances: InstalledAppearances,
    presentation: ScenePresentation,
  ): Promise<void> {
    const { gpu, canvas, world, models, scenery, look, failed } = this.o;
    this.scene?.dispose();
    this.scene = null;
    const trees = scenery(appearances);
    presentation.setGround(world.terrain.input, trees !== null);
    presentation.setShadow(look.settings.light, world.terrain.mesh);
    presentation.frame(0, this.camera, NO_UI, 0);
    const scene = await BrowserScene.create({
      gpu,
      canvas,
      presentation,
      world: { ...world, scenery: trees },
      ...look,
      appearances,
      models: models(appearances),
      instances: [],
      redraw: () => {},
      failed,
    });
    // A later show, or the stage's end, may have come while it was built.
    if (this.disposed || asked !== this.asked || !scene.worldCurrent()) {
      scene.dispose();
      presentation.free();
      return;
    }
    this.presentation.free();
    this.presentation = presentation;
    this.scene = scene;
    this.size = [0, 0];
  }

  private assertLive(operation: string): void {
    if (this.disposed) throw new Error(`ShowcaseStageDisposed: ${operation}`);
  }
}
