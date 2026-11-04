// The scene player: one capture scene played live, as the menu shows it and
// the capture tool records it. It steps the scene's battle to its warm tick,
// holds it there until `start` settles, flies the reel's camera over it on
// the battle's clock, dips the picture to black across each cut, and when
// the reel has played, behind the veil, says so (`onEnd`). The stage it
// draws is one element: the battle, any `children` laid over it, the cards
// when on, and the veil on top.
import { useEffect, useRef, useState, type ReactNode, type Ref, type RefObject } from "react";
import { ReadoutLayer } from "@web/battle/present/readouts";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { useFeed } from "@web/feed";
import { gameCamera } from "../gameCamera";
import type { graphicsProfiles } from "../graphicsSettings";
import { LabViewport } from "../LabViewport";
import type { ViewportPilot } from "../viewportRender";
import { useLabLoading } from "../LabLoading";
import { useBattleSession } from "../useBattleSession";
import type { ScriptedSim } from "../useSimSession";
import { stillReel, type CaptureScene } from "./capture";
import { reelPilot } from "./reelPilot";

const cameraAt = (pose: CameraPose): Camera3DParams => ({
  target: [pose.target[0], pose.target[1], 0],
  distance: pose.distance,
  yaw: pose.yaw,
  pitch: pose.pitch,
  ...gameCamera.lens,
});

/** The pose with its subject moved clear of `plate`: when the plate stands
 *  in the left half, the target slides so the subject sits centred in the
 *  open screen to its right. */
function clearOf(plate: HTMLElement | null | undefined, pose: CameraPose): CameraPose {
  const edge = plate?.getBoundingClientRect().right ?? 0;
  const width = window.innerWidth;
  if (edge <= 0 || edge >= width / 2) return pose;
  const aim = (edge + width) / width - 1; // the open area's centre, −1…1 across
  const halfWidth =
    pose.distance * Math.tan(gameCamera.lens.fovY / 2) * (width / window.innerHeight);
  // The camera looks along yaw + π; its right is that direction turned clockwise.
  const right = [-Math.sin(pose.yaw), Math.cos(pose.yaw)];
  const shift = aim * halfWidth;
  return {
    ...pose,
    target: [pose.target[0] - right[0] * shift, pose.target[1] - right[1] * shift],
  };
}

/** What the camera reads from the running battle, set once it runs. */
interface ReelBattle {
  /** One of the filming side's units, or null once it is gone. */
  unitAt: (id: number) => ArrayLike<number> | null;
  /** The battle's presentation clock at `now`, in simulation seconds. */
  clock: (now: number) => number | null;
}

/** The scene's camera: it veils the picture until the battle stands at its
 *  warm tick and `hold` settles, then flies the reel on the battle's clock. */
function sceneCamera(
  scene: CaptureScene,
  tickHz: number,
  veil: RefObject<HTMLDivElement | null>,
  plate: RefObject<HTMLElement | null> | undefined,
  hold: () => Promise<void>,
  onEnd: () => void,
  reducedMotion: boolean,
) {
  const reel = reducedMotion ? stillReel(scene.reel) : scene.reel;
  const fly = reelPilot(reel);
  const battle: ReelBattle = { unitAt: () => null, clock: () => null };
  let ended = false;
  let warm = false;
  // The reel runs on the battle's clock, not the wall's: a battle running
  // behind real time (a slow worker, a stalled frame) keeps each cut on the
  // moment it was cut for.
  const warmClock = Math.round(scene.warm_s * tickHz) / tickHz;
  const scripted: ScriptedSim = {
    warmTo: Math.round(scene.warm_s * tickHz),
    hold,
    onWarm: () => (warm = true),
    onTick: () => {},
  };
  const pilot: ViewportPilot = {
    pose(now) {
      const clock = warm ? battle.clock(now) : null;
      const frame =
        clock === null
          ? { pose: reel.shots[0].from, black: 1, done: false }
          : fly(Math.max(0, clock - warmClock), battle.unitAt);
      if (veil.current) veil.current.style.opacity = String(frame.black);
      if (frame.done && !ended) {
        ended = true;
        onEnd();
      }
      return clearOf(plate?.current, frame.pose);
    },
  };
  return { scripted, pilot, battle, initial: cameraAt(reel.shots[0].from) };
}

export interface ScenePlayerProps {
  scene: CaptureScene;
  /** The scene's built scenario (`useBuiltScenario`). */
  scenario: string;
  /** Draw the units' info cards over the battle. */
  cards: boolean;
  /** The battle holds at its warm tick, veiled, until this settles. */
  start: Promise<void>;
  /** The battle first stands at its warm tick. */
  onWarm?: () => void;
  /** The reel has played to the end, under the veil. */
  onEnd: () => void;
  /** The stage element's class. */
  className?: string;
  /** The stage element: the battle, its cards and the veil, nothing else. */
  stage?: Ref<HTMLDivElement>;
  /** The battle picture's class (a film look). */
  filmClassName?: string;
  /** Layers over the picture, under the cards and veil (a grade). */
  children?: ReactNode;
  /** A panel over the left of the screen the camera frames its subject clear of. */
  plate?: RefObject<HTMLElement | null>;
  /** The camera rig; the player's when omitted. */
  cameraConfig?: typeof gameCamera.config;
  quality: typeof graphicsProfiles.high;
  /** Remounts the viewport (not the battle) when it changes. */
  viewportKey?: string | number;
  /** The viewport's GPU is up. */
  onViewportReady?: () => void;
  /** A frame was drawn at `now`. */
  onFrame?: (now: number) => void;
  /** Play the battle's sound and score, heard from the reel's camera. */
  sound?: boolean;
  /** Hold the first framing for the reel's length; the system's preference
   *  when omitted. */
  reducedMotion?: boolean;
  /** The scene's battle could not start. */
  onError?: (message: string) => void;
}

export function ScenePlayer({
  scene,
  scenario,
  cards,
  start,
  onWarm,
  onEnd,
  className,
  filmClassName,
  children,
  plate,
  cameraConfig,
  quality,
  viewportKey,
  onViewportReady,
  onFrame,
  sound = false,
  reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches,
  onError,
  stage,
}: ScenePlayerProps) {
  const veil = useRef<HTMLDivElement>(null);
  // Ready to film once the battle first stands at its warm tick.
  const [warmed, setWarmed] = useState(false);
  useLabLoading("renderer", warmed);
  const [camera] = useState(() =>
    sceneCamera(
      scene,
      (JSON.parse(scenario) as { rules: { tick_hz: number } }).rules.tick_hz,
      veil,
      plate,
      () => {
        setWarmed(true);
        onWarm?.();
        return start;
      },
      onEnd,
      reducedMotion,
    ),
  );
  const session = useBattleSession({
    scenario,
    seed: scene.seed,
    scripted: camera.scripted,
    inputEnabled: false,
    xray: false,
    sound,
  });
  const { sim } = session;
  camera.battle.unitAt = (id) => sim.latest.current?.own.find((u) => u.id === id)?.position ?? null;
  camera.battle.clock = (now) => sim.client?.presentation?.clock(now) ?? null;
  useEffect(() => {
    if (!sim.error) return;
    console.error(`The scene's battle could not start: ${sim.error}`);
    onError?.(sim.error);
  }, [sim.error, onError]);
  const world = useFeed(session.meshes);
  if (!session.meshes || sim.error) return null;
  return (
    <div className={className} ref={stage} aria-hidden>
      <div className={filmClassName}>
        <LabViewport
          key={viewportKey}
          fixture="scene-player"
          inputEnabled={false}
          world={world}
          {...session.known}
          {...session.viewport}
          initialCamera={camera.initial}
          groundAt={session.surfaceZ}
          onReady={(gpu) => {
            session.viewport.onReady(gpu);
            onViewportReady?.();
          }}
          pilot={{ ...camera.pilot, frame: onFrame && (({ now }) => onFrame(now)) }}
          cameraConfig={cameraConfig}
          quality={quality}
          onFrame={
            cards || sound
              ? (project, view, pointer) => {
                  if (sound) session.hear();
                  if (cards) session.placePanels(project, view, pointer);
                }
              : undefined
          }
        />
      </div>
      {children}
      {cards && (
        <ReadoutLayer
          own={sim.observation?.own ?? []}
          rules={session.rules}
          selected={[]}
          handle={session.readouts}
        />
      )}
      <div className="menu-backdrop-veil" ref={veil} style={{ opacity: 1 }} />
    </div>
  );
}
