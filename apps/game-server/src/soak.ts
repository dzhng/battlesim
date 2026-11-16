import { decodePublication } from "../../../packages/multiplayer/src/stream/publication.ts";
// `bun run server:soak [seconds]`: a generated 5v5 of AIs on a battle thread,
// watched through one seat's stream. Each second it prints the ticks run,
// how many took longer than a tick's budget, the 95th percentile step time,
// how many ticks the battle has shed against its schedule since it began,
// and the process's memory; at the end, the same over the whole run.
import { allAi, toBattleSetup } from "../../../packages/multiplayer/src/setup.ts";
import { preset } from "../../../packages/skirmish-setup/src/lobby.ts";
import { loopback } from "../../../packages/multiplayer/src/link.ts";
import { createBattleHost } from "./battleHost/index.ts";
import { nodeSetupData } from "./serverData.ts";

const root = process.cwd();
const duration = process.argv[2] === undefined ? null : Number(process.argv[2]) * 1000;
if (duration !== null && (!Number.isFinite(duration) || duration < 0))
  throw new Error("duration must be non-negative seconds");

const data = await nodeSetupData(root);
const setup = toBattleSetup(allAi(preset("5v5")), "skirmish");
const host = createBattleHost({ candidates: [setup] }, "thread", data);
const observer = loopback({ side: "blue", player: "blue-1" });
host.attach(observer.port);
let tickHz = 0;
let latestTick = 0;
let startedAt = 0;
const allStepMs: number[] = [];
const secondStepMs: number[] = [];
let secondTicks = 0;
let secondLate = 0;
let stopping = false;
let reportTimer: ReturnType<typeof setInterval> | undefined;

function percentile95(values: number[]) {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1)];
}
function measures(values: number[], ticks: number, late: number) {
  const elapsed = performance.now() - startedAt;
  const expected = Math.floor((elapsed * tickHz) / 1000);
  const memory = process.memoryUsage();
  return {
    ticks,
    overBudget: late,
    p95StepMs: percentile95(values),
    behind: Math.max(0, expected - latestTick),
    rss: memory.rss,
  };
}
function stop() {
  if (stopping) return;
  stopping = true;
  if (reportTimer) clearInterval(reportTimer);
  host.dispose();
  console.log(
    JSON.stringify({
      event: "summary",
      ...measures(
        allStepMs,
        allStepMs.length,
        allStepMs.filter((step) => step > 1000 / tickHz).length,
      ),
    }),
  );
  if (duration !== null) process.exit(0);
}

const stream = (async () => {
  for await (const record of observer.link.records(null)) {
    if (record.kind !== "publication") continue;
    const { tick, stepMs } = decodePublication(record.payload);
    latestTick = tick;
    allStepMs.push(stepMs);
    secondStepMs.push(stepMs);
    secondTicks++;
    if (stepMs > 1000 / tickHz) secondLate++;
  }
})();

host.onEnd(() => stop());
const ready = await host.ready;
tickHz = ready.tickHz;
startedAt = performance.now();
console.log(JSON.stringify({ event: "ready", tickHz }));
reportTimer = setInterval(() => {
  console.log(
    JSON.stringify({ event: "second", ...measures(secondStepMs, secondTicks, secondLate) }),
  );
  secondStepMs.length = 0;
  secondTicks = 0;
  secondLate = 0;
  if (duration !== null && performance.now() - startedAt >= duration) stop();
}, 1000);
process.once("SIGINT", stop);
if (duration === 0) stop();
await stream;
