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
      profile: "skirmish",
      generator_version: "test",
      preset_revision: "test",
      template_catalog_hash: "test",
      limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
    },
  },
  factions: ["us", "eastern"],
  battle_seed: 7,
});
const documents = { rules: "{}", presets: "{}", templates: "[]" };
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
  const admission = admitBattle({ candidate, documents, policy }, () => {}, {
    seed: () => seeds.shift()!,
    prepare: (message: PreparationMessage) => {
      if (message.type !== "prepare") throw new Error("unexpected replay");
      requests.push(message.request);
      const source = message.request.map_source;
      const seed = source.request.seed;
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

test("a map whose generated skirmish sites the battle refuses gives way to a fresh map", async () => {
  const winner = session(candidate("22"));
  const seeds = ["11", "22"];
  const requested: string[] = [];
  const admission = admitBattle({ candidate, documents, policy }, () => {}, {
    seed: () => seeds.shift()!,
    prepare: (message) => {
      if (message.type !== "prepare") throw new Error("unexpected replay");
      const source = message.request.map_source;
      const seed = source.kind === "generated" ? source.request.seed : "";
      requested.push(seed);
      return {
        battle:
          seed === "11"
            ? Promise.reject(
                new PreparationFailed(
                  "refused",
                  [
                    {
                      code: "invalid_skirmish_sites",
                      feature: "skirmish_sites",
                      location: "$.sites.skirmish",
                      message: "no central objective within the travel-fairness bound",
                    },
                  ],
                  "encounter",
                ),
              )
            : Promise.resolve(winner),
        cancel: () => {},
      };
    },
  });
  expect(await admission.battle).toBe(winner);
  expect(requested).toEqual(["11", "22"]);
});

test("the total generation deadline refuses Play and ignores a late worker reply", async () => {
  let tick!: () => void;
  let late!: (value: PreparedSession) => void;
  let closed = false;
  const admission = admitBattle({ candidate, documents, policy }, () => {}, {
    seed: () => "11",
    schedule: (run) => {
      tick = run;
      return () => {};
    },
    prepare: () => ({
      battle: new Promise((resolve) => {
        late = resolve;
      }),
      cancel: () => {
        closed = true;
      },
    }),
  });
  const refused = expect(admission.battle).rejects.toThrow("timed out");
  tick();
  await refused;
  expect(closed).toBe(true);
  late(session(candidate("11")));
  await expect(admission.battle).rejects.toThrow("timed out");
  expect(admission.attempts.map((a) => a.outcome)).toEqual(["timeout"]);
});

test("leaving cancels the active candidate and suppresses queued stages, replies", async () => {
  let answer!: (battle: PreparedSession) => void;
  let emitStage!: (stage: "map" | "encounter") => void;
  let jobs = 0;
  let closed = false;
  let cleared = false;
  const stages: string[] = [];
  const admission = admitBattle({ candidate, documents, policy }, (stage) => stages.push(stage), {
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
  });
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

test("the second candidate inherits the remaining total deadline instead of another full budget", async () => {
  vi.useFakeTimers();
  try {
    const closed: string[] = [];
    const seeds = ["11", "22"];
    let refuse!: (error: unknown) => void;
    const admission = admitBattle({ candidate, documents, policy }, () => {}, {
      seed: () => seeds.shift()!,
      prepare: (message) => {
        if (message.type !== "prepare") throw new Error("unexpected replay");
        return {
          battle: new Promise((_resolve, reject) => {
            refuse = reject;
          }),
          cancel: () => {
            closed.push(message.request.map_source.request.seed);
          },
        };
      },
    });
    await vi.advanceTimersByTimeAsync(6000);
    refuse(
      new PreparationFailed(
        "cannot place",
        [{ code: "generation_failed", feature: null, location: "$", message: "cannot place" }],
        "map",
      ),
    );
    await vi.advanceTimersByTimeAsync(0);
    expect(closed).toEqual(["11"]);
    await vi.advanceTimersByTimeAsync(1999);
    expect(closed).toEqual(["11"]);
    const refused = expect(admission.battle).rejects.toThrow("timed out");
    await vi.advanceTimersByTimeAsync(1);
    await refused;
    expect(closed).toEqual(["11", "22"]);
    admission.cancel();
  } finally {
    vi.useRealTimers();
  }
});

test("invalid inputs and runtime faults refuse Play without trying another map", async () => {
  for (const fault of [
    new Error("worker startup failed"),
    new PreparationFailed(
      "bad presets",
      [{ code: "invalid_presets", feature: null, location: "$", message: "bad presets" }],
      "map",
    ),
  ]) {
    const requests: PrepareBattleRequest[] = [];
    const admission = admitBattle({ candidate, documents, policy }, () => {}, {
      seed: () => "11",
      prepare: (message) => {
        if (message.type !== "prepare") throw new Error("unexpected replay");
        requests.push(message.request);
        throw fault;
      },
    });
    await expect(admission.battle).rejects.toBe(fault);
    expect(requests).toEqual([candidate("11")]);
    expect(admission.attempts[0].failure?.message).toBe(fault.message);
  }
});

test("exhausted generation refuses Play without preparing a prebuilt map", async () => {
  const requests: PrepareBattleRequest[] = [];
  const loading: PrepareBattleRequest[] = [];
  const refusal = new PreparationFailed(
    "cannot fit",
    [{ code: "generation_failed", feature: null, location: "$", message: "cannot fit" }],
    "map",
  );
  const admission = admitBattle(
    { candidate, documents, policy, onRequest: (r) => loading.push(r) },
    () => {},
    {
      seed: () => "18446744073709551615",
      prepare: (message) => {
        if (message.type !== "prepare") throw new Error("unexpected replay");
        requests.push(message.request);
        return {
          battle:
            message.request.map_source.kind === "generated"
              ? Promise.reject(refusal)
              : Promise.resolve(session(message.request)),
          cancel: () => {},
        };
      },
    },
  );
  await expect(admission.battle).rejects.toBe(refusal);
  expect(loading).toEqual(requests);
  expect(requests.map((r) => r.map_source)).toEqual([
    candidate("18446744073709551615").map_source,
    candidate("0").map_source,
  ]);
});
