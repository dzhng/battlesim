// A card over each of the viewer's side's roads in, with its lane's number.
// It answers "where do my side's reinforcements come in?" from far out,
// where the road's painted number has shrunk past reading; closer in that
// number says it, so the card stands aside.
import { useImperativeHandle, type Ref } from "react";
import {
  ENTRY_DIGIT_HEIGHT_M,
  entryMarkerSpan,
  type EntryMarker,
} from "@packages/battle-renderer/src/deploymentMarker";
import { hudIcon } from "@packages/scene-assets/src/icons";
import type { Project } from "./readouts";
import { Icon } from "./icons";
import { useGroundLabels } from "./groundLabels";

export interface EntryCardsHandle {
  place(project: Project, elevation: (x: number, y: number) => number): void;
}

/** The painted number's height on screen (px) from which it reads by itself. */
const READABLE_PX = 30;

export function EntryCards({
  entries,
  handle,
}: {
  entries: readonly EntryMarker[];
  handle: Ref<EntryCardsHandle>;
}) {
  const labels = useGroundLabels();
  useImperativeHandle(
    handle,
    () => ({
      place(project, elevation) {
        entries.forEach((entry, i) => {
          const { ends, middle } = entryMarkerSpan(entry);
          const shown = (p: readonly [number, number]) =>
            project(p[0], p[1], elevation(p[0], p[1]));
          const at = shown(middle);
          // The number's height measured across the road, which the view's
          // tilt doesn't shorten.
          const side = shown([
            middle[0] + Math.sin(entry.facing) * ENTRY_DIGIT_HEIGHT_M,
            middle[1] - Math.cos(entry.facing) * ENTRY_DIGIT_HEIGHT_M,
          ]);
          const readable =
            !!at && !!side && Math.hypot(side[0] - at[0], side[1] - at[1]) >= READABLE_PX;
          // The card stands over whichever end of the marker is highest on
          // screen, so it never covers the painted beacons.
          const top = [at, ...ends.map(shown)].reduce((a, b) => (b && a && b[1] < a[1] ? b : a));
          labels.place(String(i), readable ? null : top);
        });
      },
    }),
    [entries, labels],
  );
  return (
    <div className="ground-label-layer" aria-label="Roads in">
      {entries.map((entry, i) => (
        <span
          key={i}
          ref={labels.pin(String(i))}
          className="ground-label entry-card"
          style={{ display: "none" }}
          aria-label={`Road in ${entry.lane}`}
        >
          <Icon path={hudIcon("entry")} />
          <span>{entry.lane}</span>
        </span>
      ))}
    </div>
  );
}
