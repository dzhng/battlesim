// @vitest-environment node
// The attack-ground cursor's circle is the simulation's strike footprint: the
// area the selected units' strikes there can damage, salvo and blast included,
// shown only where they can strike from where they stand; its reach rings are
// the nearest and farthest they can.
import { beforeAll, expect, test } from "vitest";
import { loadWasm } from "@web/battle/sim/module";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { attackGroundMarks } from "@apps/battle-lab/src/pointerPaint";

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

/** The marks round an aim at the origin for one unit of `kind` at `at`. */
function marks(kind: string, at: [number, number, number]) {
  const unit = specimenUnit(set, kind, { id: 1, position: at });
  return attackGroundMarks([0, 0], [unit], new Map(), JSON.stringify(set.rules))!;
}

/** The circle round an aim at the origin for one unit of `kind` at `at`. */
function circle(kind: string, at: [number, number, number]) {
  return marks(kind, at).radius_m;
}

/** The mortar row's minimum range and ground reach. */
function mortarReach() {
  const row = set.rules.weapons.artillery_mortar as unknown as {
    min_range_m: number;
    targets: { ground: { range_m: number } };
  };
  return { near: row.min_range_m, far: row.targets.ground.range_m };
}

test("a cluster-bomb jet's circle spans its whole salvo, far wider than an iron bomb's", () => {
  const cluster = circle(carrying("air_bomb_cluster"), [3000, 0, 250]);
  const iron = circle(carrying("air_bomb", ["air_bomb_cluster"]), [3000, 0, 250]);
  expect(cluster).toBeGreaterThan(iron! * 2);
  // Where the jet is now does not matter: it bombs from its run.
  expect(circle(carrying("air_bomb_cluster"), [400, 0, 250])).toBe(cluster);
});

test("a ground weapon's circle takes in its blast, even at its nearest", () => {
  const kind = carrying("artillery_mortar");
  const { near, far } = mortarReach();
  const blast = (set.rules.weapons.artillery_mortar as unknown as { blast_radius_m: number })
    .blast_radius_m;
  expect(circle(kind, [near + 1, 0, 0])).toBeGreaterThanOrEqual(blast);
  // Farther fire strays farther.
  expect(circle(kind, [far - 1, 0, 0])!).toBeGreaterThan(circle(kind, [near + 1, 0, 0])!);
});

test("out of its reach a ground unit shows no circle, and its rings show the reach", () => {
  const kind = carrying("artillery_mortar");
  const { near, far } = mortarReach();
  // It never closes on a ground aim: too near or too far, it strikes nothing.
  expect(circle(kind, [near - 1, 0, 0])).toBeNull();
  expect(circle(kind, [far + 1, 0, 0])).toBeNull();
  for (const at of [near - 1, (near + far) / 2, far + 1]) {
    expect(marks(kind, [at, 0, 0]).reach).toEqual({
      from: [at, 0],
      near_m: near,
      far_m: far,
    });
  }
});

test("a jet shows no reach rings: it strikes from its runs", () => {
  expect(marks(carrying("air_bomb_cluster"), [3000, 0, 250]).reach).toBeNull();
});
