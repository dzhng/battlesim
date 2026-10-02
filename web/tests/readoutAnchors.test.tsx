// @vitest-environment jsdom
import { createRef } from "react";
import { render, cleanup } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import game from "@fixtures/game.json";
import { ReadoutLayer, type ReadoutLayerHandle } from "../src/battle/present/readouts";
import type { ContactView } from "../src/battle/sim/observation";
import type { PanelRules } from "../src/battle/present/panelRows";

afterEach(cleanup);
const camera: import("@packages/renderer-core/src/camera3d").Camera3DParams = {
  target: [0, 0, 0],
  distance: 100,
  pitch: Math.PI / 2,
  yaw: 0,
  fovY: 1,
  aspect: 1,
  near: 0.1,
};

test("contact leaders stay on the reported ground center as radius and camera change", () => {
  const handle = createRef<ReadoutLayerHandle>();
  const contact: ContactView = {
    id: 7,
    center: [200, 300],
    radius: 80,
    source: "firing",
    primaryLabel: true,
    evidenceTick: 0,
    expiresTick: 300,
    kind: null,
    heard: ["rifle"],
  };
  const props = { own: [], selected: [], handle, rules: game as unknown as PanelRules };
  const view = render(<ReadoutLayer {...props} contacts={[contact]} />);
  for (const radius of [0, 20, 80]) {
    view.rerender(<ReadoutLayer {...props} contacts={[{ ...contact, radius }]} />);
    for (const scale of [0.5, 1]) {
      handle.current!.place((x, y, z) => [x * scale + 50, y * scale - z], camera, {
        ground: () => 12,
      });
      const path = view.container.querySelector(".ro-leader.ro-contact")!.getAttribute("d")!;
      const line = [...path.matchAll(/M ([\d.-]+) ([\d.-]+)/g)][1];
      expect([Number(line[1]), Number(line[2])]).toEqual([200 * scale + 50, 300 * scale - 12]);
    }
  }
});

test("only the preferred contact report gets a label", () => {
  const base: ContactView = {
    id: 1,
    center: [200, 300],
    radius: 20,
    source: "last_seen",
    kind: "at",
    heard: [],
    evidenceTick: 0,
    expiresTick: 300,
    primaryLabel: true,
  };
  const handle = createRef<ReadoutLayerHandle>();
  const view = render(
    <ReadoutLayer
      own={[]}
      selected={[]}
      handle={handle}
      rules={game as unknown as PanelRules}
      contacts={[base, { ...base, id: 2, source: "firing", kind: null, primaryLabel: false }]}
    />,
  );
  expect(
    [...view.container.querySelectorAll("[data-contact]")].map((n) =>
      n.getAttribute("data-contact"),
    ),
  ).toEqual(["1"]);
});

test("own callout bounds are selectable, including rectangle overlap, but hidden panels are not", () => {
  const handle = createRef<ReadoutLayerHandle>();
  const unit = {
    id: 3,
    kind: "tank",
    position: [200, 300, 0],
    hp: 100,
    memberHp: [],
    mounts: [],
    stock: null,
    service: "full",
    suppression: "none",
    deployment: null,
    garrison: null,
  } as unknown as import("../src/battle/sim/observation").OwnUnitView;
  const view = render(
    <ReadoutLayer
      own={[unit]}
      selected={[]}
      handle={handle}
      rules={game as unknown as PanelRules}
    />,
  );
  const node = view.container.querySelector("[data-unit]") as HTMLDivElement;
  node.getBoundingClientRect = () => ({ left: 230, right: 380, top: 220, bottom: 266 }) as DOMRect;
  handle.current!.place((x, y) => [x, y], camera);
  expect(handle.current!.pick(250, 240)).toEqual({ unit: 3, enemy: null, contact: null });
  expect(handle.current!.inRect(3, { x0: 370, x1: 390, y0: 240, y1: 250 })).toBe(true);
  handle.current!.place(() => [20, 300], camera);
  expect(handle.current!.pick(250, 240)).toBeNull();
  expect(handle.current!.inRect(3, { x0: 230, x1: 380, y0: 220, y1: 266 })).toBe(false);
});

test("panels show name and health until details are requested", () => {
  const handle = createRef<ReadoutLayerHandle>();
  const units = Array.from({ length: 9 }, (_, id) => ({
    id,
    kind: "tank",
    position: [200 + id * 50, 300, 0],
    hp: 100,
    memberHp: [],
    mounts: [],
    stock: null,
    service: "",
    suppression: "none",
    deployment: null,
    garrison: null,
  })) as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const view = render(
    <ReadoutLayer
      own={units}
      selected={[]}
      handle={handle}
      rules={game as unknown as PanelRules}
    />,
  );
  handle.current!.place((x, y) => [x, y], camera);
  const layer = view.container.querySelector(".ro-layer")!;
  expect(layer.getAttribute("data-zoom")).toBe("compressed");
  handle.current!.place((x, y) => (x < 500 ? [x, y] : null), camera);
  expect(layer.getAttribute("data-zoom")).toBe("compressed");
  handle.current!.place((x, y) => [x, y], camera, {}, null, true);
  expect(layer.getAttribute("data-zoom")).toBe("default");
  handle.current!.place((x, y) => [x, y], camera);
  expect(layer.getAttribute("data-zoom")).toBe("compressed");
});

test("held details reach nearby cards with only one card moved", () => {
  const handle = createRef<ReadoutLayerHandle>();
  const units = [
    [100, 100],
    [100, 130],
    [400, 100],
    [100, 300],
  ].map((p, id) => ({
    id,
    kind: "tank",
    position: [...p, 0],
    hp: 100,
    memberHp: [],
    mounts: [],
    stock: null,
    service: "",
    suppression: "none",
    deployment: null,
    garrison: null,
  })) as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const view = render(
    <ReadoutLayer
      own={units}
      selected={[0]}
      handle={handle}
      rules={game as unknown as PanelRules}
    />,
  );
  const layer = view.container.querySelector(".ro-layer") as HTMLElement;
  const cards = [...view.container.querySelectorAll<HTMLDivElement>(".ro-unit")];
  for (const card of cards) {
    const full = () => (card.dataset.zoom ?? layer.dataset.zoom) === "default";
    Object.defineProperties(card, {
      offsetWidth: { get: () => (full() ? 160 : 80) },
      offsetHeight: { get: () => (full() ? 50 : 15) },
    });
  }
  handle.current!.place((x, y) => [x, y], camera);
  const positions = cards.map((c) => c.style.transform);
  handle.current!.place((x, y) => [x, y], camera, {}, null, true);
  expect(cards.filter((c, i) => c.style.transform !== positions[i])).toHaveLength(1);
  expect(cards[0].dataset.zoom).toBe("default");
  expect(cards[2].dataset.zoom).toBe("default");
  expect(cards.filter((c) => c.dataset.zoom === "default").length).toBeGreaterThanOrEqual(2);
  handle.current!.place((x, y) => [x, y], camera);
  expect(cards.map((c) => c.style.transform)).toEqual(positions);
  expect(cards.every((c) => c.dataset.zoom === "compressed")).toBe(true);
});

test("detail priority follows the actual camera eye rather than its orbit target", () => {
  const handle = createRef<ReadoutLayerHandle>();
  const units = [200, 600].map((x, id) => ({
    id,
    kind: "tank",
    position: [x, 0, 0],
    hp: 100,
    memberHp: [],
    mounts: [],
    stock: null,
    service: "",
    suppression: "none",
    deployment: null,
    garrison: null,
  })) as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const view = render(
    <ReadoutLayer
      own={units}
      selected={[1]}
      handle={handle}
      rules={game as unknown as PanelRules}
    />,
  );
  const cards = [...view.container.querySelectorAll<HTMLDivElement>(".ro-unit")];
  for (const [i, card] of cards.entries())
    Object.defineProperties(card, {
      offsetWidth: { get: () => (card.dataset.zoom === "default" ? (i ? 60 : 160) : 40) },
      offsetHeight: { get: () => (card.dataset.zoom === "default" ? (i ? 200 : 100) : 10) },
    });
  const project = (x: number): [number, number] => (x === 200 ? [100, 200] : [200, 120]);
  const low = {
    ...camera,
    distance: 600,
    target: [600, 0, 0] as [number, number, number],
    pitch: 0,
    yaw: Math.PI,
  };
  // Eye at X=0, target at X=600: the selected unit on the target is farther.
  handle.current!.place(project, low, {}, null, true);
  expect(cards.map((c) => c.dataset.zoom)).toEqual(["default", "compressed"]);
  handle.current!.place(project, { ...low, yaw: 0 }, {}, null, true);
  expect(cards.map((c) => c.dataset.zoom)).toEqual(["compressed", "default"]);
});

test("retiring contact panels fade together with their leader and cannot be picked", () => {
  const handle = createRef<ReadoutLayerHandle>();
  const contact = {
    id: 7,
    source: "last_seen" as const,
    center: [200, 300] as [number, number],
    radius: 20,
    evidenceTick: 0,
    expiresTick: 300,
    primaryLabel: true,
    kind: "tank",
    heard: [],
    opacity: 0.4,
    retiring: true,
  };
  const view = render(
    <ReadoutLayer
      own={[]}
      selected={[]}
      handle={handle}
      rules={game as unknown as PanelRules}
      contacts={[contact]}
      tick={60}
    />,
  );
  const node = view.container.querySelector("[data-contact]") as HTMLDivElement;
  node.getBoundingClientRect = () => ({ left: 230, right: 380, top: 220, bottom: 266 }) as DOMRect;
  handle.current!.place((x, y) => [x, y], camera);
  expect(node.closest<HTMLElement>(".ro-callout")!.style.opacity).toBe("0.4");
  expect(node.closest(".ro-callout")!.contains(view.container.querySelector(".ro-leaders"))).toBe(
    true,
  );
  expect(handle.current!.pick(250, 240)).toBeNull();
  expect(node.textContent).toContain("LAST SEEN 2 s AGO");
});
