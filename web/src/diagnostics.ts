// The diagnostics report a player copies from settings to report a problem:
// the build, the machine, and a rolling window of how the page has run.
// Collection is on unless the player turns it off; nothing leaves the page
// except by the player's copy. Pages add live sections (`diagnosticsSource`).

const KEY = "battle.diagnostics";
/** Frames, inputs and long frames kept: the last several seconds of play. */
const WINDOW = 600;
const ERRORS = 20;

function load(): boolean {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    if (raw) return (JSON.parse(raw) as { enabled?: unknown }).enabled !== false;
  } catch {
    // Storage blocked: the default.
  }
  return true;
}

/** A ring of the last `size` values. */
class Ring<T> {
  private items: T[] = [];
  constructor(private readonly size: number) {}
  push(item: T) {
    this.items.push(item);
    if (this.items.length > this.size) this.items.splice(0, this.items.length - this.size);
  }
  get values(): readonly T[] {
    return this.items;
  }
  clear() {
    this.items = [];
  }
}

interface LongFrame {
  at: number;
  ms: number;
  blockingMs: number;
  /** The scripts that ran in it: what called each, and how long it took. */
  scripts: [string, number][];
}

const frames = new Ring<number>(WINDOW);
const inputDelays = new Ring<number>(WINDOW);
const longFrames = new Ring<LongFrame>(WINDOW);
/** Errors in the order first seen. A repeat (a failure every frame) counts on
 *  its first entry, so a flood cannot push out the error that started it. */
const errors = {
  entries: [] as { at: string; last: string; count: number; text: string }[],
  push(text: string) {
    const now = new Date().toISOString();
    const known = this.entries.find((e) => e.text === text.slice(0, 2000));
    if (known) {
      known.count++;
      known.last = now;
      return;
    }
    this.entries.push({ at: now, last: now, count: 1, text: text.slice(0, 2000) });
    if (this.entries.length > ERRORS) this.entries.splice(0, this.entries.length - ERRORS);
  },
  get values() {
    return this.entries;
  },
  clear() {
    this.entries = [];
  },
};
const sources = new Map<string, () => unknown>();
const listeners = new Set<() => void>();
let enabled = load();
let stop: (() => void) | null = null;

function start() {
  if (stop || typeof window === "undefined") return;
  const lifetime = new AbortController();
  const { signal } = lifetime;
  let last = 0;
  let raf = 0;
  const frame = (now: number) => {
    // A hidden page draws no frames: its gap is not a slow frame.
    if (last && !document.hidden) frames.push(now - last);
    last = now;
    raf = requestAnimationFrame(frame);
  };
  raf = requestAnimationFrame(frame);
  // How long a pointer move waited before the page could handle it.
  window.addEventListener("pointermove", (e) => inputDelays.push(performance.now() - e.timeStamp), {
    capture: true,
    passive: true,
    signal,
  });
  window.addEventListener("error", (e) => errors.push(`${e.message} (${e.filename}:${e.lineno})`), {
    signal,
  });
  window.addEventListener(
    "unhandledrejection",
    (e) => errors.push(`unhandled: ${String(e.reason)}`),
    { signal },
  );
  // The app reports its own failures (a battle's aborted resources) here.
  const consoleError = console.error;
  console.error = (...args: unknown[]) => {
    errors.push(args.map(String).join(" "));
    consoleError.apply(console, args);
  };
  let observer: PerformanceObserver | null = null;
  if (globalThis.PerformanceObserver?.supportedEntryTypes?.includes("long-animation-frame")) {
    observer = new PerformanceObserver((list) => {
      for (const e of list.getEntries() as PerformanceEntry[] & LoafEntry[])
        longFrames.push({
          at: Math.round(e.startTime),
          ms: Math.round(e.duration),
          blockingMs: Math.round(e.blockingDuration),
          scripts: e.scripts.map((s) => [scriptName(s), Math.round(s.duration)]),
        });
    });
    observer.observe({ type: "long-animation-frame" });
  }
  stop = () => {
    lifetime.abort();
    cancelAnimationFrame(raf);
    console.error = consoleError;
    observer?.disconnect();
    for (const ring of [frames, inputDelays, longFrames, errors]) ring.clear();
    stop = null;
  };
}

interface LoafScript {
  invoker: string;
  sourceFunctionName: string;
  sourceURL: string;
  sourceCharPosition: number;
  duration: number;
}
interface LoafEntry {
  startTime: number;
  duration: number;
  blockingDuration: number;
  scripts: LoafScript[];
}

const scriptName = (s: LoafScript) =>
  `${s.invoker} ${s.sourceFunctionName || "?"} ${s.sourceURL.split("/").pop()?.split("?")[0] ?? ""}:${s.sourceCharPosition}`;

/** The opt-out setting, for `useSyncExternalStore`, and its collection. */
export const diagnostics = {
  enabled: () => enabled,
  set(next: boolean) {
    enabled = next;
    try {
      globalThis.localStorage?.setItem(KEY, JSON.stringify({ enabled }));
    } catch {
      // Storage blocked: the setting lasts this page.
    }
    if (enabled) start();
    else stop?.();
    for (const l of listeners) l();
  },
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => void listeners.delete(listener);
  },
  /** Begin collecting if the player has not turned it off. */
  start() {
    if (enabled) start();
  },
};

/** A failure the player was shown (a loading screen's plate): its message
 *  and the details behind it, which the console never saw. */
export function diagnosticsFailure(message: string, details: readonly string[]) {
  if (stop) errors.push(`shown: ${[message, ...details].join(" | ")}`);
}

/** Add `name`'s live section to the report while the page shows it; returns
 *  its removal. A section that throws reports its error instead. */
export function diagnosticsSource(name: string, read: () => unknown): () => void {
  sources.set(name, read);
  return () => {
    if (sources.get(name) === read) sources.delete(name);
  };
}

/** Percentiles of `values`, rounded to 0.1. */
export function spread(values: readonly number[]) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const at = (f: number) => Math.round(sorted[Math.floor((sorted.length - 1) * f)] * 10) / 10;
  return { n: values.length, p50: at(0.5), p90: at(0.9), p99: at(0.99), max: at(1) };
}

/** The long frames' scripts, summed by name: where the main thread went. */
function heaviestScripts(entries: readonly LongFrame[]) {
  const total = new Map<string, number>();
  for (const f of entries)
    for (const [name, ms] of f.scripts) total.set(name, (total.get(name) ?? 0) + ms);
  return [...total]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 10)
    .map(([name, ms]) => ({ name, ms }));
}

/** The report as the player copies it. `build` names what is running. */
export function diagnosticsReport(build: Record<string, unknown>): string {
  const intervals = frames.values;
  const span = intervals.reduce((a, b) => a + b, 0);
  const long = longFrames.values;
  const nav = navigator as Navigator & { deviceMemory?: number };
  const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory;
  const live: Record<string, unknown> = {};
  for (const [name, read] of sources) {
    try {
      live[name] = read();
    } catch (error) {
      live[name] = { error: String(error) };
    }
  }
  return JSON.stringify(
    {
      report: "battlegame diagnostics",
      at: new Date().toISOString(),
      build,
      page: location.pathname + location.search,
      browser: navigator.userAgent,
      machine: {
        platform: navigator.platform,
        cores: navigator.hardwareConcurrency,
        memoryGb: nav.deviceMemory ?? null,
        screen: [screen.width, screen.height],
        window: [innerWidth, innerHeight],
        devicePixelRatio,
        jsHeapMb: memory ? Math.round(memory.usedJSHeapSize / 2 ** 20) : null,
      },
      collecting: enabled,
      frames: enabled && {
        fps: span > 0 ? Math.round((intervals.length * 1000) / span) : null,
        intervalMs: spread(intervals),
        // Of the window's time, how much long frames kept the page from input.
        blockedShare:
          span > 0
            ? Math.round((long.reduce((a, f) => a + f.blockingMs, 0) / span) * 100) / 100
            : null,
      },
      pointerInputDelayMs: enabled && spread(inputDelays.values),
      longFrames: enabled && {
        count: long.length,
        durationMs: spread(long.map((f) => f.ms)),
        heaviestScripts: heaviestScripts(long),
      },
      live,
      errors: errors.values,
    },
    null,
    2,
  );
}
