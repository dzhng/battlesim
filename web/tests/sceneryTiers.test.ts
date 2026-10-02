// @vitest-environment node
// Scenery through the static chunk owner: what is drawn into the view, and
// what is drawn into the sun's cascades. A caster is drawn only where its
// shadow can land in view, so a forest's cost follows the view, not the map.
import { expect, test } from "vitest";
import {
  chunkTier,
  createPlacedInstances,
  INSTANCE_FLOATS,
  sceneryChunks,
  tierFor,
  TIER_COUNT,
  type TierView,
} from "@packages/battle-renderer/src/scenery/lod";
import {
  createStagedLevels,
  selectChunks,
  stageNear,
  type StagedLevels,
  type StaticChunks,
} from "@packages/battle-renderer/src/frame/staticChunks";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";

const CHUNK_M = 128;
const TREE_M = 12;

interface Forest {
  chunks: StaticChunks;
  staged: StagedLevels;
  casts: boolean;
}

/** One 12 m tree at each of `at`, each in a chunk of its own. */
function forest(at: [number, number][], casts: boolean): Forest {
  const placed = createPlacedInstances(at.length);
  at.forEach(([x, y], i) => {
    placed.records.set([x, y, 0, 0, 1, 1, 1, 6, 1, 1, 1, 0], i * INSTANCE_FLOATS);
    placed.heights[i] = TREE_M;
    placed.reaches[i] = 4;
  });
  const chunks = sceneryChunks(placed, 1, CHUNK_M);
  return { chunks, staged: createStagedLevels(chunks), casts };
}

/** Choose what `view` draws of `pop`, as the scenery layer does. */
function selectTiers(pop: Forest, view: TierView) {
  selectChunks(pop.chunks, view, chunkTier, pop.casts ? view.shadow : null);
  stageNear(pop.chunks, pop.staged, view, tierFor);
}

/** A camera 300 m south of (1000, 1000) looking north along +Y and down at it;
 *  the sun stands in the west, so shadows fall east (+X), 3 m a metre of height. */
function view(reach = 2600): TierView {
  return {
    ...setDetailView(
      createDetailView(),
      {
        target: [1000, 1000, 0],
        distance: 300,
        pitch: 0.85,
        yaw: Math.PI / 2,
        fovY: 0.8,
        aspect: 16 / 9,
        near: 1,
      },
      1080,
    ),
    lodPx: [150, 60, 24],
    shadow: { fall: [3, 0], reach },
  };
}

/** Instances in a population's merged `[first, count]` ranges. */
const ranged = (ranges: number[]) => ranges.reduce((n, v, k) => (k % 2 ? n + v : n), 0);
const staged = (pop: Forest) => pop.staged.counts[0].reduce((a, b) => a + b, 0);
const drawn = (pop: Forest) => ranged(pop.chunks.ranges[0][TIER_COUNT - 1]) + staged(pop);

test("a tree outside the view is not drawn into it", () => {
  // In view at the target, and 3 km to either side of it.
  const pop = forest(
    [
      [1000, 1000],
      [4000, 1000],
      [-2000, 1000],
    ],
    true,
  );
  selectTiers(pop, view());
  expect(drawn(pop)).toBe(1);
});

test("a tree off screen casts only when its shadow can land in view", () => {
  // Left of the view by 20 m to 60 m at the target's depth (the view is about
  // 450 m wide there): the first tree's 36 m shadow reaches in, the tree east
  // of the view casts away from it, and a tree 3 km off casts nowhere near.
  const seen: [number, number] = [1000, 1000];
  const west: [number, number] = [1000 - 240, 1000];
  const east: [number, number] = [1000 + 240, 1000];
  const far: [number, number] = [1000 - 3000, 1000];
  for (const [at, casts] of [
    [west, true],
    [east, false],
    [far, false],
  ] as const) {
    const pop = forest([seen, at], true);
    selectTiers(pop, view());
    expect(drawn(pop)).toBe(1);
    // The seen tree casts too.
    expect(ranged(pop.chunks.cast[0]) + staged(pop)).toBe(casts ? 2 : 1);
  }
});

test("nothing off screen casts past the shadow's reach, and a population that casts nothing lists none", () => {
  // The camera is 300 m from the tree: past a 100 m reach nothing is shadowed.
  const beyond = forest([[1000 - 240, 1000]], true);
  selectTiers(beyond, view(100));
  expect(ranged(beyond.chunks.cast[0])).toBe(0);

  const backdrop = forest([[1000 - 240, 1000]], false);
  selectTiers(backdrop, view());
  expect(ranged(backdrop.chunks.cast[0])).toBe(0);
});
