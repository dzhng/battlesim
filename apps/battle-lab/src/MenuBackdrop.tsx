// The main menu's backdrop: live battles on saved battlefields, one scene
// after another, each filmed by its reel's camera moves and graded to the
// menu's amber. Silent (the menu's music plays over it) and inert: input
// belongs to the menu. Each is the menu's own battle, so leaving the menu
// releases its worker and scene like any battle's; when a scene's reel ends
// the next scene's battle starts behind the veil (its saved scenario fetched
// while the last one played), and after the last the first starts again.
// Its preparation reports to the menu's loading screen, and the battle holds
// at its warm tick until the menu is shown, so the reel's first cut opens on
// the moment it was cut for.
import { useEffect, useRef, useState, useSyncExternalStore, type RefObject } from "react";
import backdrop from "@fixtures/menu-backdrop.json";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { appResources } from "./appResources";
import { useFeed } from "./feed";
import { gameCamera } from "./gameCamera";
import { LabViewport, type ViewportPilot } from "./LabViewport";
import { sampleReel, validateBackdrop, type BackdropScene, type MenuReel } from "./menuReel";
import { savedBattle } from "./savedMaps";
import { buildFailed, useBuiltScenario } from "./useBuiltScenario";
import { useBattleSession } from "./useBattleSession";
import { useLabLoading } from "./LabLoading";
import type { ScriptedSim } from "./useSimSession";
import game from "@fixtures/game.json";

const SCENES = validateBackdrop(backdrop).scenes;

/** Each scene's saved scenario, fetched once: the next scene's is asked for
 *  while the current one plays, so the cut to it waits on starting and
 *  warming its battle, not on the download. */
const scenarios = new Map<BackdropScene, Promise<string>>();
function sceneScenario(scene: BackdropScene): Promise<string> {
  let s = scenarios.get(scene);
  if (!s) {
    s = savedBattle(scene.map, scene.encounter).then((b) => b.scenario);
    // A failed preparation is asked again next time, not remembered.
    s.catch(() => scenarios.delete(scene));
    scenarios.set(scene, s);
  }
  return s;
}

/** Under reduced motion the camera holds the first framing for the reel's length. */
function reelFor(reel: MenuReel, reduced: boolean): MenuReel {
  if (!reduced) return reel;
  const seconds = reel.shots.reduce((sum, shot) => sum + shot.seconds, 0);
  const still = reel.shots[0].from;
  return { fade_s: reel.fade_s, shots: [{ seconds, from: still, to: still }] };
}

const cameraAt = (pose: CameraPose): Camera3DParams => ({
  target: [pose.target[0], pose.target[1], 0],
  distance: pose.distance,
  yaw: pose.yaw,
  pitch: pose.pitch,
  ...gameCamera.lens,
});

export function MenuBackdrop({
  plate,
  shown,
}: {
  plate: RefObject<HTMLElement | null>;
  /** Settles when the menu is shown. */
  shown: Promise<void>;
}) {
  // A refused page GPU leaves the menu its plain background, with nothing to wait for.
  const refused = useSyncExternalStore(appResources.subscribe, appResources.error);
  useLabLoading("renderer", refused ? true : null);
  // Scenes played so far: the one playing is `played` modulo their count.
  const [played, setPlayed] = useState(0);
  const scene = SCENES[played % SCENES.length];
  const next = SCENES[(played + 1) % SCENES.length];
  const battle = useBuiltScenario(scene, (_, s) => sceneScenario(s));
  const failed = buildFailed(battle) ? battle.error : null;
  useEffect(() => {
    if (failed) console.error(`The menu backdrop could not be prepared: ${failed}`);
  }, [failed]);
  if (refused || !battle || buildFailed(battle)) return null;
  return (
    <BackdropBattle
      key={played}
      scene={scene}
      scenario={battle}
      plate={plate}
      shown={shown}
      onWarm={() => void sceneScenario(next).catch(() => {})}
      onEnd={() => setPlayed((n) => n + 1)}
    />
  );
}

/** How long a tracking shot's camera takes to catch up with its unit, seconds:
 *  it glides with the unit instead of stepping with each simulation tick. */
const TRACK_LAG_S = 0.5;

/** What the pilot reads from the running battle. */
interface ReelBattle {
  /** An own unit's position, or null once it is gone. */
  unitAt: (id: number) => ArrayLike<number> | null;
  /** The battle's presentation clock at `now`, in simulation seconds. */
  clock: (now: number) => number | null;
}

/** The reel's pilot: it veils the picture until the battle stands at its warm
 *  tick and `hold` settles, plays the shots, and at the end, behind the veil,
 *  hands over to the next scene (`onEnd`). */
function createReelPilot(
  scene: BackdropScene,
  veil: RefObject<HTMLDivElement | null>,
  plate: RefObject<HTMLElement | null>,
  hold: () => Promise<void>,
  onEnd: () => void,
) {
  const reel = reelFor(scene.reel, matchMedia("(prefers-reduced-motion: reduce)").matches);
  const battle: ReelBattle = { unitAt: () => null, clock: () => null };
  let ended = false;
  let warm = false;
  // The reel runs on the battle's clock, not the wall's: a battle running
  // behind real time (a slow worker, a stalled frame) keeps each cut on the
  // moment it was cut for.
  const warmClock = Math.round(scene.warm_s * game.tick_hz) / game.tick_hz;
  // The tracked unit's smoothed position; it holds where a fallen unit was.
  let tracked: { follow: number; at: [number, number]; now: number } | null = null;
  // The last framed subject's world position.
  let subject: readonly [number, number] = [0, 0];
  const track = (follow: number, now: number): [number, number] | null => {
    const live = battle.unitAt(follow);
    if (tracked?.follow !== follow) tracked = live && { follow, at: [live[0], live[1]], now };
    if (!tracked) return null;
    if (live) {
      const k = 1 - Math.exp(-(now - tracked.now) / 1000 / TRACK_LAG_S);
      tracked.at = [
        tracked.at[0] + (live[0] - tracked.at[0]) * k,
        tracked.at[1] + (live[1] - tracked.at[1]) * k,
      ];
    }
    tracked.now = now;
    return tracked.at;
  };
  const scripted: ScriptedSim = {
    warmTo: Math.round(scene.warm_s * game.tick_hz),
    hold,
    onWarm: () => (warm = true),
    onTick: () => {},
  };
  const pilot: ViewportPilot = {
    pose(now) {
      const clock = warm ? battle.clock(now) : null;
      const sample =
        clock === null
          ? { pose: reel.shots[0].from, follow: null, black: 1, done: false }
          : sampleReel(reel, Math.max(0, clock - warmClock));
      if (veil.current) veil.current.style.opacity = String(sample.black);
      if (sample.done && !ended) {
        ended = true;
        onEnd();
      }
      const [x, y] = sample.pose.target;
      if (sample.follow === null) subject = [x, y];
      else {
        // A unit gone before its shot opens: frame where the last subject was.
        const unit = track(sample.follow, now) ?? subject;
        subject = unit;
        sample.pose = { ...sample.pose, target: [unit[0] + x, unit[1] + y] };
      }
      return composed(sample.pose);
    },
  };
  /** The pose with its subject moved clear of the plate: when the plate stands
   *  in the left half, the target slides so the subject sits centred in the
   *  open screen to its right. */
  const composed = (pose: CameraPose): CameraPose => {
    const edge = plate.current?.getBoundingClientRect().right ?? 0;
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
  };
  return { scripted, pilot, battle, initial: cameraAt(reel.shots[0].from) };
}

function BackdropBattle({
  scene,
  scenario,
  plate,
  shown,
  onWarm,
  onEnd,
}: {
  scene: BackdropScene;
  scenario: string;
  plate: RefObject<HTMLElement | null>;
  shown: Promise<void>;
  /** The battle first stands at its warm tick. */
  onWarm: () => void;
  /** Its reel has played to the end, under the veil. */
  onEnd: () => void;
}) {
  const veil = useRef<HTMLDivElement>(null);
  // Ready to film once the battle first stands at its warm tick.
  const [warmed, setWarmed] = useState(false);
  useLabLoading("renderer", warmed);
  const [reel] = useState(() =>
    createReelPilot(
      scene,
      veil,
      plate,
      () => {
        setWarmed(true);
        onWarm();
        return shown;
      },
      onEnd,
    ),
  );
  const session = useBattleSession({
    scenario,
    seed: scene.seed,
    scripted: reel.scripted,
    destroyable: "apart",
    inputEnabled: false,
    xray: false,
  });
  const { sim } = session;
  reel.battle.unitAt = (id) => sim.latest.current?.own.find((u) => u.id === id)?.position ?? null;
  reel.battle.clock = (now) => sim.interpolator.current?.time(now) ?? null;
  useEffect(() => {
    if (sim.error) console.error(`The menu backdrop battle could not start: ${sim.error}`);
  }, [sim.error]);
  const world = useFeed(session.meshes);
  if (!session.meshes || session.sim.error) return null;
  return (
    <div className="menu-backdrop" aria-hidden>
      <div className="menu-backdrop-film">
        <LabViewport
          fixture="menu-backdrop"
          inputEnabled={false}
          world={world}
          structures={session.structures}
          buildings={session.buildingsFeed}
          obstacles={session.cameraObstaclesFeed}
          frame={session.frame}
          appearances={session.appearances}
          initialCamera={reel.initial}
          groundAt={session.surfaceZ}
          onReady={session.onReady}
          pilot={reel.pilot}
        />
      </div>
      <div className="menu-backdrop-grade" />
      <div className="menu-backdrop-veil" ref={veil} style={{ opacity: 1 }} />
    </div>
  );
}
