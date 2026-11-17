import { useMemo, useRef, useState } from "react";
import { vec3 } from "math";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { BattleSession } from "./useBattleSession";
import type { ViewportPilot } from "./viewportRender";
import { recordCamera, replayCameraAt, type ReplayCameraTrack } from "./replayCamera";

/** One human recording per authority lifetime; watching never records a track. */
export function useReplayCamera(
  session: BattleSession,
  initial: Camera3DParams,
  recorded: ReplayCameraTrack | undefined,
  playing: boolean,
  inCommand: boolean,
) {
  const { sim, viewer } = session;
  const [following, setFollowing] = useState(true);
  const recording = useMemo<ReplayCameraTrack | undefined>(
    () =>
      sim.client && playing && viewer
        ? { player: viewer.id, side: viewer.side, samples: [] }
        : undefined,
    [sim.client, playing, viewer],
  );
  const live = useRef({ client: sim.client, following });
  live.current = { client: sim.client, following };
  const pilot = useMemo<ViewportPilot | undefined>(() => {
    if (!recorded?.samples.length) return undefined;
    const camera = { ...initial, target: vec3.clone(initial.target) };
    return {
      frameRate: true,
      pose(now) {
        const { client, following } = live.current;
        const time = client?.presentation?.clock(now);
        return following && time != null && replayCameraAt(camera, recorded, time) ? camera : null;
      },
    };
  }, [recorded, initial]);
  return {
    track: recorded ?? recording,
    pilot,
    following,
    setFollowing,
    record(time: number, camera: Camera3DParams) {
      if (recording && inCommand) recordCamera(recording, time, camera);
    },
  };
}
