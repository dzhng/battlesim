import { expect, test } from "vitest";
import { nodeCatalogSet } from "@web/battle/catalog/node.ts";
import { rangeRuler } from "@web/battle/present/rangeRuler";

test("a bomb marks no reach on the range ruler: it lands where the jet releases it", async () => {
  const { units, weapons, rules } = await nodeCatalogSet("game");
  // Any shipped aircraft that carries an unpowered round (a bomb).
  const bomber = units.ids.find((id) =>
    units
      .type(id)
      .mounts.some((m) =>
        m.weapons.some((w) => (weapons[w] as { delivery?: string })?.delivery === "unpowered"),
      ),
  );
  expect(bomber, "a shipped unit carries a bomb").toBeDefined();
  const ruler = rangeRuler(
    { id: 1, kind: bomber!, position: [0, 0, 250] },
    [500, 0, 0],
    { weapons, physics: rules.physics },
    units,
  );
  expect(ruler.marks.length).toBeGreaterThan(0);
  expect(ruler.marks.every((m) => m.range_m > 0)).toBe(true);
  const bombNames = units
    .type(bomber!)
    .mounts.flatMap((m) =>
      m.weapons
        .filter((w) => (weapons[w] as { delivery?: string })?.delivery === "unpowered")
        .map((w) => weapons[w].name),
    );
  expect(ruler.marks.flatMap((m) => m.names).some((n) => bombNames.includes(n))).toBe(false);
});
