// Command line of `bun run --cwd web capture <file.json> [flags]`.
//
// Relative paths resolve from the repo root, where the command is typed:
// `bun run --cwd web` moves the process into web/ and keeps no trace of the
// caller's directory.
import { resolve } from "node:path";

export const REPO_ROOT = resolve(new URL("../..", import.meta.url).pathname);
/** Output is 16:9 at one of these heights; 4K is out of scope. */
export const HEIGHTS = [1080, 1440];
/** The page draws at most at the display's refresh, 120 Hz on the fastest Mac. */
export const MAX_FPS = 120;

export const USAGE = `usage: bun run --cwd web capture <file.json> [flags]

  --height=1080|1440   frame height; width is height x 16/9 (default 1080)
  --fps=<n>            target frame rate, an integer 1 to ${MAX_FPS} (default 60)
  --no-cards           hide the unit info cards
  --output=<dir>       output directory (default throwaway/capture/)`;

/** Parse argv (without node and the script) into
 * `{file, width, height, fps, cards, output}`, throwing a named error for
 * anything it does not accept. */
export function parseCaptureArgs(argv, { root = REPO_ROOT } = {}) {
  const args = {
    file: null,
    height: 1080,
    width: 0,
    fps: 60,
    cards: true,
    output: resolve(root, "throwaway/capture"),
  };
  for (const arg of argv) {
    if (arg === "--") continue;
    if (arg === "--no-cards") {
      args.cards = false;
    } else if (arg.startsWith("--height=")) {
      args.height = Number(arg.slice("--height=".length));
      if (!HEIGHTS.includes(args.height)) {
        throw new Error(`--height must be ${HEIGHTS.join(" or ")}, got ${arg.slice(9)}`);
      }
    } else if (arg.startsWith("--fps=")) {
      const text = arg.slice("--fps=".length);
      args.fps = Number(text);
      if (
        !/^\d+$/.test(text) ||
        !Number.isSafeInteger(args.fps) ||
        args.fps < 1 ||
        args.fps > MAX_FPS
      ) {
        throw new Error(`--fps must be an integer from 1 to ${MAX_FPS}, got ${text}`);
      }
    } else if (arg.startsWith("--output=")) {
      const dir = arg.slice("--output=".length);
      if (!dir) throw new Error("--output needs a directory");
      args.output = resolve(root, dir);
    } else if (arg.startsWith("-")) {
      throw new Error(`unknown flag ${arg.split("=")[0]}\n\n${USAGE}`);
    } else if (args.file) {
      throw new Error(`expected one capture file, got ${args.file} and ${arg}`);
    } else {
      if (!arg.endsWith(".json")) throw new Error(`capture file must be .json, got ${arg}`);
      args.file = resolve(root, arg);
    }
  }
  if (!args.file) throw new Error(`missing the capture file\n\n${USAGE}`);
  args.width = (args.height * 16) / 9;
  return args;
}
