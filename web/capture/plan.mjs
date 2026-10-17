// A capture file plus parsed args → one recording job per scene.
//
// The capture's TypeScript validator owns its shape; this only reads what the
// runner needs (scene count, shot lengths) and refuses an empty scene list.
import { basename, join } from "node:path";

/** Return `{name, jobs, joined, report}`. Each job is
 * `{scene, url, seconds, webm, mp4}`: the capture route to record, the
 * scripted length (sum of shot seconds) and its output paths. `joined` is the
 * crossfaded MP4 of every scene, or null for a single scene. */
export function planCapture(capture, args) {
  const name = basename(args.file, ".json");
  const scenes = capture?.scenes ?? [];
  if (scenes.length === 0) throw new Error(`capture ${name} has no scenes`);
  const out = (file) => join(args.output, file);
  const jobs = scenes.map((scene, i) => ({
    scene: i,
    url: `/labs/capture?${new URLSearchParams({
      capture: name,
      scene: String(i),
      cards: args.cards ? "1" : "0",
      record: "1",
    })}`,
    seconds: scene.reel.shots.reduce((sum, shot) => sum + shot.seconds, 0),
    webm: out(`${name}-scene${i}.webm`),
    mp4: out(`${name}-scene${i}.mp4`),
  }));
  return {
    name,
    jobs,
    joined: jobs.length > 1 ? out(`${name}.mp4`) : null,
    report: out(`${name}.report.json`),
  };
}
