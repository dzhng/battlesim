// @vitest-environment node
import { resolve } from "node:path";
import { expect, test } from "vitest";
import { parseCaptureArgs } from "./capture/args.mjs";
import { planCapture } from "./capture/plan.mjs";
import { transcodeArgs } from "./capture/transcode.mjs";
import { CROSSFADE_S, joinArgs, joinOffsets } from "./capture/join.mjs";
import { buildReport, frameStats } from "./capture/report.mjs";
import { CaptureError, recordingVerdict, recordTimeoutMs } from "./capture/recording.mjs";

test("capture args: defaults, 16:9 width from height, flags, and refusals by name", () => {
  const plain = parseCaptureArgs(["--", "fixtures/captures/menu.json"], { root: "/repo" });
  expect(plain).toMatchObject({
    file: "/repo/fixtures/captures/menu.json",
    height: 1080,
    width: 1920,
    fps: 60,
    cards: true,
  });
  expect(plain.output).toMatch(/[\\/]throwaway[\\/]capture$/);

  expect(
    parseCaptureArgs(["a.json", "--height=1440", "--fps=30", "--no-cards", "--output=out"], {
      root: "/repo",
    }),
  ).toEqual({
    file: "/repo/a.json",
    height: 1440,
    width: 2560,
    fps: 30,
    cards: false,
    output: resolve("/repo", "out"),
  });

  expect(() => parseCaptureArgs([])).toThrow(/capture file/);
  expect(() => parseCaptureArgs(["menu.yaml"])).toThrow(/\.json/);
  expect(() => parseCaptureArgs(["a.json", "b.json"])).toThrow(/one capture file/);
  expect(() => parseCaptureArgs(["a.json", "--height=2160"])).toThrow(/--height.*1080.*1440/);
  expect(() => parseCaptureArgs(["a.json", "--fps=0"])).toThrow(/--fps/);
  expect(() => parseCaptureArgs(["a.json", "--fps=29.97"])).toThrow(/--fps/);
  expect(() => parseCaptureArgs(["a.json", "--width=1920"])).toThrow(/unknown flag --width/);
  expect(() => parseCaptureArgs(["a.json", "--output="])).toThrow(/--output/);
});

test("capture plan: one recording job per scene, a join only for several scenes", () => {
  const args = parseCaptureArgs(["fixtures/captures/trailer.json", "--no-cards", "--output=/out"]);
  const scene = (seconds) => ({
    map: "market-town",
    encounter: "menu",
    seed: 7,
    warm_s: 4,
    reel: { fade_s: 0.6, shots: seconds.map((s) => ({ seconds: s })) },
  });

  const plan = planCapture({ scenes: [scene([10, 5.5]), scene([8])] }, args);
  expect(plan.name).toBe("trailer");
  expect(plan.jobs).toEqual([
    {
      scene: 0,
      url: "/labs/capture?capture=trailer&scene=0&cards=0&record=1&fps=60",
      seconds: 15.5,
      webm: "/out/trailer-scene0.webm",
      mp4: "/out/trailer-scene0.mp4",
    },
    {
      scene: 1,
      url: "/labs/capture?capture=trailer&scene=1&cards=0&record=1&fps=60",
      seconds: 8,
      webm: "/out/trailer-scene1.webm",
      mp4: "/out/trailer-scene1.mp4",
    },
  ]);
  expect(plan.joined).toBe("/out/trailer.mp4");
  expect(plan.report).toBe("/out/trailer.report.json");

  const single = planCapture(
    { scenes: [scene([3])] },
    parseCaptureArgs(["fixtures/captures/one.json", "--fps=30"]),
  );
  expect(single.jobs[0].url).toContain("cards=1");
  expect(single.jobs[0].url).toContain("fps=30");
  expect(single.joined).toBeNull();

  expect(() => planCapture({ scenes: [] }, args)).toThrow(/no scenes/);
  // A malformed file is refused by name, never a bare TypeError.
  expect(() => planCapture({ scenes: [scene([3]), { map: "market-town" }] }, args)).toThrow(
    "capture scenes[1].reel.shots: none",
  );
  // The route loads captures by name from fixtures/captures/ only.
  expect(() => planCapture({ scenes: [scene([3])] }, parseCaptureArgs(["one.json"]))).toThrow(
    /fixtures\/captures/,
  );
});

/** The value that follows `flag` in an ffmpeg argv. */
const after = (argv, flag) => argv[argv.indexOf(flag) + 1];

test("transcode: VP9/Opus WebM to CFR H.264 yuv420p + AAC at the exact frame size", () => {
  const argv = transcodeArgs({
    input: "in.webm",
    output: "out.mp4",
    width: 2560,
    height: 1440,
    fps: 60,
  });
  expect(after(argv, "-i")).toBe("in.webm");
  expect(argv.at(-1)).toBe("out.mp4");
  expect(after(argv, "-vf")).toContain("scale=2560:1440");
  expect(after(argv, "-vf")).toContain("pad=2560:1440");
  expect(after(argv, "-vf")).toContain("fps=60");
  expect(after(argv, "-c:v")).toBe("libx264");
  expect(after(argv, "-pix_fmt")).toBe("yuv420p");
  expect(after(argv, "-fps_mode")).toBe("cfr");
  expect(after(argv, "-r")).toBe("60");
  expect(after(argv, "-c:a")).toBe("aac");
  expect(after(argv, "-movflags")).toBe("+faststart");
});

test("join: chained xfade/acrossfade with offsets from each scene's length", () => {
  const d = CROSSFADE_S;
  expect(d).toBeGreaterThan(0);
  expect(d).toBeLessThan(1);
  // Each transition starts `d` before the running joined length ends.
  expect(joinOffsets([10, 8])).toEqual([10 - d]);
  expect(joinOffsets([10, 8, 6])).toEqual([10 - d, 10 + 8 - 2 * d]);

  const argv = joinArgs({
    inputs: [
      { file: "a.mp4", seconds: 10 },
      { file: "b.mp4", seconds: 8 },
      { file: "c.mp4", seconds: 6 },
    ],
    output: "all.mp4",
    fps: 60,
  });
  expect(argv.filter((a, i) => argv[i - 1] === "-i")).toEqual(["a.mp4", "b.mp4", "c.mp4"]);
  expect(argv.at(-1)).toBe("all.mp4");
  const graph = after(argv, "-filter_complex");
  expect(graph).toContain(
    `[0:v][1:v]xfade=transition=fadeblack:duration=${d}:offset=${10 - d}[v1]`,
  );
  expect(graph).toContain(
    `[v1][2:v]xfade=transition=fadeblack:duration=${d}:offset=${18 - 2 * d}[v2]`,
  );
  expect(graph).toContain(`[0:a][1:a]acrossfade=d=${d}[a1]`);
  expect(graph).toContain(`[a1][2:a]acrossfade=d=${d}[a2]`);
  expect(argv.filter((a, i) => argv[i - 1] === "-map")).toEqual(["[v2]", "[a2]"]);

  expect(() =>
    joinArgs({ inputs: [{ file: "a.mp4", seconds: 10 }], output: "x", fps: 60 }),
  ).toThrow(/two/);
  expect(() => joinOffsets([10, d / 2])).toThrow(/scene 1/);
});

test("report: warns below 90% of the target fps", () => {
  const output = (scene, avgFps) => ({
    scene,
    file: `s${scene}.mp4`,
    seconds: 10,
    avgFps,
    worstGapMs: 40,
    audio: true,
  });
  const report = buildReport({
    capture: "fixtures/captures/menu.json",
    fps: 60,
    outputs: [output(0, 54), output(1, 53.9), output(2, 59.8)],
    joined: "menu.mp4",
  });
  expect(report.capture).toBe("fixtures/captures/menu.json");
  expect(report.joined).toBe("menu.mp4");
  expect(report.outputs).toHaveLength(3);
  expect(report.warnings).toEqual(["scene 1: avg 53.9 fps is below 90% of 60"]);
  expect(buildReport({ capture: "c", fps: 60, outputs: [output(0, 60)] }).joined).toBeNull();
});

test("frame stats: average fps and the worst gap from recorded frame times", () => {
  // 1 s at 60 fps with one dropped frame.
  const times = Array.from({ length: 61 }, (_, i) => i / 60).filter((_, i) => i !== 30);
  const stats = frameStats(times);
  expect(stats.avgFps).toBeCloseTo(59, 5);
  expect(stats.worstGapMs).toBeCloseTo(1000 / 30, 5);
  expect(frameStats([0])).toEqual({ avgFps: 0, worstGapMs: 0 });
});

test("recording verdict: a hang, a failure or an empty recording fails by name", () => {
  const timeoutMs = recordTimeoutMs(10);
  expect(timeoutMs).toBeGreaterThan(10_000);
  const verdict = (result, bytes) => recordingVerdict({ scene: 1, result, bytes, timeoutMs });
  const ok = { seconds: 10 };
  expect(verdict(ok, 1000)).toBe(ok);

  const named = (error) => error instanceof CaptureError && `${error.name}: ${error.message}`;
  // Nothing arrived: the capture never started, which is what a prompt looks like.
  expect(named(verdict(undefined, 0))).toMatch(
    /^TabCaptureNotStarted: scene 1: .*--auto-accept-this-tab-capture/,
  );
  expect(named(verdict(undefined, 5))).toMatch(/^TabCaptureTimeout: scene 1/);
  expect(named(verdict({ error: "no-audio-track: x" }, 0))).toBe(
    "TabCaptureFailed: scene 1: no-audio-track: x",
  );
  expect(named(verdict(ok, 0))).toMatch(/^TabCaptureEmpty/);
});
