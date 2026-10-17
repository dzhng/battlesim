// @vitest-environment node
import { expect, test } from "vitest";
import { mapPreviews, type PreviewState } from "../src/battle/prepare/preview";
import {
  PreparationFailed,
  type Preparation,
  type PreparedSession,
} from "../src/battle/prepare/client";
import type { PrepareBattleRequest, PreparationMessage } from "../src/battle/prepare/protocol";

/** A request for `label`'s map, at `seed`: the label tells two lobby states apart. */
const candidate =
  (label: string) =>
  (seed: string): PrepareBattleRequest => ({
    map_source: {
      kind: "generated",
      request: {
        type: "mixed",
        size: "small",
        seed,
        region: label,
        profile: "skirmish",
        generator_version: "test",
        preset_revision: "test",
        template_catalog_hash: "test",
        limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
      },
    },
    players: [
      { id: "blue-1", side: "blue", faction: "us", controller: "human", human_capable: true },
      {
        id: "red-1",
        side: "red",
        faction: "eastern",
        controller: "active_ai",
        human_capable: false,
      },
    ],
    battle_seed: 7,
  });
const documents = { rules: "{}", presets: "{}", templates: "[]" };
const policy = { max_generated_attempts: 3, generated_deadline_ms: 8000 };

const labelOf = (request: PrepareBattleRequest) => request.map_source.request.region;
const prepared = (request: PrepareBattleRequest): PreparedSession => ({
  scenario: "{}",
  report: { request } as PreparedSession["report"],
  preview: {
    map: `map ${labelOf(request)}`,
    sites: "sites",
    svg: `<svg>${labelOf(request)}</svg>`,
  },
  connect: () => ({ send: () => {}, close: () => {} }),
});
const refusal = () =>
  new PreparationFailed(
    "refused",
    [{ code: "generation_failed", feature: null, location: "$", message: "cannot fit" }],
    "map",
  );

/** A preparation the test settles by hand, and whether its worker was closed. */
function pending(message: PreparationMessage) {
  if (message.type !== "prepare") throw new Error("a preview only prepares");
  let settle!: { resolve(s: PreparedSession): void; reject(e: unknown): void };
  const battle = new Promise<PreparedSession>((resolve, reject) => {
    settle = { resolve, reject };
  });
  const run = { message, closed: false, ...settle };
  const preparation: Preparation = { battle, cancel: () => void (run.closed = true) };
  return { run, preparation };
}

function harness() {
  const states: PreviewState[] = [];
  const runs: ReturnType<typeof pending>["run"][] = [];
  let seed = 0;
  let deadline: (() => void) | null = null;
  const previews = mapPreviews(
    { documents, policy, onChange: (s) => states.push(s) },
    {
      seed: () => String(++seed),
      prepare: (message) => {
        const { run, preparation } = pending(message);
        runs.push(run);
        return preparation;
      },
      schedule: (run) => {
        deadline = run;
        return () => {
          if (deadline === run) deadline = null;
        };
      },
    },
  );
  const settled = () => new Promise((resolve) => setTimeout(resolve, 0));
  return { states, runs, previews, settled, expire: () => deadline?.() };
}

test("a preview shows loading at once, and a finished one keeps its picture and closes its worker", async () => {
  const { states, runs, previews, settled } = harness();
  previews.show(candidate("a"));
  expect(previews.state.status).toBe("loading");
  expect(runs[0].message.type === "prepare" && runs[0].message.preview).toBe(true);
  runs[0].resolve(prepared(runs[0].message.type === "prepare" ? runs[0].message.request : null!));
  await settled();
  expect(previews.state).toEqual({
    status: "ready",
    preview: {
      request: candidate("a")("1"),
      map: "map a",
      sites: "sites",
      svg: "<svg>a</svg>",
    },
  });
  expect(runs[0].closed).toBe(true);
  expect(states.map((s) => s.status)).toEqual(["loading", "ready"]);
});

test("a stale preview, answering after a newer request, is dropped", async () => {
  const { states, runs, previews, settled } = harness();
  previews.show(candidate("old"));
  previews.show(candidate("new"));
  expect(runs[0].closed).toBe(true);
  const request = (i: number) =>
    runs[i].message.type === "prepare" ? runs[i].message.request : null!;
  runs[0].resolve(prepared(request(0)));
  await settled();
  expect(previews.state.status).toBe("loading");
  runs[1].resolve(prepared(request(1)));
  await settled();
  expect(previews.state.status === "ready" && labelOf(previews.state.preview.request)).toBe("new");
  expect(states.filter((s) => s.status === "ready")).toHaveLength(1);
});

test("a refusal another seed can fix shows retrying, then ready", async () => {
  const { states, runs, previews, settled } = harness();
  previews.show(candidate("a"));
  runs[0].reject(refusal());
  await settled();
  expect(previews.state).toEqual({ status: "retrying", attempt: 2 });
  const request = runs[1].message.type === "prepare" ? runs[1].message.request : null!;
  runs[1].resolve(prepared(request));
  await settled();
  expect(states.map((s) => s.status)).toEqual(["loading", "retrying", "ready"]);
  expect(
    previews.state.status === "ready" && previews.state.preview.request.map_source.request.seed,
  ).toBe("2");
});

test("a preview that runs past its deadline fails", async () => {
  const { runs, previews, settled, expire } = harness();
  previews.show(candidate("a"));
  expire();
  await settled();
  expect(previews.state.status).toBe("failed");
  expect(runs[0].closed).toBe(true);
});

test("disposing the previews closes the running worker and reports nothing more", async () => {
  const { states, runs, previews, settled } = harness();
  previews.show(candidate("a"));
  previews.dispose();
  expect(runs[0].closed).toBe(true);
  runs[0].reject(refusal());
  await settled();
  expect(states.map((s) => s.status)).toEqual(["loading"]);
});
