import type { BrowserScene } from "@web/battle/render/scene";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import type { GraphicsQualityProfile } from "./graphicsSettings";

/** Complete inputs captured for one viewport scene candidate. */
export interface SceneAdmissionIdentity {
  observer: ScenePresentation;
  world: WorldLayers;
  appearances: InstalledAppearances | null;
  quality: GraphicsQualityProfile | undefined;
  qualityKey: string;
}

export interface SceneAdmissionOptions {
  /** Supplies the latest complete observer/world/art/quality identity. */
  capture: () => Promise<SceneAdmissionIdentity>;
  /** Checks the captured identity against the latest viewport inputs. */
  current: (identity: SceneAdmissionIdentity) => boolean;
  /** Builds one candidate. `null` means its captured inputs became stale. */
  build: (identity: SceneAdmissionIdentity, current: () => boolean) => Promise<BrowserScene | null>;
  /** The frame loop owns redraw scheduling; admission only asks for one. */
  redraw: () => void;
  /** The installed scene is exposed to the viewport's frame consumers. */
  installed: (scene: BrowserScene) => void;
}

/**
 * Viewport-local owner for complete GPU scene admission and replacement.
 *
 * It serializes candidates, captures their complete source identity, disposes
 * stale successes, and lets a newer request supersede a stale rejection. It
 * never owns the page GPU, frame scheduler, or asset-only admission.
 */
export class SceneAdmission {
  private scene: BrowserScene | null = null;
  private building: Promise<void> | null = null;
  private disposed = false;

  constructor(private readonly options: SceneAdmissionOptions) {}

  /** Request the latest complete scene, sharing one bounded candidate. */
  request(): Promise<void> {
    if (this.disposed) return Promise.resolve();
    if (this.building) return this.building;
    const run = this.replace();
    const settled = run.finally(() => {
      if (this.building === settled) this.building = null;
    });
    this.building = settled;
    return settled;
  }

  /** The currently installed scene, or null while the first candidate loads. */
  current(): BrowserScene | null {
    return this.scene;
  }

  /** The frame loop intentionally skips the old scene while replacing it. */
  replacing(): boolean {
    return this.building !== null;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.scene?.dispose();
    this.scene = null;
  }

  private async replace(): Promise<void> {
    while (!this.disposed) {
      let identity: SceneAdmissionIdentity;
      try {
        identity = await this.options.capture();
      } catch (error) {
        if (this.disposed) return;
        throw error;
      }
      if (this.disposed) return;
      const isCurrent = () => !this.disposed && this.options.current(identity);
      let next: BrowserScene | null;
      try {
        next = await this.options.build(identity, isCurrent);
      } catch (error) {
        if (this.disposed) return;
        if (!isCurrent()) continue;
        throw error;
      }
      if (!next) continue;
      if (this.disposed || !isCurrent()) {
        next.dispose();
        continue;
      }
      const previous = this.scene;
      if (previous) next.inheritControls(previous);
      this.scene = next;
      this.options.installed(next);
      previous?.dispose();
      this.options.redraw();
      return;
    }
  }
}
