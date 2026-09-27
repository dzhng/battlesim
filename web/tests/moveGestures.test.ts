// @vitest-environment node
import { expect, test } from "vitest";
import { MoveGestures } from "../src/battle/input/moveGestures";

test("a single right-click is an ordinary shortest move with a fresh token", () => {
  const g = new MoveGestures();
  const a = g.rightClick({ x: 10, y: 10, time: 0 }, [1, 2], [50, 60]);
  const b = g.rightClick({ x: 300, y: 10, time: 1000 }, [1], [80, 60]);
  expect(a).toEqual({ kind: "move", units: [1, 2], gesture: 1, goal: [50, 60], route: "shortest", direction: "forward" });
  expect(b.kind === "move" && b.gesture).toBe(2);
});

test("a second click within 350 ms and 6 px upgrades that token, adding no waypoint", () => {
  const g = new MoveGestures();
  g.rightClick({ x: 10, y: 10, time: 0 }, [1], [50, 60]);
  expect(g.rightClick({ x: 14, y: 13, time: 340 }, [1], [51, 61])).toEqual({
    kind: "upgrade_move",
    gesture: 1,
    route: "fastest",
  });
  // A third click starts a new gesture rather than upgrading again.
  expect(g.rightClick({ x: 14, y: 13, time: 400 }, [1], [51, 61]).kind).toBe("move");
});

test("slow or distant second clicks are separate moves", () => {
  const g = new MoveGestures();
  g.rightClick({ x: 10, y: 10, time: 0 }, [1], [50, 60]);
  expect(g.rightClick({ x: 10, y: 10, time: 360 }, [1], [50, 60]).kind).toBe("move");
  g.rightClick({ x: 10, y: 10, time: 1000 }, [1], [50, 60]);
  expect(g.rightClick({ x: 20, y: 10, time: 1100 }, [1], [50, 60]).kind).toBe("move");
});
