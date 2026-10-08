import { useImperativeHandle, useRef, type Ref } from "react";
import type { Project } from "./readouts";
import type { SkirmishView } from "../sim/observation";
import { Icon } from "./icons";
import { hudIcon } from "@packages/scene-assets/src/icons";

export interface ObjectiveMarkersHandle {
  place(project: Project, elevation: (x: number, y: number) => number): void;
}
/** Public numbered flags connect the score strip to its actual capture areas. */
export function ObjectiveMarkers({
  objectives,
  handle,
}: {
  objectives: SkirmishView["objectives"];
  handle: Ref<ObjectiveMarkersHandle>;
}) {
  const nodes = useRef(new Map<string, HTMLSpanElement>());
  useImperativeHandle(
    handle,
    () => ({
      place(project, elevation) {
        for (const objective of objectives) {
          const node = nodes.current.get(objective.id);
          if (!node) continue;
          const [x, y] = objective.center;
          const point = project(x, y, elevation(x, y));
          node.style.display = point ? "" : "none";
          if (point)
            node.style.transform = `translate(${point[0]}px, ${point[1]}px) translate(-50%, -100%)`;
        }
      },
    }),
    [objectives],
  );
  // Contested flags alternate their owner colour and warning yellow in CSS;
  // the data attribute remains the state contract for styling and tests.
  return (
    <div className="hud-objective-flags skirmish-objective-layer" aria-label="Capture areas">
      {objectives.map((objective, index) => (
        <span
          key={objective.id}
          ref={(node) => {
            if (node) nodes.current.set(objective.id, node);
            else nodes.current.delete(objective.id);
          }}
          className="skirmish-objective-mark"
          data-owner={objective.owner ?? "neutral"}
          data-contested={objective.contested}
          style={{ display: "none" }}
          aria-label={`Capture area ${index + 1}`}
        >
          <Icon path={hudIcon("objective")} />
          {index + 1}
          {objective.contested
            ? " · CONTESTED"
            : objective.capturing
              ? ` · ${Math.floor(objective.captureProgress * 100)}%`
              : ""}
        </span>
      ))}
    </div>
  );
}
