// The battle's world-space indicators as one side's view draws them: its
// contact glyphs (aloft signs facing the camera), supply, orders, aircraft
// marks, roads in, zone, objectives and the map's border, from the side's
// own observation and presented contacts, with strokes sized for the camera.
// The battle view and the graphics test (which films for native parity) draw
// the same overlay through this one hook.
import { useCallback, useMemo, useRef, useState } from "react";
import { metresPerPxAt, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { borderWidthM, buildMapBorder } from "@packages/battle-renderer/src/playAreaOverlay";
import { combineWorldMeshes } from "@packages/battle-renderer/src/mesh";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import type { SideName } from "@web/battle/sim/protocol";
import { useFeed } from "@web/feed";
import { battleOverlayParts, battleOverlayScenario } from "./battleOverlay";
import { useContactFacing } from "./contactFacing";
import { gameMapBorder } from "./gameFog";
import { gameStroke } from "./gameOverlay";
import type { BattleSession } from "./useBattleSession";

/** Zoom steps for the marks whose strokes are sized on screen (the border,
 *  the orders): distance = ZOOM_BASE ** step. */
const ZOOM_BASE = 1.25;
const zoomStep = (distance: number) => Math.round(Math.log(distance) / Math.log(ZOOM_BASE));

/** The overlay `session`'s view of `scenario` draws for `side` (blue when
 *  omitted) through a camera opening at `camera`; nothing when not `shown`.
 *  `overlayFeed` is what the viewport uploads, and `partsFeed` the layers it
 *  combines (`battleOverlayParts`), set together. Call `follow` with each
 *  frame's camera and render scale. */
export function useBattleOverlay(
  session: BattleSession,
  {
    scenario,
    side,
    camera,
    shown = true,
  }: { scenario: string; side?: SideName; camera: Camera3DParams; shown?: boolean },
) {
  const parsed = useMemo(
    () => ({
      size: (JSON.parse(scenario) as { map: { size: [number, number] } }).map.size,
      drawn: battleOverlayScenario(scenario, side ?? "blue"),
    }),
    [scenario, side],
  );
  // The border and the orders are rebuilt only when the zoom crosses a step
  // (×1.25), so their widths hold near their `_px` widths on screen.
  const [zoom, setZoom] = useState(() => zoomStep(camera.distance));
  const zoomRef = useRef(zoom);
  const metresPerPx = metresPerPxAt(ZOOM_BASE ** zoom, camera.fovY, window.innerHeight);
  const [renderScale, setRenderScale] = useState(1);
  const contactFacing = useContactFacing(camera, metresPerPx, renderScale);
  const { world, control, surfaceZ, contacts } = session;
  const { observation } = session.sim;
  const border = useMemo(
    () =>
      world
        ? buildMapBorder(
            parsed.size,
            gameMapBorder,
            borderWidthM(gameMapBorder, gameStroke(metresPerPx, renderScale)),
            surfaceZ,
          )
        : null,
    [world, parsed.size, metresPerPx, renderScale, surfaceZ],
  );
  const built = useMemo(() => {
    if (!shown || !world || !observation) return undefined;
    const parts = battleOverlayParts(
      session.units,
      session.rules,
      observation,
      control.selected,
      surfaceZ,
      parsed.drawn,
      {
        showOrders: control.showOrders,
        reveal: session.revealed,
        contacts,
        facing: contactFacing.facing,
        allies: session.allies,
      },
      border,
      gameStroke(metresPerPx, renderScale),
    );
    return { parts, meshes: combineWorldMeshes(parts) };
  }, [
    shown,
    session.allies,
    contactFacing.facing,
    session.units,
    session.rules,
    world,
    observation,
    contacts,
    surfaceZ,
    control.selected,
    parsed.drawn,
    control.showOrders,
    session.revealed,
    border,
    metresPerPx,
    renderScale,
  ]);
  const overlayFeed = useFeed<WorldMeshes | undefined>(built?.meshes);
  const partsFeed = useFeed(built?.parts);
  const { follow: face } = contactFacing;
  const follow = useCallback(
    (view: Camera3DParams, scale: number) => {
      setRenderScale(scale);
      face(view);
      const step = zoomStep(view.distance);
      if (step !== zoomRef.current) {
        zoomRef.current = step;
        setZoom(step);
      }
    },
    [face],
  );
  return { overlayFeed, partsFeed, follow };
}
