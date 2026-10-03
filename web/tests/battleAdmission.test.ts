// @vitest-environment node
import { expect, test, vi } from "vitest";
import { admitBattle } from "../src/battle/prepare/admission";
import { PreparationFailed, type PreparedSession } from "../src/battle/prepare/client";
import type { PrepareBattleRequest, PreparationMessage } from "../src/battle/prepare/protocol";

const candidate = (seed: string): PrepareBattleRequest => ({
  map_source: {
    kind: "generated",
    request: {
      type: "mixed",
      size: "small",
      seed,
      generator_version: "test",
      preset_revision: "test",
      template_catalog_hash: "test",
      limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
    },
  },
  recipe_id: "assault",
  encounter_seed: "3",
  battle_seed: 7,
});
const fallback: PrepareBattleRequest = {
  ...candidate("1"),
  map_source: { kind: "catalogue", id: "market-town" },
};
const documents = { rules: "{}", presets: "{}", templates: "[]", recipes: "{}" };
const policy = { max_generated_attempts: 2, generated_deadline_ms: 8000 };
const session = (request: PrepareBattleRequest): PreparedSession => ({
  scenario: "winner",
  report: { request } as PreparedSession["report"],
  connect: () => ({ send: () => {}, close: () => {} }),
});

test("ordinary Play admits a fresh candidate after refusal and retains its worker authority", async () => {
  const requests: PrepareBattleRequest[] = [];
  const closed: string[] = [];
  const winner = session(candidate("22"));
  const seeds = ["11", "22"];
  const admission = admitBattle({ candidate, documents, fallback, policy }, () => {}, {
    seed: () => seeds.shift()!,
    prepare: (message: PreparationMessage) => {
      if (message.type !== "prepare") throw new Error("unexpected replay");
      requests.push(message.request);
      const source = message.request.map_source;
      const seed = source.kind === "generated" ? source.request.seed : source.id;
      return {
        battle:
          seed === "11"
            ? Promise.reject(
                new PreparationFailed(
                  "refused",
                  [
                    {
                      code: "generation_failed",
                      feature: null,
                      location: "$",
                      message: "cannot fit",
                    },
                  ],
                  "map",
                ),
              )
            : Promise.resolve(winner),
        cancel: () => {
          closed.push(seed);
        },
      };
    },
  });
  expect(await admission.battle).toBe(winner);
  expect(
    requests.map((r) => r.map_source.kind === "generated" && r.map_source.request.seed),
  ).toEqual(["11", "22"]);
  expect(closed).toEqual(["11"]);
  expect(admission.attempts.map((a) => a.outcome)).toEqual(["refused", "admitted"]);
  admission.cancel();
  expect(closed).toEqual(["11", "22"]);
});

test("encounter exhaustion changes the candidate; invalid catalogue inputs go straight to saved fallback", async () => {
  for (const [stage, code, wanted] of [
    ["encounter", "no_open_approach", ["11", "22"]],
    ["map", "invalid_catalogue", ["11", "market-town"]],
  ] as const) {
    const seen: string[] = [];
    const seeds = ["11", "22"];
    const admission = admitBattle({ candidate, documents, fallback, policy }, () => {}, {
      seed: () => seeds.shift()!,
      prepare: (message) => {
        if (message.type !== "prepare") throw new Error("unexpected replay");
        const source = message.request.map_source;
        const key = source.kind === "generated" ? source.request.seed : source.id;
        seen.push(key);
        return {
          battle:
            seen.length === 1
              ? Promise.reject(
                  new PreparationFailed(
                    "refused",
                    [{ code, feature: null, location: "$", message: "refused" }],
                    stage,
                  ),
                )
              : Promise.resolve(session(message.request)),
          cancel: () => {},
        };
      },
    });
    const battle = await admission.battle;
    expect(seen).toEqual(wanted);
    expect(battle.report.request.map_source).toEqual(
      wanted[1] === "market-town" ? fallback.map_source : candidate("22").map_source,
    );
    admission.cancel();
  }
});

test("the total generated deadline terminates a worker waiting in encounter placement and admits saved fallback", async () => {
  let tick!: () => void;
  const closed: string[] = [];
  let late!: (value: PreparedSession) => void;
  const admission = admitBattle({ candidate, documents, fallback, policy }, () => {}, {
    seed: () => "11",
    schedule: (run) => {
      tick = run;
      return () => {};
    },
    prepare: (message) => {
      if (message.type !== "prepare") throw new Error("unexpected replay");
      const source = message.request.map_source;
      return source.kind === "generated"
        ? {
            battle: new Promise((resolve) => {
              late = resolve;
            }),
            cancel: () => {
              closed.push(source.request.seed);
            },
          }
        : {
            battle: Promise.resolve(session(message.request)),
            cancel: () => {
              closed.push(source.id);
            },
          };
    },
  });
  tick();
  const admitted = await admission.battle;
  expect(closed).toEqual(["11"]);
  expect(admitted.report.request).toEqual(fallback);
  late(session(candidate("11")));
  expect(await admission.battle).toBe(admitted);
  expect(admission.attempts.map((a) => a.outcome)).toEqual(["timeout", "admitted"]);
  admission.cancel();
  expect(closed).toEqual(["11", "market-town"]);
});

test("exhausted distinct candidates fall back even when the seed source repeats a full u64 value", async () => {
  const seen: string[] = [];
  const closed: string[] = [];
  const admission = admitBattle({ candidate, documents, fallback, policy }, () => {}, {
    seed: () => "18446744073709551615",
    prepare: (message) => {
      if (message.type !== "prepare") throw new Error("unexpected replay");
      const source = message.request.map_source;
      const key = source.kind === "generated" ? source.request.seed : source.id;
      seen.push(key);
      return {
        battle:
          source.kind === "generated"
            ? Promise.reject(
                new PreparationFailed(
                  "no map",
                  [{ code: "generation_failed", feature: null, location: "$", message: "no map" }],
                  "map",
                ),
              )
            : Promise.resolve(session(message.request)),
        cancel: () => {
          closed.push(key);
        },
      };
    },
  });
  expect((await admission.battle).report.request).toEqual(fallback);
  expect(seen).toEqual(["18446744073709551615", "0", "market-town"]);
  expect(closed).toEqual(["18446744073709551615", "0"]);
  admission.cancel();
});

test("leaving cancels the active candidate and suppresses queued stages, replies and fallback", async () => {
  let answer!: (battle: PreparedSession) => void;
  let emitStage!: (stage: "map" | "encounter") => void;
  let jobs = 0;
  let closed = false;
  let cleared = false;
  const stages: string[] = [];
  const admission = admitBattle(
    { candidate, documents, fallback, policy },
    (stage) => stages.push(stage),
    {
      seed: () => "11",
      schedule: () => () => {
        cleared = true;
      },
      prepare: (_message, stage) => {
        jobs++;
        emitStage = stage;
        return {
          battle: new Promise((resolve) => {
            answer = resolve;
          }),
          cancel: () => {
            closed = true;
          },
        };
      },
    },
  );
  let settled = false;
  void admission.battle.then(() => {
    settled = true;
  });
  admission.cancel();
  emitStage("encounter");
  answer(session(candidate("11")));
  await Promise.resolve();
  await Promise.resolve();
  expect({ jobs, closed, cleared, settled, stages }).toEqual({
    jobs: 1,
    closed: true,
    cleared: true,
    settled: false,
    stages: [],
  });
});

test("loading identifies the current candidate and then the saved fallback", async () => {
  const loading: PrepareBattleRequest[] = [];
  const admission = admitBattle(
    { candidate, documents, fallback, policy, onRequest: (request) => loading.push(request) },
    () => {},
    {
      seed: () => "11",
      prepare: (message) => {
        if (message.type !== "prepare") throw new Error("unexpected replay");
        return {
          battle:
            message.request.map_source.kind === "generated"
              ? Promise.reject(
                  new PreparationFailed(
                    "bad presets",
                    [
                      {
                        code: "invalid_presets",
                        feature: null,
                        location: "$",
                        message: "bad presets",
                      },
                    ],
                    "map",
                  ),
                )
              : Promise.resolve(session(message.request)),
          cancel: () => {},
        };
      },
    },
  );
  await admission.battle;
  expect(loading).toEqual([candidate("11"), fallback]);
  admission.cancel();
});

test("the second candidate inherits the remaining total deadline instead of another full budget", async () => {
  vi.useFakeTimers();
  try {
    const closed: string[] = [];
    const seeds = ["11", "22"];
    let refuse!: (error: unknown) => void;
    const admission = admitBattle({ candidate, documents, fallback, policy }, () => {}, {
      seed: () => seeds.shift()!,
      prepare: (message) => {
        if (message.type !== "prepare") throw new Error("unexpected replay");
        const source = message.request.map_source;
        return source.kind === "generated"
          ? {
              battle: new Promise((_resolve, reject) => {
                refuse = reject;
              }),
              cancel: () => {
                closed.push(source.request.seed);
              },
            }
          : {
              battle: Promise.resolve(session(message.request)),
              cancel: () => {
                closed.push(source.id);
              },
            };
      },
    });
    await vi.advanceTimersByTimeAsync(6000);
    refuse(
      new PreparationFailed(
        "cannot place",
        [{ code: "no_objective", feature: null, location: "$", message: "cannot place" }],
        "encounter",
      ),
    );
    await vi.advanceTimersByTimeAsync(0);
    expect(closed).toEqual(["11"]);
    await vi.advanceTimersByTimeAsync(1999);
    expect(closed).toEqual(["11"]);
    await vi.advanceTimersByTimeAsync(1);
    expect((await admission.battle).report.request).toEqual(fallback);
    expect(closed).toEqual(["11", "22"]);
    admission.cancel();
  } finally {
    vi.useRealTimers();
  }
});

test("a worker that cannot start records the runtime fault and resolves fallback once", async () => {
  const admission = admitBattle({ candidate, documents, fallback, policy }, () => {}, {
    seed: () => "11",
    prepare: (message) => {
      if (message.type !== "prepare") throw new Error("unexpected replay");
      if (message.request.map_source.kind === "generated") throw new Error("worker startup failed");
      return { battle: Promise.resolve(session(message.request)), cancel: () => {} };
    },
  });
  expect((await admission.battle).report.request).toEqual(fallback);
  expect(admission.attempts[0]).toMatchObject({
    outcome: "error",
    failure: { message: "worker startup failed", stage: null, diagnostics: [] },
  });
  admission.cancel();
});
