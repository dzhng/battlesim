/** Bounded complete-generation admission at the browser's frame boundary. */
import type { PreparedAppearances, Renderer } from "@wasm/renderer_browser";
import type { ScenePresentation } from "../sim/presentation";
import type { LabModel, ModelDetail } from "../sim/presentationModels";
import type { ModelAsset } from "./modelAssets";

interface Request {
  assets: readonly ModelAsset[];
  models: readonly LabModel[];
  done: Promise<boolean>;
  resolve: (committed: boolean) => void;
  completed: boolean;
  error?: unknown;
}

/** Only asset resources are staged. The live presentation keeps observing and
 * framing its committed catalog while one candidate performs real GPU work. */
export class SceneAppearances {
  private desired: Request | null = null;
  private active: Request | null = null;
  private building: Request | null = null;
  private ready: { request: Request; assets: PreparedAppearances } | null = null;
  private disposed = false;

  constructor(
    private readonly renderer: Renderer,
    private readonly presentation: ScenePresentation,
    private readonly detail: ModelDetail,
    private readonly redraw: () => void,
    private readonly failed: (error: unknown) => void,
    initial?: { assets: readonly ModelAsset[]; models: readonly LabModel[] },
  ) {
    if (initial)
      this.active = this.desired = {
        ...initial,
        completed: true,
        done: Promise.resolve(true),
        resolve: () => {},
      };
  }

  /** A newer catalog supersedes outstanding work without starting a second
   * candidate. A false result means this request never became current. */
  request(assets: readonly ModelAsset[], models: readonly LabModel[]): Promise<boolean> {
    if (this.disposed) return Promise.resolve(false);
    if (this.desired?.assets === assets) {
      this.models(models);
      return this.desired.done;
    }
    if (this.desired) this.finish(this.desired, false);
    this.discardReady();
    let resolve!: Request["resolve"];
    const done = new Promise<boolean>((complete) => {
      resolve = complete;
    });
    this.desired = { assets, models, done, resolve, completed: false };
    this.start();
    return done;
  }

  /** Playback and articulation update the desired model list. During admission
   * the committed list stays intact; commit uses the latest requested pose. */
  models(models: readonly LabModel[]): void {
    const desired = this.desired;
    if (this.disposed || !desired) return;
    desired.models = models;
    if (this.active === desired) this.presentation.setLabModels(models);
  }

  current(): boolean {
    return this.active === this.desired && this.desired?.completed === true;
  }

  /** Called before canonical framing. A successful swap must be followed by a
   * fresh presentation frame before rendering, enforced by the Rust runtime. */
  flush(): boolean {
    const ready = this.ready;
    if (!ready || this.disposed) return false;
    this.ready = null;
    const request = ready.request;
    // Rust consumes this handle on success, stale rejection and failure alike.
    try {
      const committed = this.presentation.commitRendererAppearances(
        this.renderer,
        ready.assets,
        request.models,
      );
      if (committed) this.active = request;
      this.finish(request, committed);
      return committed;
    } catch (error) {
      this.fail(request, error);
      return false;
    }
  }

  /** Await the latest request, including any request that supersedes it. The
   * frame loop must keep running so ready generations can commit. */
  async settled(): Promise<boolean> {
    let waited = false;
    while (!this.disposed) {
      const desired = this.desired;
      if (!desired) break;
      waited ||= !desired.completed;
      await desired.done;
      if (desired !== this.desired) continue;
      if (desired.error !== undefined) throw desired.error;
      break;
    }
    return waited;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    if (this.desired) this.finish(this.desired, false);
    this.discardReady();
  }

  private start(): void {
    const request = this.desired;
    if (this.disposed || this.building || this.ready || !request || request.completed) return;
    this.building = request;
    let pending: Promise<PreparedAppearances>;
    try {
      pending = this.presentation.prepareRendererAppearances(
        this.renderer,
        request.assets,
        this.detail,
      );
    } catch (error) {
      this.building = null;
      this.fail(request, error);
      return;
    }
    void pending.then(
      (assets) => {
        this.building = null;
        if (this.disposed || this.desired !== request) {
          assets.free();
          this.start();
          return;
        }
        this.ready = { request, assets };
        this.redraw();
      },
      (error: unknown) => {
        this.building = null;
        if (!this.disposed && this.desired === request) this.fail(request, error);
        this.start();
      },
    );
  }

  private finish(request: Request, committed: boolean): void {
    if (request.completed) return;
    request.completed = true;
    request.resolve(committed);
  }

  private fail(request: Request, error: unknown): void {
    request.error = error;
    this.finish(request, false);
    this.failed(error);
  }

  private discardReady(): void {
    this.ready?.assets.free();
    this.ready = null;
  }
}
