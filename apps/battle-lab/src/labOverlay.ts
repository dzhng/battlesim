import { useCallback, useMemo, useState } from "react";
import { metresPerPxAt, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { ViewportPointer } from "./LabViewport";
import type { useBattleSession } from "./useBattleSession";
import { useFeed } from "./feed";
import { buildBattleOverlay } from "./battleOverlay";
import { useContactFacing } from "./contactFacing";
import type { Project } from "@web/battle/present/readouts";

/** The overlay's strokes are sized for the camera's zoom; the zoom is kept in
 *  eighths of a doubling so a camera move rebuilds them only when they would
 *  visibly change. */
const zoomStep = (metresPerPx: number) => 2 ** (Math.round(Math.log2(metresPerPx) * 8) / 8);

/** A battle lab's overlay, drawn as the battle draws it (markers, drop lines,
 *  contact glyphs facing the camera), and the per-frame hook that keeps its
 *  panels placed and its strokes sized for the camera that opened at
 *  `initial`. */
export function useLabOverlay(
  session: ReturnType<typeof useBattleSession>,
  initial: Camera3DParams,
) {
  const { world, control, surfaceZ, placePanels } = session;
  const { observation } = session.sim;
  const [metresPerPx, setMetresPerPx] = useState(() =>
    zoomStep(metresPerPxAt(initial.distance, initial.fovY, window.innerHeight)),
  );
  const contactFacing = useContactFacing(initial, metresPerPx);
  const { follow } = contactFacing;
  const onFrame = useCallback(
    (project: Project, view: Camera3DParams, pointer: ViewportPointer) => {
      placePanels(project, view, pointer);
      follow(view);
      setMetresPerPx(zoomStep(metresPerPxAt(view.distance, view.fovY, window.innerHeight)));
    },
    [placePanels, follow],
  );
  const overlay = useMemo(
    () =>
      world && observation
        ? buildBattleOverlay(
            session.units,
            observation,
            control.selected,
            surfaceZ,
            { supplyRadius: 0, zone: null, deployment: null },
            {
              showOrders: control.showOrders,
              reveal: session.revealed,
              contacts: session.contacts,
              facing: contactFacing.facing,
            },
            null,
            metresPerPx,
          )
        : undefined,
    [
      session.contacts,
      contactFacing.facing,
      world,
      observation,
      session.units,
      control.selected,
      control.showOrders,
      session.revealed,
      surfaceZ,
      metresPerPx,
    ],
  );
  return { overlayFeed: useFeed(overlay), onFrame };
}
