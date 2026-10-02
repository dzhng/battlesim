// @vitest-environment jsdom
// Starting a battle from the menu: the seed stays exact text from the field
// to the address and back, the address says what it gets wrong, and a
// preparation that is cancelled or replaced never delivers its battle.
import { createElement } from "react";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { askedBattle, battleHref } from "@apps/battle-lab/src/battleLinks";
import { MainMenu } from "@apps/battle-lab/src/MainMenu";
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
  expect(askedBattle("?map=village&recipe=lean")).toMatchObject({
    kind: "catalogue",
    id: "village",
    recipe: "lean",
  });
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
    ["?map=village", /^recipe /],
  ] as const) {
    const refused = askedBattle(search);
    expect("error" in refused && refused.error, search).toMatch(names);
  }
});

test("the menu deploys the chosen type, size and seed, and nothing while the seed is not one", () => {
  const menu = render(createElement(MainMenu));
  const deploy = () => menu.getByTestId("menu-deploy").getAttribute("href");
  const seed = menu.getByTestId("menu-seed") as HTMLInputElement;
  // It opens on a fresh seed, already a battle.
  expect(canonicalSeed(seed.value)).toBe(seed.value);
  expect(deploy()).toBe(`/battle?type=mixed&size=small&seed=${seed.value}`);

  fireEvent.click(menu.getByRole("radio", { name: "metro" }));
  fireEvent.click(menu.getByRole("radio", { name: "large" }));
  fireEvent.change(seed, { target: { value: ABOVE_NUMBER } });
  expect(deploy()).toBe(`/battle?type=metro&size=large&seed=${ABOVE_NUMBER}`);
  expect(menu.getByRole("radio", { name: "metro" }).getAttribute("aria-checked")).toBe("true");
  expect(menu.getByRole("radio", { name: "mixed" }).getAttribute("aria-checked")).toBe("false");

  // A seed that is not one deploys nothing and says why; no other seed is
  // put in its place.
  fireEvent.change(seed, { target: { value: "18446744073709551616" } });
  expect(deploy()).toBeNull();
  expect(seed.getAttribute("aria-invalid")).toBe("true");
  expect(menu.getByTestId("menu-note").textContent).toMatch(/whole number/);
  expect(seed.value).toBe("18446744073709551616");

  // A new seed replaces it only when asked for.
  fireEvent.click(menu.getByTestId("menu-new-seed"));
  expect(canonicalSeed(seed.value)).toBe(seed.value);
  expect(deploy()).toBe(`/battle?type=metro&size=large&seed=${seed.value}`);
});

test("the menu opens on the battle a cancelled or refused request asked for", () => {
  window.history.replaceState(null, "", `/?type=open&size=medium&seed=${ABOVE_NUMBER}`);
  const menu = render(createElement(MainMenu));
  expect(menu.getByTestId("menu-deploy").getAttribute("href")).toBe(
    `/battle?type=open&size=medium&seed=${ABOVE_NUMBER}`,
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
  expect(await settled(second.battle)).toEqual(battle("second"));
  expect(stages).toEqual(["second map"]);
  expect(await settled(first.battle)).toBe("pending");
  // The worker is closed once it has answered.
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
