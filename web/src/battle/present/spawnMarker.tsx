import { useImperativeHandle, useRef, type Ref } from "react";
import type { Project } from "./readouts";
import { Icon } from "./icons";
import { hudIcon } from "@packages/scene-assets/src/icons";

export interface SpawnMarkerHandle {
  place(project: Project, elevation: (x: number, y: number) => number): void;
}

/** A single world-anchored deployment cue, visible while the player is deploying. */
export function SpawnMarker({
  at,
  handle,
}: {
  at: [number, number] | null;
  handle: Ref<SpawnMarkerHandle>;
}) {
  const node = useRef<HTMLSpanElement>(null);
  useImperativeHandle(
    handle,
    () => ({
      place(project, elevation) {
        const el = node.current;
        if (!el || !at) return;
        const point = project(at[0], at[1], elevation(at[0], at[1]));
        el.style.display = point ? "" : "none";
        if (point)
          el.style.transform = `translate(${point[0]}px, ${point[1]}px) translate(-50%, -50%)`;
      },
    }),
    [at],
  );
  return (
    <div className="hud-spawn-layer" aria-label="Your deployment point">
      <span ref={node} className="hud-spawn-mark" style={{ display: "none" }}>
        <Icon path={hudIcon("objective")} />
        <span>DEPLOY</span>
      </span>
    </div>
  );
}
