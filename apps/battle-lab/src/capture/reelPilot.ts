// A reel's pilot: the camera framing and veil at each moment of a reel over a
// live battle. Pure but for its memory of where its subject was: a tracking
// shot glides after its unit, and holds where the unit last stood once it is
// gone. The scene player asks it once a frame.
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { sampleReel, type Reel } from "./capture";

/** How long a tracking shot's camera takes to catch up with its unit, seconds:
 *  it glides with the unit instead of stepping with each simulation tick. */
const TRACK_LAG_S = 0.5;

/** A followed unit's ground position now, or null once it is gone. */
export type UnitAt = (id: number) => ArrayLike<number> | null;

export interface PilotFrame {
  pose: CameraPose;
  /** 1 is a black picture, 0 the battle unveiled. */
  black: number;
  /** Every shot has played. */
  done: boolean;
}

/** The pilot of `reel`: called with seconds into the reel (never decreasing)
 *  and where its units stand, it frames that moment. */
export function reelPilot(reel: Reel): (t: number, unitAt: UnitAt) => PilotFrame {
  // The tracked unit's smoothed position; it holds where a fallen unit was.
  let tracked: { follow: number; at: [number, number]; t: number } | null = null;
  // The last framed subject's world position.
  let subject: readonly [number, number] = reel.shots[0].from.target;
  const track = (follow: number, t: number, unitAt: UnitAt): [number, number] | null => {
    const live = unitAt(follow);
    if (tracked?.follow !== follow) tracked = live && { follow, at: [live[0], live[1]], t };
    if (!tracked) return null;
    if (live) {
      const k = 1 - Math.exp(-(t - tracked.t) / TRACK_LAG_S);
      tracked.at = [
        tracked.at[0] + (live[0] - tracked.at[0]) * k,
        tracked.at[1] + (live[1] - tracked.at[1]) * k,
      ];
    }
    tracked.t = t;
    return tracked.at;
  };
  return (t, unitAt) => {
    const { pose, follow, black, done } = sampleReel(reel, t);
    const [x, y] = pose.target;
    if (follow === null) {
      subject = [x, y];
      return { pose, black, done };
    }
    // A unit gone before its shot opens: frame where the last subject was.
    const unit = track(follow, t, unitAt) ?? subject;
    subject = unit;
    return { pose: { ...pose, target: [unit[0] + x, unit[1] + y] }, black, done };
  };
}
