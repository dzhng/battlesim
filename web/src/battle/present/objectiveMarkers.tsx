import { useImperativeHandle, type Ref } from "react";
import type { Project } from "./readouts";
import type { ObjectiveView, SkirmishView } from "../sim/observationView";
import type { SideName } from "../sim/protocol";
import { Icon } from "./icons";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { useGroundLabels } from "./groundLabels";

export interface ObjectiveMarkersHandle {
  place(project: Project, elevation: (x: number, y: number) => number): void;
}
/** Whose a capture area is to `viewer`'s side: its own, the enemy's, or
 *  nobody's. Every mark of an objective (its ring, its flags) takes its colour
 *  from this, so a player reads their own holdings alike on either side. A
 *  watching client (no side) reads the match as Blue. */
export type ObjectiveStanding = "own" | "enemy" | "neutral";
export function objectiveStanding(
  owner: ObjectiveView["owner"],
  viewer: SideName | null,
): ObjectiveStanding {
  if (owner === null) return "neutral";
  return owner === (viewer ?? "blue") ? "own" : "enemy";
}

/** What follows an objective's number on its flags: its capture share while
 *  a side takes it uncontested, else nothing (a contested flag flashes). */
export function captureShare(objective: ObjectiveView): string {
  return !objective.contested && objective.capturing
    ? ` · ${Math.floor(objective.captureProgress * 100)}%`
    : "";
}

/** Public numbered flags connect the score strip to its actual capture areas. */
export function ObjectiveMarkers({
  objectives,
  viewer,
  handle,
}: {
  objectives: SkirmishView["objectives"];
  viewer: SideName | null;
  handle: Ref<ObjectiveMarkersHandle>;
}) {
  const labels = useGroundLabels();
  useImperativeHandle(
    handle,
    () => ({
      place(project, elevation) {
        for (const {
          id,
          center: [x, y],
        } of objectives)
          labels.place(id, project(x, y, elevation(x, y)));
      },
    }),
    [objectives, labels],
  );
  // Contested flags alternate their owner colour and warning yellow in CSS;
  // the data attribute remains the state contract for styling and tests.
  return (
    <div className="hud-objective-flags ground-label-layer" aria-label="Capture areas">
      {objectives.map((objective, index) => (
        <span
          key={objective.id}
          ref={labels.pin(objective.id)}
          className="ground-label"
          data-owner={objectiveStanding(objective.owner, viewer)}
          data-contested={objective.contested}
          style={{ display: "none" }}
          aria-label={`Capture area ${index + 1}`}
        >
          <Icon path={hudIcon("objective")} />
          <span>
            {index + 1}
            {captureShare(objective)}
          </span>
        </span>
      ))}
    </div>
  );
}
