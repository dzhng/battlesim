// @vitest-environment node
// The attack-ground cursor's circle is the simulation's strike footprint: the
// area the selected units' strikes there can damage, salvo and blast included.
import { beforeAll, expect, test } from "vitest";
import { loadWasm } from "@web/battle/sim/module";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { attackGroundSpread } from "@apps/battle-lab/src/pointerPaint";

let set: SessionCatalog;
beforeAll(async () => {
  await loadWasm();
  set = await resolveCatalogSet("test");
});

/** A roster kind whose rows include `row`, and none of `without`. */
function carrying(row: string, without: string[] = []) {
  const kind = set.units.ids.find((id) => {
    const rows = set.units.type(id).mounts.flatMap((m) => m.weapons);
    return rows.includes(row) && !without.some((w) => rows.includes(w));
  });
  if (!kind) throw new Error(`no kind carries ${row}`);
  return kind;
}

/** The circle round an aim at the origin for one unit of `kind` at `at`. */
function circle(kind: string, at: [number, number, number]) {
  const unit = specimenUnit(set, kind, { id: 1, position: at });
  const spread = attackGroundSpread([0, 0], [unit], new Map(), JSON.stringify(set.rules));
  return spread!.radius_m;
}

test("a cluster-bomb jet's circle spans its whole salvo, far wider than an iron bomb's", () => {
  const cluster = circle(carrying("air_bomb_cluster"), [3000, 0, 250]);
  const iron = circle(carrying("air_bomb", ["air_bomb_cluster"]), [3000, 0, 250]);
  expect(cluster).toBeGreaterThan(iron * 2);
  // Where the jet is now does not matter: it bombs from its run.
  expect(circle(carrying("air_bomb_cluster"), [400, 0, 250])).toBe(cluster);
});

test("a ground weapon's circle takes in its blast, even point blank", () => {
  const kind = carrying("artillery_mortar");
  const blast = (set.rules.weapons.artillery_mortar as unknown as { blast_radius_m: number })
    .blast_radius_m;
  expect(circle(kind, [10, 0, 0])).toBeGreaterThanOrEqual(blast);
  // Farther fire strays farther.
  expect(circle(kind, [1500, 0, 0])).toBeGreaterThan(circle(kind, [300, 0, 0]));
});
