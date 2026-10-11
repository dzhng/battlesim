// Per-frame GPU time from a Metal System Trace, the same way for either client:
//
//   DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
//     xcrun xctrace record --template 'Metal System Trace' --attach <pid> \
//     --time-limit 3s --output <name>.trace
//   node gpu-trace.mjs <name>.trace <process name prefix>
//
// Attach to Godot, or to Chrome's GPU process ("Google Chrome for Testing
// Helper"). The trace is system-wide; only the named process's GPU work counts,
// and other processes' share is reported. Tracing slows the frames it records,
// so read a trace for proportions and gaps; absolute ms come from untraced
// runs.
// For each frame Metal assigned (by present) it reports the span from its
// first GPU work to the next frame's, the GPU busy time (the union of its
// encoders' intervals on every channel), the idle gap (span less busy), and
// each encoder's mean time by channel and command index (Godot's debug labels
// don't reach Metal's encoder labels; WebGPU's need Dawn's
// `use_user_defined_labels_in_backend`). Writes <name>.trace.json beside it.
import { execFileSync } from "node:child_process";
import { writeFile } from "node:fs/promises";

const XCTRACE = ["xctrace", "export", "--input"];
const TABLE = '/trace-toc/run[@number="1"]/data/table[@schema="metal-gpu-intervals"]';

/** The export's rows as arrays of top-level cells: `{tag, attrs, text, children}`,
 *  with every `ref` resolved to the element its `id` named. */
export function parseRows(xml) {
  const byId = new Map();
  const rows = [];
  const stack = [];
  const tags = /<(\/?)([\w-]+)((?:\s+[\w-]+="[^"]*"|\s+[\w-]+='[^']*')*)\s*(\/?)>|([^<]+)/g;
  for (const m of xml.matchAll(tags)) {
    const [, closing, tag, rawAttrs, selfClosing, text] = m;
    if (text !== undefined) {
      if (stack.length) stack.at(-1).text += text;
      continue;
    }
    if (closing) {
      // Elements outside a row were never opened on the stack.
      if (stack.at(-1)?.tag !== tag) continue;
      const done = stack.pop();
      if (done.tag === "row") rows.push(done.children);
      else stack.at(-1).children.push(done);
      continue;
    }
    const attrs = Object.fromEntries(
      [...(rawAttrs ?? "").matchAll(/([\w-]+)=(?:"([^"]*)"|'([^']*)')/g)].map((a) => [
        a[1],
        a[2] ?? a[3],
      ]),
    );
    const element = { tag, attrs, text: "", children: [] };
    if (attrs.id) byId.set(attrs.id, element);
    if (tag !== "row" && !stack.some((e) => e.tag === "row")) continue;
    if (selfClosing) {
      if (stack.length) stack.at(-1).children.push(attrs.ref ? byId.get(attrs.ref) : element);
    } else stack.push(element);
  }
  return rows;
}

const COLUMNS = ["start", "duration", "channel", "frame", "latency", "depth", "label", "state"];
const PROCESS_COLUMN = 10;

/** Merged length of `[start, end)` intervals. */
export function unionLength(intervals) {
  let total = 0;
  let open = null;
  for (const [a, b] of [...intervals].sort((x, y) => x[0] - y[0])) {
    if (!open || a > open[1]) {
      if (open) total += open[1] - open[0];
      open = [a, b];
    } else open[1] = Math.max(open[1], b);
  }
  return open ? total + open[1] - open[0] : total;
}

/** Each frame's span, busy and idle ms, and each encoder's mean ms per frame,
 *  from the GPU intervals of processes whose name starts with `process`. */
export function frameAnatomy(rows, process) {
  const frames = new Map();
  const own = [];
  const foreign = [];
  for (const row of rows) {
    const name = row[PROCESS_COLUMN]?.attrs.fmt ?? "";
    const cell = Object.fromEntries(COLUMNS.map((c, i) => [c, row[i]]));
    if (cell.depth?.attrs.fmt === "0") {
      const start = Number(cell.start.text);
      (name.startsWith(process) ? own : foreign).push([start, start + Number(cell.duration.text)]);
    }
    if (!name.startsWith(process)) continue;
    const frame = Number(cell.frame?.text);
    if (!Number.isFinite(frame)) continue;
    const start = Number(cell.start.text);
    const end = start + Number(cell.duration.text);
    const command =
      /(Render|Compute|Blit) Command \d+/.exec(cell.label?.attrs.fmt ?? "")?.[0] ?? "other";
    const encoder = `${cell.channel.attrs.fmt} · ${command}${cell.depth.attrs.fmt === "0" ? "" : " (nested)"}`;
    if (!frames.has(frame)) frames.set(frame, { intervals: [], encoders: new Map() });
    const f = frames.get(frame);
    if (cell.depth.attrs.fmt === "0") f.intervals.push([start, end]);
    f.encoders.set(encoder, (f.encoders.get(encoder) ?? 0) + (end - start));
  }
  // A frame's span runs to the next frame's first work; the last frame has no
  // next and the first may be cut by the trace's start, so both are dropped.
  const numbers = [...frames.keys()].sort((a, b) => a - b);
  const first = (n) => Math.min(...frames.get(n).intervals.map(([a]) => a));
  const anatomy = [];
  for (const [i, n] of numbers.entries()) {
    if (i === 0 || i === numbers.length - 1) continue;
    const f = frames.get(n);
    if (f.intervals.length === 0) continue;
    const span = first(numbers[i + 1]) - first(n);
    const busy = unionLength(f.intervals);
    anatomy.push({
      frame: n,
      spanMs: span / 1e6,
      busyMs: busy / 1e6,
      idleMs: (span - busy) / 1e6,
      encoders: f.encoders,
    });
  }
  const encoders = new Map();
  for (const f of anatomy)
    for (const [name, ns] of f.encoders) encoders.set(name, (encoders.get(name) ?? 0) + ns / 1e6);
  const mean = (pick) => anatomy.reduce((s, f) => s + pick(f), 0) / anatomy.length;
  const foreignBusy = unionLength(foreign);
  return {
    process,
    frames: anatomy.length,
    // Every other process's GPU work over the whole trace, and its share of
    // all GPU work: a trace with more than a trivial share is re-recorded.
    foreign: { busyMs: foreignBusy / 1e6, share: foreignBusy / (unionLength(own) + foreignBusy) },
    spanMs: mean((f) => f.spanMs),
    busyMs: mean((f) => f.busyMs),
    idleMs: mean((f) => f.idleMs),
    encodersMs: Object.fromEntries(
      [...encoders]
        .map(([name, total]) => [name, total / anatomy.length])
        .sort((a, b) => b[1] - a[1]),
    ),
    perFrame: anatomy.map(({ frame, spanMs, busyMs, idleMs }) => ({
      frame,
      spanMs,
      busyMs,
      idleMs,
    })),
  };
}

/** Above this share of the GPU's work from other processes, a trace measured
 *  a shared GPU. Below it the share is ambient (the desktop, the app showing
 *  the run) and hits both clients alike. */
const FOREIGN_SHARE_MAX = 0.1;

export async function main([trace, name] = globalThis.process.argv.slice(2)) {
  if (!trace || !name) throw new Error("usage: node gpu-trace.mjs <trace> <process name prefix>");
  const xml = execFileSync("xcrun", [...XCTRACE, trace, "--xpath", TABLE], {
    encoding: "utf8",
    maxBuffer: 1 << 30,
    env: {
      ...globalThis.process.env,
      DEVELOPER_DIR:
        globalThis.process.env.DEVELOPER_DIR ?? "/Applications/Xcode.app/Contents/Developer",
    },
  });
  const anatomy = frameAnatomy(parseRows(xml), name);
  if (anatomy.frames === 0) throw new Error(`${trace}: no whole frame of GPU work from ${name}`);
  await writeFile(`${trace}.json`, `${JSON.stringify(anatomy, null, 2)}\n`);
  console.log(
    `${anatomy.frames} frames of ${name}: span ${anatomy.spanMs.toFixed(2)} ms, GPU busy ${anatomy.busyMs.toFixed(2)} ms, idle ${anatomy.idleMs.toFixed(2)} ms; other processes ${(100 * anatomy.foreign.share).toFixed(1)}% of GPU work`,
  );
  for (const [encoder, ms] of Object.entries(anatomy.encodersMs).slice(0, 15))
    console.log(`  ${ms.toFixed(2).padStart(7)} ms  ${encoder}`);
  if (anatomy.foreign.share > FOREIGN_SHARE_MAX) {
    console.error(
      `discard: other processes did ${(100 * anatomy.foreign.share).toFixed(1)}% of the GPU's work; record again on a quiet GPU`,
    );
    return 1;
  }
  return 0;
}

if (import.meta.url === `file://${globalThis.process.argv[1]}`)
  main().then(
    (code) => globalThis.process.exit(code),
    (error) => {
      console.error(error.message);
      globalThis.process.exit(2);
    },
  );
