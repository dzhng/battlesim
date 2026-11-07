import { proxyPickBox, type PickBox } from "@packages/battle-renderer/src/picking";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import type { ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import type { BrowserScene } from "@web/battle/render/scene";

/**
 * Holds the last successfully submitted scene, camera and picks. Rays always
 * use its camera, including across replacement, because that is what the
 * canvas still shows. Picks exist only while their scene is installed; frame
 * picks carry over only between submissions of that same scene. ViewportFrames
 * remains the waiter owner.
 */
export interface SubmittedViewSnapshot {
  readonly scene: BrowserScene;
  readonly camera: ViewportCamera;
  readonly picks: readonly PickBox[];
  readonly framePicks: boolean;
}

/** The scene owner remains responsible for admission and replacement; this
 * owner only installs a snapshot after its scene has rendered successfully.
 * The frame's submitted callback publishes the session observation in the
 * same commit; this owner does not retain presentation-module views.
 */
export class ViewportSubmittedView {
  private snapshot: SubmittedViewSnapshot | null = null;

  currentFor(scene: BrowserScene | null | undefined): SubmittedViewSnapshot | null {
    const current = this.snapshot;
    return current && current.scene === scene ? current : null;
  }

  camera(fallback: ViewportCamera): ViewportCamera {
    return this.snapshot?.camera ?? fallback;
  }

  submit(
    input: {
      scene: BrowserScene;
      camera: ViewportCamera;
      framePicks: readonly PickBox[] | undefined;
      routePicks: readonly PickBox[] | undefined;
      instances: readonly SceneInstance[];
    },
    onSubmitted: (() => void) | undefined,
  ): void {
    const previous = this.snapshot;
    const keepFramePicks =
      !input.framePicks && previous?.scene === input.scene && previous.framePicks;
    const routePicks = input.routePicks;
    const picks =
      input.framePicks ??
      (keepFramePicks ? previous.picks : undefined) ??
      routePicks ??
      input.instances.map(proxyPickBox);
    this.snapshot = {
      scene: input.scene,
      camera: input.camera,
      picks,
      framePicks: Boolean(input.framePicks || keepFramePicks),
    };
    onSubmitted?.();
  }
}
