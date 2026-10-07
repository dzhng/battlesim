import { createRef } from "react";
import { UNITS } from "./catalog";
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, render as renderView } from "@testing-library/react";
import { WithTestCatalog } from "./catalog";
import game from "@fixtures/game.json";
import type { PanelRules } from "@web/battle/present/panelRows";
import type { ReadoutLayerHandle } from "@web/battle/present/readouts";
import { ContactPresentation } from "@web/battle/present/contactPresentation";
import type { ContactView } from "@web/battle/sim/observation";
import Weapons from "@apps/battle-lab/src/routes/weapons";
import FogLook from "@apps/battle-lab/src/routes/fogLook";
import { PointerPaint } from "@apps/battle-lab/src/pointerPaint";

/** Views render under the test set, as a lab page provides it. */
const render = (ui: Parameters<typeof renderView>[0]) =>
  renderView(ui, { wrapper: WithTestCatalog });

let session: ReturnType<typeof makeSession>;
vi.mock("@apps/battle-lab/src/useBattleSession", () => ({ useBattleSession: () => session }));
vi.mock("@apps/battle-lab/src/LabViewport", () => ({ LabViewport: () => null }));
vi.mock("@apps/battle-lab/src/streetScenario", async (original) => ({
  ...(await original<typeof import("@apps/battle-lab/src/streetScenario")>()),
  useStreetScenario: () => "{}",
}));
vi.mock("@apps/battle-lab/src/savedMaps", () => ({
  SavedEncounter: ({ children }: { children: (battle: { scenario: string }) => unknown }) =>
    children({ scenario: "{}" }),
}));
vi.mock("@apps/battle-lab/src/FeedInspector", () => ({ FeedInspector: () => null }));
vi.mock("@apps/battle-lab/src/AckLog", () => ({ AckLog: () => null }));
afterEach(cleanup);

const report: ContactView = {
  id: 7,
  source: "firing",
  center: [400, 300],
  radius: 20,
  evidenceTick: 5,
  expiresTick: 20,
  kind: null,
  heard: [],
};
function makeSession() {
  return {
    world: {},
    meshes: {},
    pointerPaint: new PointerPaint(UNITS),
    units: UNITS,
    fog: null,
    surfaceZ: () => 0,
    rules: game as unknown as PanelRules,
    contacts: new ContactPresentation(30, 3).update([report], 5),
    readouts: createRef<ReadoutLayerHandle>(),
    probes: {},
    control: { selected: [], selectedUnits: [], acks: [] },
    sim: {
      observation: { tick: 5, contacts: [report], own: [], identified: [], projectiles: [] },
      status: { status: "paused" },
    },
  };
}

for (const [name, Route] of [
  ["weapons", Weapons],
  ["fog-look", FogLook],
] as const) {
  test(`${name} uses the shared truthful contact label through removal and renewal`, () => {
    session = makeSession();
    const shown = new ContactPresentation(30, 3);
    session.contacts = shown.update([report], 5);
    const view = render(<Route />);
    for (const [tick, alpha] of [
      [5, 1],
      [21, 1],
      [66, 0.5],
      [111, null],
    ] as const) {
      session.sim.observation = {
        ...session.sim.observation,
        tick,
        contacts: tick === 5 ? [report] : [],
      };
      session.contacts = shown.update(session.sim.observation.contacts, tick);
      view.rerender(<Route />);
      const nodes = view.container.querySelectorAll<HTMLElement>("[data-contact]");
      if (alpha === null) expect(nodes.length).toBe(0);
      else {
        expect(nodes.length).toBe(1);
        expect(nodes[0].dataset.contact).toBe("7");
        expect(nodes[0].textContent).toContain("UNKNOWN");
        expect(Number((nodes[0].closest(".ro-callout") as HTMLElement).style.opacity)).toBe(alpha);
        expect(nodes[0].closest(".ro-callout")?.hasAttribute("data-retiring")).toBe(tick !== 5);
      }
    }
    session.sim.observation = {
      ...session.sim.observation,
      tick: 112,
      contacts: [{ ...report, evidenceTick: 112, expiresTick: 127 }],
    };
    session.contacts = shown.update(session.sim.observation.contacts, 112);
    view.rerender(<Route />);
    expect(view.container.querySelectorAll("[data-contact]").length).toBe(1);
    expect(view.container.querySelector("[data-contact]")?.textContent).toContain("HEARD 0 s AGO");
  });
}
