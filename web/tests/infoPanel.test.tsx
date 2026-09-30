// @vitest-environment jsdom
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { InfoPanel } from "../src/battle/present/infoPanel";
import { weaponRows, type Panel, type PanelRules } from "../src/battle/present/panelRows";

afterEach(cleanup);

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
