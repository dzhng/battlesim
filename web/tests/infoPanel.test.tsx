// @vitest-environment jsdom
import { TEST_CATALOG, UNITS } from "./catalog";
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { InfoPanel } from "../src/battle/present/infoPanel";
import { stateIcon } from "@packages/scene-assets/src/icons";
import {
  enemyPanel,
  ownPanel,
  weaponRows,
  type Panel,
  type PanelRules,
} from "../src/battle/present/panelRows";

import game from "@fixtures/game.json";
import { panelSpecimens, specimenUnit } from "@apps/battle-lab/src/panelSpecimens";

const fixtureRules = game as unknown as PanelRules;
afterEach(cleanup);

test("concealment remains beside the name in every detail mode and clears with the bonus", () => {
  const hidden: Panel = {
    name: "RIFLE SQUAD",
    strength: 1,
    mark: null,
    weapons: [],
    states: [
      {
        state: "hidden",
        icon: stateIcon("hidden"),
        word: "HIDDEN",
        progress: null,
        fill: null,
        lasting: true,
        tone: null,
      },
    ],
  };
  const view = render(<InfoPanel panel={hidden} />);
  for (const zoom of ["default", "far", "compressed"] as const) {
    view.rerender(<InfoPanel panel={hidden} zoom={zoom} />);
    const name = view.container.querySelector(".ro-name")!;
    expect(name.querySelector('[title="Hidden"] svg')).not.toBeNull();
    expect(name.textContent?.trim()).toBe("RIFLE SQUAD");
  }
  view.rerender(<InfoPanel panel={{ ...hidden, states: [] }} zoom="compressed" />);
  expect(view.container.querySelector('.ro-name [title="Hidden"]')).toBeNull();
});

const rules: PanelRules = {
  tick_hz: 30,
  weapons: { atgm: { name: "ATGM", ammo: 4, icon: "atgm" } },
  service: { radius_m: 80 },
};

const panel = (aim: number, reload: number, loaded: number | null): Panel => ({
  name: "AT TEAM",
  strength: 1,
  mark: null,
  states: [],
  weapons: weaponRows(
    [
      {
        name: "launcher",
        weapons: ["atgm"],
        squad: false,
        special: true,
        turret: false,
        on: null,
        pivot_m: [0, 0, 0],
        muzzle_m: null,
      },
    ],
    rules,
    [
      {
        mount: 0,
        aim,
        reload,
        loaded,
        reloading: loaded === null ? 0 : null,
        ammo: [3],
        target: { kind: "identified", id: 1 },
        reason: "reloading",
        guiding: false,
      },
    ],
  ),
});

test("one progress ring shows aiming first, then reload, then disappears", () => {
  const view = render(<InfoPanel panel={panel(0.4, 0.7, null)} />);
  const circle = () => view.container.querySelectorAll(".ro-ring .ro-track");
  expect(circle()).toHaveLength(1);
  expect(view.container.querySelector(".ro-aim")).not.toBeNull();
  expect(view.container.querySelector(".ro-reload")).toBeNull();
  const radius = circle()[0].getAttribute("r");
  const aimArc = view.container.querySelector(".ro-arc")!.getAttribute("d");

  view.rerender(<InfoPanel panel={panel(1, 0.7, null)} />);
  expect(circle()).toHaveLength(1);
  expect(circle()[0].getAttribute("r")).toBe(radius);
  expect(view.container.querySelector(".ro-aim")).toBeNull();
  expect(view.container.querySelector(".ro-reload")).not.toBeNull();
  expect(view.container.querySelector(".ro-arc")!.getAttribute("d")).not.toBe(aimArc);

  view.rerender(<InfoPanel panel={panel(1, 0, 0)} />);
  expect(view.container.querySelector(".ro-ring")).toBeNull();
});

test("unit cards omit personnel counts, including for own squads", () => {
  const squad = specimenUnit(TEST_CATALOG, "test_rifle", { memberHp: [10, 0, 3] });
  const own = render(<InfoPanel panel={ownPanel(UNITS, squad, [squad], fixtureRules)} />);
  expect(own.queryByText("2 soldiers")).toBeNull();
  own.unmount();
  const enemy = render(<InfoPanel panel={enemyPanel(UNITS, "test_rifle", fixtureRules)} />);
  expect(enemy.queryByText(/soldiers/)).toBeNull();
  enemy.unmount();
  const tank = specimenUnit(TEST_CATALOG, "test_tank");
  const vehicle = render(<InfoPanel panel={ownPanel(UNITS, tank, [tank], fixtureRules)} />);
  expect(vehicle.queryByText(/soldiers/)).toBeNull();
});

test("numbered launchers separate their equipment index from rounds remaining", () => {
  const panel = panelSpecimens(TEST_CATALOG, fixtureRules).find(
    (s) => s.id === "key cases/two launchers, separate reloads",
  )!.panel;
  const view = render(<InfoPanel panel={panel} />);
  expect(view.getByText("ATGM 1").closest(".ro-row")!.textContent).toContain("ATGM 1:2");
  expect(view.getByText("ATGM 2").closest(".ro-row")!.textContent).toContain("ATGM 2:3");
});
