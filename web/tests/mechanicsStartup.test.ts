import { afterEach, beforeEach, expect, test, vi } from "vitest";
import shippedGame from "@fixtures/game.json";
import shippedCatalog from "@fixtures/catalog.json";
const baselineGame = structuredClone(shippedGame);
const baselineCatalog = structuredClone(shippedCatalog);
beforeEach(() => vi.resetModules());
afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

test("page startup captures one accepted rules and presentation generation", async () => {
  const { startWithMechanics } = await import("@web/mechanicsStartup");
  const game = structuredClone(baselineGame);
  const catalog = structuredClone(baselineCatalog);
  game.weapons.grenade.speed_mps = 123;
  catalog.weapons.grenade.speed_mps = 123;
  const tank = catalog.units.find((unit) => unit.id === "tank")!;
  tank.name = "Snapshot tank";
  catalog.documents[0].units.tank.name = "Snapshot tank";
  const fetcher = vi.fn(
    async () =>
      new Response(
        JSON.stringify({
          revision: "accepted",
          documents: [{ path: "fixtures/game.json", value: game }],
          catalog,
        }),
      ),
  );
  await startWithMechanics(
    true,
    async () => {
      const { UNITS, WEAPONS } = await import("@packages/scene-assets/src/shippedUnits");
      const { GAME_RULES } = await import("@apps/battle-lab/src/scenarios");
      expect(GAME_RULES.weapons.grenade.speed_mps).toBe(123);
      expect(WEAPONS.grenade.speed_mps).toBe(123);
      expect(UNITS.type("tank").name).toBe("Snapshot tank");
      expect(JSON.parse(JSON.stringify(GAME_RULES)).catalog[0].units.tank.name).toBe(
        "Snapshot tank",
      );
      // Saving elsewhere changes the next response, never this page's values.
      game.weapons.grenade.speed_mps = 456;
      catalog.weapons.grenade.speed_mps = 456;
      expect(GAME_RULES.weapons.grenade.speed_mps).toBe(123);
      expect(WEAPONS.grenade.speed_mps).toBe(123);
    },
    fetcher,
  );
  expect(fetcher).toHaveBeenCalledTimes(1);
  // A fresh page imports fresh consumers after fetching the newly accepted pair.
  vi.resetModules();
  const nextPage = await import("@web/mechanicsStartup");
  await nextPage.startWithMechanics(
    true,
    async () => {
      const { WEAPONS } = await import("@packages/scene-assets/src/shippedUnits");
      const { GAME_RULES } = await import("@apps/battle-lab/src/scenarios");
      expect(GAME_RULES.weapons.grenade.speed_mps).toBe(456);
      expect(WEAPONS.grenade.speed_mps).toBe(456);
    },
    fetcher,
  );
});

test("production startup uses shipped rules without the development backend", async () => {
  const { startWithMechanics } = await import("@web/mechanicsStartup");
  const fetcher = vi.fn(async () => {
    throw new Error("no dev backend");
  });
  await startWithMechanics(
    false,
    async () => {
      const { default: game } = await import("@fixtures/game.json");
      const { WEAPONS } = await import("@packages/scene-assets/src/shippedUnits");
      expect(game.weapons.grenade.speed_mps).toBe(baselineGame.weapons.grenade.speed_mps);
      expect(WEAPONS.grenade.speed_mps).toBe(baselineCatalog.weapons.grenade.speed_mps);
    },
    fetcher,
  );
});

test("a rejected mechanics generation refuses startup instead of mixing old values", async () => {
  const { startWithMechanics } = await import("@web/mechanicsStartup");
  let started = false;
  await expect(
    startWithMechanics(
      true,
      () => {
        started = true;
      },
      async () => new Response("invalid combined rules", { status: 422 }),
    ),
  ).rejects.toThrow("Mechanics could not be loaded (422): invalid combined rules");
  expect(started).toBe(false);
});
