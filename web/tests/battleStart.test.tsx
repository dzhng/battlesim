import { renderInRouter as render } from "./support/router";
// @vitest-environment jsdom
// Starting a battle from the menu: the seed stays exact text from the address
// to the address and back, the address says what it gets wrong, and a
// preparation that is cancelled or replaced never delivers its battle.
import { createElement } from "react";
import { cleanup, fireEvent } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { askedBattle, battleHref, preparedBattleHref } from "@apps/battle-lab/src/battleLinks";
import { MainMenu } from "@apps/battle-lab/src/MainMenu";
import { admitBattle } from "../src/battle/prepare/admission";
import { PreparationFailed, prepareBattle } from "../src/battle/prepare/client";
import type {
  PrepareBattleRequest,
  PreparedBattle,
  PrepareMessage,
  PrepareReply,
} from "../src/battle/prepare/protocol";
import { canonicalSeed, newSeed } from "../src/maps/source";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

const U64_MAX = "18446744073709551615";
/** 2^53 + 1: the first whole number a JS number cannot hold. */
const ABOVE_NUMBER = "9007199254740993";

test("a seed is canonical u64 decimal text, exact above what a number holds", () => {
  expect(canonicalSeed(ABOVE_NUMBER)).toBe(ABOVE_NUMBER);
  expect(canonicalSeed(U64_MAX)).toBe(U64_MAX);
  expect(canonicalSeed("0")).toBe("0");
  // Typed forms of one seed are that seed.
  expect(canonicalSeed(" 0042 ")).toBe("42");
  for (const not of ["18446744073709551616", "", "-1", "1.5", "1e3", "0x10", "12 34", "seed"])
    expect(canonicalSeed(not), not).toBeNull();
  for (let draw = 0; draw < 50; draw++) {
    const seed = newSeed();
    expect(canonicalSeed(seed)).toBe(seed);
  }
});

test("a battle address round-trips the menu's choice, and names a parameter it gets wrong", () => {
  const map = { type: "metro", size: "large", seed: ABOVE_NUMBER } as const;
  const asked = askedBattle(new URL(battleHref(map), "http://game").search);
  expect(asked).toMatchObject({ kind: "generated", map, recipe: "assault" });
  // A chosen region rides along; without one the address names none.
  const paris = { ...map, region: "paris" };
  expect(askedBattle(new URL(battleHref(paris), "http://game").search)).toMatchObject({
    map: paris,
  });
  expect("map" in asked && asked.map).not.toHaveProperty("region");
  // The address Play publishes after admission names the region it asked
  // for, so the link makes the same map again.
  const admitted = preparedBattleHref({
    map_source: {
      kind: "generated",
      request: {
        ...paris,
        generator_version: "test",
        preset_revision: "test",
        template_catalog_hash: "test",
        limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
      },
    },
    recipe_id: "assault",
    encounter_seed: "1",
    battle_seed: 1,
  });
  expect(askedBattle(new URL(admitted, "http://game").search)).toMatchObject({ map: paris });
  expect(askedBattle("?map=village&recipe=lean")).toHaveProperty("error");
  expect(askedBattle("?replay=saved")).toEqual({ kind: "replay" });
  // A wrong parameter is refused by name: nothing is guessed in its place.
  for (const [search, names] of [
    ["?type=huge&size=small&seed=1", /^type /],
    ["?type=open&size=tiny&seed=1", /^size /],
    ["?type=open&size=small&seed=18446744073709551616", /^seed /],
    ["?seed=1&encounter=1.5", /^encounter /],
    ["?seed=1&battle=9007199254740993", /^battle /],
    ["?seed=1&battle=-1", /^battle /],
    ["?map=../village&recipe=lean", /^map /],
    ["?map=village", /^map /],
    ["?seed=1&region=atlantis", /^region /],
    ["?play=1&region=", /^region /],
  ] as const) {
    const refused = askedBattle(search);
    expect("error" in refused && refused.error, search).toMatch(names);
  }
});

test("ordinary menu Play asks for the chosen type and size without pinning a seed", () => {
  const menu = render(createElement(MainMenu));
  fireEvent.click(menu.getByRole("button", { name: "Skirmish" }));
  const deploy = () => menu.getByTestId("menu-deploy").getAttribute("href")!;
  expect(deploy()).toBe("/battle?play=1&type=mixed&size=small");
  expect(askedBattle(new URL(deploy(), "http://game").search)).toMatchObject({
    kind: "play",
    map: { type: "mixed", size: "small" },
  });
  expect(menu.container.textContent).not.toMatch(/seed/i);

  // The preference changes before admission chooses a concrete battle.
  fireEvent.click(menu.getByRole("radio", { name: "metro" }));
  fireEvent.click(menu.getByRole("radio", { name: "xl" }));
  expect(deploy()).toBe("/battle?play=1&type=metro&size=xl");
  expect(menu.getByRole("radio", { name: "metro" }).getAttribute("aria-checked")).toBe("true");
  expect(menu.getByRole("radio", { name: "mixed" }).getAttribute("aria-checked")).toBe("false");
});

test("the menu leaves the region to the seed unless the player picks one of the presets' regions", () => {
  const menu = render(createElement(MainMenu));
  fireEvent.click(menu.getByRole("button", { name: "Skirmish" }));
  const deploy = () => menu.getByTestId("menu-deploy").getAttribute("href")!;
  expect(menu.getByRole("radio", { name: "random" }).getAttribute("aria-checked")).toBe("true");
  for (const region of ["china", "new york", "paris"])
    expect(menu.getByRole("radio", { name: region })).toBeTruthy();

  fireEvent.click(menu.getByRole("radio", { name: "new york" }));
  expect(deploy()).toBe("/battle?play=1&type=mixed&size=small&region=new_york");
  expect(askedBattle(new URL(deploy(), "http://game").search)).toMatchObject({
    kind: "play",
    map: { type: "mixed", size: "small", region: "new_york" },
  });
  fireEvent.click(menu.getByRole("radio", { name: "random" }));
  expect(deploy()).toBe("/battle?play=1&type=mixed&size=small");
});

test("the menu opens on the battle a cancelled or refused request asked for", () => {
  window.history.replaceState(null, "", `/?type=open&size=medium&seed=${ABOVE_NUMBER}`);
  const menu = render(createElement(MainMenu));
  expect(menu.getByTestId("menu-deploy").getAttribute("href")).toBe(
    `/battle?type=open&size=medium&seed=${ABOVE_NUMBER}`,
  );
  cleanup();
  window.history.replaceState(null, "", `/?type=open&size=medium&region=paris`);
  const again = render(createElement(MainMenu));
  expect(again.getByTestId("menu-deploy").getAttribute("href")).toBe(
    `/battle?play=1&type=open&size=medium&region=paris`,
  );
});

test("the menu offers fresh skirmishes without prebuilt battlefields", () => {
  const menu = render(createElement(MainMenu));
  expect(menu.queryByRole("button", { name: "Battlefields" })).toBeNull();
  fireEvent.click(menu.getByRole("button", { name: "Skirmish" }));
  expect(menu.getByRole("link", { name: "Deploy" }).getAttribute("href")).toBe(
    "/battle?play=1&type=mixed&size=small",
  );
});

/** A preparation worker that answers when the test says so. */
class FakeWorker {
  static all: FakeWorker[] = [];
  onmessage: ((event: MessageEvent<PrepareReply>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  posted: PrepareMessage[] = [];
  terminated = false;
  constructor() {
    FakeWorker.all.push(this);
  }
  postMessage(message: PrepareMessage) {
    this.posted.push(message);
  }
  terminate() {
    this.terminated = true;
  }
  /** Deliver a reply, as a message already queued for the page would be
   *  even after the worker is closed. */
  answer(reply: PrepareReply) {
    this.onmessage?.({ data: reply } as MessageEvent<PrepareReply>);
  }
}

const message = (seed: number): PrepareMessage => ({
  type: "prepare",
  request: { battle_seed: seed } as PrepareBattleRequest,
  documents: { rules: "{}", presets: "{}", templates: "[]", recipes: "{}" },
});
const battle = (name: string) => ({ scenario: name }) as PreparedBattle;
/** What `promise` has settled to by now, or "pending". */
async function settled(promise: Promise<unknown>): Promise<unknown> {
  const pending = Symbol("pending");
  const state = await Promise.race([
    promise.then(
      (value) => value,
      (error: unknown) => error,
    ),
    new Promise((resolve) => setTimeout(() => resolve(pending), 0)),
  ]);
  return state === pending ? "pending" : state;
}

test("a cancelled or replaced preparation never delivers its battle; the request that stands does", async () => {
  FakeWorker.all = [];
  vi.stubGlobal("Worker", FakeWorker);
  const stages: string[] = [];
  const first = prepareBattle(message(1), (stage) => stages.push(`first ${stage}`));
  // The player asks for another battle: the first is cancelled, then replaced.
  first.cancel();
  const second = prepareBattle(message(2), (stage) => stages.push(`second ${stage}`));
  const [stale, live] = FakeWorker.all;
  expect(stale.terminated).toBe(true);
  expect(live.posted[0].request.battle_seed).toBe(2);

  // The first worker's answer was already on its way.
  stale.answer({ type: "stage", stage: "encounter" });
  stale.answer({ type: "prepared", battle: battle("first") });
  expect(await settled(first.battle)).toBe("pending");

  live.answer({ type: "stage", stage: "map" });
  live.answer({ type: "prepared", battle: battle("second") });
  expect(await settled(second.battle)).toMatchObject(battle("second"));
  expect(stages).toEqual(["second map"]);
  expect(await settled(first.battle)).toBe("pending");
  // The prepared worker stays owned by this battle until it is abandoned.
  expect(live.terminated).toBe(false);
  second.cancel();
  expect(live.terminated).toBe(true);
});

test("a refusal carries the stage that refused and its diagnostics, and no battle", async () => {
  FakeWorker.all = [];
  vi.stubGlobal("Worker", FakeWorker);
  const asked = prepareBattle(message(1), () => {});
  const diagnostics = [
    { code: "no_objective", feature: null, location: "$", message: "no settlement can be held" },
  ];
  FakeWorker.all[0].answer({ type: "refused", stage: "encounter", diagnostics });
  const failure = (await settled(asked.battle)) as PreparationFailed;
  expect(failure).toBeInstanceOf(PreparationFailed);
  expect(failure.stage).toBe("encounter");
  expect(failure.diagnostics).toEqual(diagnostics);
});

test("ordinary admission closes refused workers, and exact winner identity keeps its authority for play", async () => {
  FakeWorker.all = [];
  vi.stubGlobal("Worker", FakeWorker);
  const request = (seed: string): PrepareBattleRequest => ({
    map_source: {
      kind: "generated",
      request: {
        type: "metro",
        size: "large",
        seed,
        generator_version: "test",
        preset_revision: "test",
        template_catalog_hash: "test",
        limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
      },
    },
    recipe_id: "assault",
    encounter_seed: ABOVE_NUMBER,
    battle_seed: 7,
  });
  const seeds = ["11", ABOVE_NUMBER];
  const admission = admitBattle(
    {
      candidate: request,
      documents: message(1).documents,
      policy: { max_generated_attempts: 2, generated_deadline_ms: 8000 },
    },
    () => {},
    { seed: () => seeds.shift()! },
  );
  const first = FakeWorker.all[0];
  first.answer({
    type: "refused",
    stage: "map",
    diagnostics: [{ code: "generation_failed", feature: null, location: "$", message: "no map" }],
  });
  await Promise.resolve();
  await Promise.resolve();
  const winner = FakeWorker.all[1];
  const wire = {
    scenario: "admitted",
    report: { request: request(ABOVE_NUMBER) },
  } as PreparedBattle;
  winner.answer({ type: "prepared", battle: wire });
  const admitted = await admission.battle;
  const address = preparedBattleHref(admitted.report.request);
  expect(askedBattle(new URL(address, "http://game").search)).toEqual({
    kind: "generated",
    map: { type: "metro", size: "large", seed: ABOVE_NUMBER },
    recipe: "assault",
    encounterSeed: ABOVE_NUMBER,
    battleSeed: 7,
  });
  expect(first.terminated).toBe(true);
  expect(winner.terminated).toBe(false);
  const observed: unknown[] = [];
  const channel = admitted.connect(
    (reply) => observed.push(reply),
    () => {},
  );
  channel.send({ type: "pause" });
  winner.answer({ type: "status", status: "paused", slow: false } as unknown as PrepareReply);
  expect(observed).toEqual([{ type: "status", status: "paused", slow: false }]);
  admission.cancel();
  expect(winner.terminated).toBe(true);
});

test("skirmish controls and Deploy stand alone without explanatory subtitles", () => {
  const menu = render(createElement(MainMenu));
  fireEvent.click(menu.getByRole("button", { name: "Skirmish" }));
  expect(menu.container.textContent).not.toMatch(/fields and woods|defended town|as blue/i);
  expect(menu.getByRole("link", { name: "Deploy" }).textContent).toBe("Deploy");
});

test("repeated preparation diagnostics appear once in loading details", async () => {
  const { LoadingScreen } = await import("@apps/battle-lab/src/LoadingScreen");
  const loading = render(
    createElement(LoadingScreen, {
      title: "Deploying",
      stages: [],
      current: "",
      failure: {
        message: "Cannot prepare",
        details: ["cannot fit", "cannot fit", "bad input"],
      },
    }),
  );
  fireEvent.click(loading.getByRole("button", { name: "Details" }));
  expect(loading.getAllByRole("listitem").map((li) => li.textContent)).toEqual([
    "cannot fit",
    "bad input",
  ]);
});
