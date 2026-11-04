// `bun run --cwd web hdr-capture <run-dir> <scene> [flags]`: an HDR clip of a
// recorded battle. The release native client replays the run with the HDR
// look, writing its HLG signal as 16-bit PNG frames at a fixed rate and the battle's
// sound cut to the same window; ffmpeg encodes them to 8-bit HLG H.264, which
// X's web uploader accepts (or 10-bit HLG HEVC with --hevc), and the stream's
// tags are checked.
//
// <run-dir> is a native parity run (`bun run --cwd web scene --
// native-parity-capture` writes one under throwaway/native-parity/), the
// parent of <scene>/.

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { REPO_ROOT } from "./args.mjs";
import { hdrEncodeArgs, hdrProbeArgs, hlgTagErrors } from "./hdr.mjs";

const NATIVE_CLIENT = join(REPO_ROOT, "target/release/native-client");
const USAGE = `usage: bun run --cwd web hdr-capture <run-dir> <scene> [flags]

  --fps=<n>       frame rate (default 60)
  --from-ms=<n>   start of the clip in reel milliseconds (default 0)
  --to-ms=<n>     end of the clip (default: the end of the reel)
  --no-music      leave the music out (for a clip with its own soundtrack)
  --hevc          write 10-bit HEVC Main 10 (<scene>-hdr-hevc.mp4) instead of
                  8-bit H.264 High; X's web uploader hangs on HEVC
  --keep-frames   keep the frames after encoding
  --output=<dir>  output directory (default throwaway/hdr-capture/)`;

function options(argv) {
  const { values, positionals } = parseArgs({
    args: argv.filter((arg) => arg !== "--"),
    allowPositionals: true,
    options: {
      fps: { type: "string", default: "60" },
      "from-ms": { type: "string" },
      "to-ms": { type: "string" },
      "no-music": { type: "boolean", default: false },
      hevc: { type: "boolean", default: false },
      "keep-frames": { type: "boolean", default: false },
      output: { type: "string", default: "throwaway/hdr-capture" },
    },
  });
  const [runDir, scene] = positionals;
  if (!runDir || !scene || positionals.length > 2) throw new Error(USAGE);
  return {
    ...values,
    runDir: resolve(REPO_ROOT, runDir),
    scene,
    output: resolve(REPO_ROOT, values.output),
  };
}

function run(command, args) {
  console.log(`  ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    stdio: "inherit",
    env: { ...process.env, BATTLEGAME_ROOT: REPO_ROOT },
  });
  if (result.status !== 0) throw new Error(`${command} failed with exit code ${result.status}`);
}

function checkTags(file, hevc) {
  const probe = JSON.parse(execFileSync("ffprobe", hdrProbeArgs(file), { encoding: "utf8" }));
  const errors = hlgTagErrors(probe, { hevc });
  if (errors.length > 0) throw new Error(`${file} is not tagged as HLG:\n  ${errors.join("\n  ")}`);
}

function main() {
  const args = options(process.argv.slice(2));
  if (!existsSync(NATIVE_CLIENT))
    throw new Error(`no native client at ${NATIVE_CLIENT}: cargo build --release -p native-client`);
  const frames = join(args.output, "frames");
  const sound = join(args.output, "sound.wav");
  rmSync(frames, { recursive: true, force: true });
  mkdirSync(frames, { recursive: true });

  console.log("Rendering HDR frames...");
  run(NATIVE_CLIENT, [
    "--run",
    args.runDir,
    "--scene",
    args.scene,
    "--out",
    frames,
    "--sound",
    sound,
    "--hdr",
    "--fps",
    args.fps,
    "--window",
    "offscreen",
    "--pace",
    "0",
    ...(args["from-ms"] ? ["--from-ms", args["from-ms"]] : []),
    ...(args["to-ms"] ? ["--to-ms", args["to-ms"]] : []),
    ...(args["no-music"] ? ["--no-music"] : []),
  ]);
  const count = readdirSync(frames).filter((f) => f.endsWith(".png")).length;
  if (count === 0) throw new Error(`the native client wrote no frames to ${frames}`);
  const size = ["-v", "error", "-show_entries", "stream=width,height", "-of", "json"];
  const { width, height } = JSON.parse(
    execFileSync("ffprobe", [...size, join(frames, "frame-000000.png")], { encoding: "utf8" }),
  ).streams[0];

  console.log(`Encoding ${count} frames at ${width}x${height}...`);
  const outputs = [];
  for (const hevc of [args.hevc]) {
    const output = join(args.output, `${args.scene}-hdr${hevc ? "-hevc" : ""}.mp4`);
    const framePattern = join(frames, "frame-%06d.png");
    run(
      "ffmpeg",
      hdrEncodeArgs({ framePattern, audio: sound, output, fps: args.fps, width, height, hevc }),
    );
    checkTags(output, hevc);
    outputs.push(output);
  }
  if (!args["keep-frames"]) rmSync(frames, { recursive: true });
  console.log(`\nHDR clip, tagged HLG: ${outputs.join(", ")}`);
}

try {
  main();
} catch (err) {
  console.error(`\nError: ${err.message}`);
  process.exit(1);
}
