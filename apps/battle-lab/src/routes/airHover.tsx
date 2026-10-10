import { useCallback, useMemo, useState } from "react";
import { metresPerPxAt, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounters, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";
import { buildBattleOverlay } from "../battleOverlay";
import type { ViewportPointer } from "../LabViewport";
import type { Project } from "@web/battle/present/readouts";

// Test helicopters hovering at cruise height, drawn as the battle draws
// them: each with its ground marker and drop line (D18), its rotors turning
// and its shadow on the ground. Variants are saved encounters of the air map:
// `hover`, one over open ground beside the road with the test jeep on the
// road below it for scale, and nothing else moving; `roof`, one over the
// house's roof and an identified enemy one over the field across the road;
// `fog`, an enemy one over the ground the house hides from blue's jeep, seen
// over the fog (V02).
const VARIANTS = {
  hover: { label: "Over open ground", target: [142, 90, 8] },
  roof: { label: "Over a roof, enemy beyond", target: [256, 97, 10] },
  fog: { label: "Enemy over fog", target: [270, 60, 8] },
} as const;
type Variant = keyof typeof VARIANTS;
const VARIANT_NAMES = Object.keys(VARIANTS) as Variant[];
const SEED = 6;

// From the south-east at about the game's tactical pitch: the airframe over
// the ground, its shadow beside it.
const camera = (variant: Variant): Camera3DParams => ({
  target: [...VARIANTS[variant].target],
  distance: 58,
  pitch: 0.72,
  yaw: -1.25,
  ...gameCamera.lens,
});

/** The overlay's strokes are sized for the camera's zoom, as the battle
 *  view's are; the zoom is kept in eighths of a doubling so a camera move
 *  rebuilds them only when they would visibly change. */
const zoomStep = (metresPerPx: number) => 2 ** (Math.round(Math.log2(metresPerPx) * 8) / 8);

export default function AirHover() {
  return (
    <SavedEncounters fixture="air-hover" encounters={VARIANT_NAMES}>
      {(battles) => <AirHoverLab battles={battles} />}
    </SavedEncounters>
  );
}

function AirHoverLab({ battles }: { battles: Record<Variant, SavedBattle> }) {
  const [variant, setVariant] = useState<Variant>(() => {
    const requested = new URLSearchParams(window.location.search).get("variant");
    return VARIANT_NAMES.find((name) => name === requested) ?? "hover";
  });
  const session = useBattleSession({ ...battles[variant], seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;
  // The camera opens on the variant the address asks for, and stays put
  // when another is chosen.
  const [initial] = useState(() => camera(variant));
  const [metresPerPx, setMetresPerPx] = useState(() =>
    zoomStep(metresPerPxAt(initial.distance, initial.fovY, window.innerHeight)),
  );
  const { placePanels } = session;
  const onFrame = useCallback(
    (project: Project, view: Camera3DParams, pointer: ViewportPointer) => {
      placePanels(project, view, pointer);
      setMetresPerPx(zoomStep(metresPerPxAt(view.distance, view.fovY, window.innerHeight)));
    },
    [placePanels],
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
            { showOrders: control.showOrders, reveal: session.revealed, contacts: [] },
            null,
            metresPerPx,
          )
        : undefined,
    [
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
  const overlayFeed = useFeed(overlay);
  const chooseVariant = (next: Variant) => {
    if (next === variant) return;
    const url = new URL(window.location.href);
    url.searchParams.set("variant", next);
    window.history.replaceState(window.history.state, "", url);
    setVariant(next);
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="air-hover"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={initial}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onReady={session.onReady}
        onFrame={onFrame}
        diagnostics={{ ...session.probes, variant: (v: Variant) => chooseVariant(v) }}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="air-hover-panel">
        <strong>Air hover</strong>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-row">
          {VARIANT_NAMES.map((name) => (
            <button
              key={name}
              type="button"
              aria-pressed={name === variant}
              onClick={() => chooseVariant(name)}
            >
              {VARIANTS[name].label}
            </button>
          ))}
        </div>
        <div className="lab-row">
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
        </div>
      </aside>
    </>
  );
}
