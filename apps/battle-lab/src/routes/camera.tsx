// /lab/camera: camera clearance. Scripted camera trajectories round a wall,
// a tower, a corner of two tall slabs, a courtyard block and a concave
// compound, each flown through the rig and its clearance. Watching, the free
// camera sees the eye path asked for and the eye path drawn; riding, the
// viewport's own camera flies the trajectory, placed and cleared as the
// benchmark's tour is.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Vec3 } from "math";
import { loadMap } from "@web/maps/browser";
import type { MapDefinition } from "@web/maps/resolve";
import {
  buildingObstacles,
  buildingPartProps,
} from "@packages/battle-renderer/src/buildingObstacles";
import {
  concatMeshes,
  EMPTY_MESH,
  MeshBuilder,
  type Mesh,
  type Rgba,
} from "@packages/battle-renderer/src/mesh";
import { knownStanding, mapProps } from "@packages/battle-renderer/src/models/propAppearance";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import {
  fallenBuildings,
  indexBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { apartKinds, buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import { nearEnvelope } from "@packages/renderer-core/src/cameraClearance";
import { CameraController, type CameraPose } from "@packages/renderer-core/src/cameraController";
import {
  COLLAPSING_OWNER,
  flyTrajectory,
  OBSERVER,
  PATH_WIDTH,
  TRAJECTORIES,
  trajectoryPose,
  watchingPose,
  type FlightFrame,
} from "../cameraLab";
import { gameGuttedShells, seenDestroyed } from "../destroyedBuildings";
import { useFeed } from "../feed";
import { LabViewport, type ViewportPilot } from "../LabViewport";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { useStaticWorld } from "../useStaticWorld";
import { useMapAppearances } from "../gameAppearances";
import { gameBiome } from "../gameBiome";
import { gameCamera } from "../gameCamera";

/** The eye path asked for: amber, red where it runs inside the clearance. */
const ASKED: Rgba = [1, 0.66, 0.2, 1];
const ASKED_BLOCKED: Rgba = [1, 0.25, 0.2, 1];
/** The eye path drawn. */
const DRAWN: Rgba = [0.3, 0.95, 1, 1];
/** Paths are drawn through every third frame of a flight. */
const PATH_EVERY = 3;
/** The pause between a played trajectory's end and its restart, seconds. */
const REPLAY_PAUSE_S = 1.5;
/** How often the watched eyes' markers move, a second. */
const MARKER_HZ = 20;

/** The two eye paths of `flight` as tubes `half` metres thick: the whole
 *  path asked for, and the path drawn wherever it leaves that one. */
function pathMesh(
  flight: readonly FlightFrame[],
  half: number,
  inClearance: (eye: Vec3) => boolean,
): Mesh {
  const mesh = new MeshBuilder();
  for (let k = PATH_EVERY; k < flight.length; k += PATH_EVERY) {
    const [a, b] = [flight[k - PATH_EVERY], flight[k]];
    const between = flight.slice(k - PATH_EVERY + 1, k + 1);
    mesh.segment(a.askedEye, b.askedEye, half, inClearance(b.askedEye) ? ASKED_BLOCKED : ASKED);
    // A cut is a jump, not a path.
    if (a.drawn !== a.asked || b.drawn !== b.asked)
      if (!between.some((f) => f.cut)) mesh.segment(a.eye, b.eye, half, DRAWN);
  }
  return mesh.build();
}

/** The two eyes at one frame of a flight: a box each, `half` metres across,
 *  and its line of sight. */
function eyeMesh(frame: FlightFrame, half: number): Mesh {
  const mesh = new MeshBuilder();
  for (const [eye, color] of [
    [frame.askedEye, ASKED],
    [frame.eye, DRAWN],
  ] as const) {
    mesh.box(eye[0], eye[1], eye[2], half, half, half, color);
    mesh.segment(eye, frame.asked.target, half / 6, color);
  }
  return mesh.build();
}

export default function CameraLab() {
  const map = useBuiltScenario("camera-lab", async () => (await loadMap("camera-lab")).definition);
  if (!map) return null;
  if (buildFailed(map))
    return (
      <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
        the camera lab's map could not be loaded: {map.error}
      </main>
    );
  return <Arena map={map} />;
}

function Arena({ map }: { map: MapDefinition }) {
  const world = useStaticWorld(map);
  // The lab's buildings are the generator's catalogue's, drawn as a
  // generated town's are.
  const drawn = useMemo(() => {
    if (!world) return null;
    const props = mapProps(world.exports, world.layout);
    return { props, index: indexBuildings(world.exports.buildings, props) };
  }, [world]);
  const appearances = useMapAppearances(drawn?.index.placed ?? null);
  const [trajectory, setTrajectory] = useState(TRAJECTORIES[0]);
  const [riding, setRiding] = useState(false);
  const [fallen, setFallen] = useState(false);
  // The trajectory's clock: playing from `at` seconds since `since`, or held at `at`.
  const [clock, setClock] = useState({ playing: true, at: 0, since: performance.now() });
  // What the pilot reads each frame, ahead of React's next render.
  const live = useRef({ trajectory, riding, clock });
  live.current = { trajectory, riding, clock };
  const secondsAt = useCallback((now: number) => {
    const { clock, trajectory } = live.current;
    if (!clock.playing) return clock.at;
    const lap = trajectory.seconds + REPLAY_PAUSE_S;
    return Math.min(trajectory.seconds, (clock.at + (now - clock.since) / 1000) % lap);
  }, []);

  const meshes = useMemo(
    () =>
      world &&
      appearances &&
      buildWorldLayers(
        world.exports,
        world.layout,
        gameBiome,
        "surface",
        apartKinds(world.layout, true),
        appearances,
      ),
    [world, appearances],
  );
  const worldFeed = useFeed(meshes);
  const surfaceZ = useCallback(
    (x: number, y: number) => world?.view.surface_at(x, y)[0] ?? 0,
    [world],
  );
  // What blue knows: nothing but the map, or that it has seen the courtyard
  // block collapse (it is low enough to; a tower would stand, gutted).
  const standing = useMemo(() => {
    const library = appearances?.templates?.library;
    if (!world || !drawn || !library) return null;
    const { props, index } = drawn;
    const known = fallen
      ? seenDestroyed(index, index.placed.owners.indexOf(COLLAPSING_OWNER), library)
      : [];
    const parts = buildingPartProps(world.exports.buildings);
    return {
      boxes: knownStanding(props, known, parts).map((s) => s.box),
      buildings: {
        placed: index.placed,
        fallen: fallenBuildings(index, known, gameGuttedShells),
      },
      obstacles: buildingObstacles(props, known, parts, surfaceZ),
    };
  }, [world, drawn, fallen, surfaceZ, appearances]);
  const buildingsFeed = useFeed(standing?.buildings ?? null);
  const obstaclesFeed = useFeed(standing?.obstacles ?? null);

  // The flight: the trajectory through the same rig numbers, obstacles and
  // lens the viewport uses.
  const rig = useMemo(() => new CameraController(gameCamera.config, surfaceZ), [surfaceZ]);
  const lens = useMemo(
    () => ({ ...gameCamera.opening(), aspect: window.innerWidth / window.innerHeight }),
    [],
  );
  const fly = useCallback(
    (id: string) => {
      const t = TRAJECTORIES.find((t) => t.id === id);
      if (!t || !standing) throw new Error(`no trajectory ${id}`);
      return flyTrajectory(t, rig, standing.obstacles, lens);
    },
    [standing, rig, lens],
  );
  const flight = useMemo(() => (standing ? fly(trajectory.id) : null), [standing, fly, trajectory]);
  // Sized for where the trajectory is watched from.
  const pathHalf = (watchingPose(trajectory).distance * PATH_WIDTH) / 2;
  const paths = useMemo(() => {
    if (!flight || !standing) return EMPTY_MESH;
    const clearance = nearEnvelope(lens) + gameCamera.config.clearance.margin_m;
    return pathMesh(flight, pathHalf, (eye) => !standing.obstacles.clear(eye, clearance));
  }, [flight, standing, lens, pathHalf]);
  // Watching: the two eyes move along their paths.
  const [marked, setMarked] = useState(0);
  useEffect(() => {
    if (riding || !clock.playing) return;
    const timer = window.setInterval(
      () => setMarked(secondsAt(performance.now())),
      1000 / MARKER_HZ,
    );
    return () => window.clearInterval(timer);
  }, [riding, clock, secondsAt]);
  const seconds = clock.playing ? marked : clock.at;
  const now = flight?.[Math.min(flight.length - 1, Math.round(seconds * 60))];
  const overlay = useMemo<WorldMeshes | undefined>(
    () =>
      // Riding, the paths pass through the camera itself: nothing is drawn.
      riding || !now
        ? { opaque: EMPTY_MESH, translucent: EMPTY_MESH }
        : { opaque: concatMeshes([paths, eyeMesh(now, 3 * pathHalf)]), translucent: EMPTY_MESH },
    [riding, paths, now, pathHalf],
  );
  const overlayFeed = useFeed(overlay);

  // Where the free camera is cut to, once, to watch a trajectory just chosen.
  const stand = useRef<CameraPose | null>(watchingPose(trajectory));
  const [pilot] = useState<ViewportPilot>(() => ({
    pose(at) {
      if (live.current.riding) return trajectoryPose(live.current.trajectory, secondsAt(at));
      const once = stand.current;
      stand.current = null;
      return once;
    },
  }));
  /** Play trajectory `id` from its start, or hold it `at` seconds in;
   *  ridden or watched (as now, unless told). */
  const choose = useCallback((id: string, ride = live.current.riding, at: number | null = null) => {
    const next = TRAJECTORIES.find((t) => t.id === id) ?? live.current.trajectory;
    const clock =
      at === null
        ? { playing: true, at: 0, since: performance.now() }
        : { playing: false, at, since: 0 };
    // The pilot reads these on the very next frame. Watching, it first
    // cuts the free camera to where this trajectory is watched from.
    const before = live.current;
    live.current = { trajectory: next, riding: ride, clock };
    if (!ride && (before.riding || before.trajectory !== next)) stand.current = watchingPose(next);
    setTrajectory(next);
    setRiding(ride);
    setClock(clock);
  }, []);

  const stats = useMemo(() => {
    if (!flight) return null;
    const adjusted = flight.filter((f) => f.drawn !== f.asked);
    const holds = [...new Set(adjusted.map((f) => f.hold).filter((h) => h !== "none"))];
    return {
      adjusted: adjusted.length / 60,
      holds,
      cuts: flight.filter((f) => f.cut).length,
    };
  }, [flight]);

  const diagnostics = useMemo(
    () =>
      standing && {
        trajectories: () => TRAJECTORIES.map((t) => ({ id: t.id, seconds: t.seconds })),
        /** The buildings the camera keeps clear of now. */
        boxes: () => standing.boxes,
        /** How many buildings the lab's map holds. */
        counts: () => ({ buildings: world?.exports.buildings.buildings.length ?? 0 }),
        /** The near plane's envelope and the clear space kept beyond it. */
        clearance: () => ({
          envelope: nearEnvelope(lens),
          margin: gameCamera.config.clearance.margin_m,
        }),
        /** A trajectory flown frame by frame: the eye asked for and drawn. */
        flight: (id: string) =>
          fly(id).map((f) => ({
            seconds: f.seconds,
            askedEye: [...f.askedEye],
            eye: [...f.eye],
            asked: f.asked,
            drawn: f.drawn,
            hold: f.hold,
            cut: f.cut,
            blocked: f.blocked,
          })),
        /** Ride a trajectory, or watch it from outside: held at `seconds`,
         *  or playing from its start with none. */
        ride: (id: string, seconds: number | null = null) => choose(id, true, seconds),
        watch: (id: string, seconds: number | null = null) => choose(id, false, seconds),
        /** How far into its trajectory the clock stands, and stopping it there. */
        seconds: () => secondsAt(performance.now()),
        hold: () =>
          choose(live.current.trajectory.id, live.current.riding, secondsAt(performance.now())),
        setFallen,
      },
    [world, standing, fly, lens, choose, secondsAt],
  );

  if (!world || !meshes) return null;
  return (
    <>
      <LabViewport
        fixture="camera"
        world={worldFeed}
        buildings={buildingsFeed}
        obstacles={obstaclesFeed}
        overlay={overlayFeed}
        appearances={appearances}
        initialCamera={rig.place(lens, OBSERVER)}
        groundAt={surfaceZ}
        pilot={pilot}
        diagnostics={diagnostics ?? undefined}
      />
      <aside className="hud-panel lab-panel" data-testid="camera-panel">
        <strong>Camera clearance</strong>
        <div className="lab-hint">
          The eye asked for is amber (red inside a building's clearance); the eye drawn is cyan
          where it leaves it. WASD/arrows pan · Q/E turn · middle‑drag orbit · wheel zoom
        </div>
        <fieldset className="lab-field">
          <legend>Trajectory</legend>
          {TRAJECTORIES.map((t) => (
            <label key={t.id}>
              <input
                type="radio"
                name="trajectory"
                checked={t.id === trajectory.id}
                onChange={() => choose(t.id)}
              />
              {t.label}
            </label>
          ))}
        </fieldset>
        <div className="lab-row">
          <button type="button" onClick={() => choose(trajectory.id, !riding)}>
            {riding ? "Watch from outside" : "Ride the camera"}
          </button>
          <button type="button" onClick={() => choose(trajectory.id)}>
            Replay
          </button>
          <button type="button" onClick={() => window.__lab?.reset?.()}>
            Reset view
          </button>
        </div>
        <label>
          <input type="checkbox" checked={fallen} onChange={(e) => setFallen(e.target.checked)} />
          Blue has seen the courtyard block collapse
        </label>
        {stats && (
          <div data-testid="clearance">
            Adjusted for {stats.adjusted.toFixed(1)} of {trajectory.seconds} s
            {stats.holds.length > 0 && ` (${stats.holds.join(", ")})`} · {stats.cuts}{" "}
            {stats.cuts === 1 ? "cut" : "cuts"}
            {now && now.hold !== "none" && ` · now: ${now.hold}`}
          </div>
        )}
      </aside>
    </>
  );
}
