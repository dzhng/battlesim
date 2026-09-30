// @vitest-environment jsdom
import { createRef } from "react";
import { render, cleanup } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import village from "@fixtures/village.json";
import { ReadoutLayer, type ReadoutLayerHandle } from "../src/battle/present/readouts";
import type { ContactView } from "../src/battle/sim/observation";
import type { PanelRules } from "../src/battle/present/panelRows";

afterEach(cleanup);

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
  const props = { own: [], selected: [], handle, rules: village as unknown as PanelRules };
  const view = render(<ReadoutLayer {...props} contacts={[contact]} />);
  for (const radius of [0, 20, 80]) {
    view.rerender(<ReadoutLayer {...props} contacts={[{ ...contact, radius }]} />);
    for (const scale of [0.5, 1]) {
      handle.current!.place((x, y, z) => [x * scale + 50, y * scale - z], 100, {
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
      rules={village as unknown as PanelRules}
      contacts={[base, { ...base, id: 2, source: "firing", kind: null, primaryLabel: false }]}
    />,
  );
  expect(
    [...view.container.querySelectorAll("[data-contact]")].map((n) =>
      n.getAttribute("data-contact"),
    ),
  ).toEqual(["1"]);
});
