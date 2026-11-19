/** A shop an authored scenario narrowed: only the cards it offers, a squad
 *  only in its offered rides, under only the tabs those cards stand under. */
import { act, cleanup, fireEvent, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import {
  APC,
  highlighted,
  match,
  render,
  RIDES,
  RIFLES,
  RULES,
  shownNames,
  tab,
  TEAM,
  transportMenu,
} from "./support/shop";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

/** The shop over the transport cards, narrowed to `offered`. */
function openNarrowed(offered: { kind: string; carrier?: string }[]) {
  const choose = vi.fn();
  const view = render(
    <PurchasePicker
      cards={RIDES}
      faction="us"
      match={{ ...match, players: [{ ...match.players[0], offered }] }}
      onChoose={choose}
      rules={RULES}
    />,
  );
  fireEvent.click(view.getByRole("button", { name: "Reinforcements" }));
  return { view, choose };
}

/** The group tabs on show, by label. */
const tabs = (view: ReturnType<typeof render>) =>
  view.getAllByRole("tab").map((t) => t.textContent);

test("a shop an authored scenario narrowed shows only what it offers, and a squad only in its offered rides", () => {
  vi.useFakeTimers();
  // The rifles only in the APC, the AT team on foot; the carrier never alone.
  const offered = [{ kind: RIFLES.id, carrier: APC.id }, { kind: TEAM.id }];
  const { view, choose } = openNarrowed(offered);
  // Only the tab its cards stand under, open.
  expect(tabs(view)).toEqual([tab("inf")]);
  expect(shownNames(view)).toEqual([RIFLES.name, TEAM.name]);
  fireEvent.mouseEnter(view.getByRole("button", { name: TEAM.name }));
  expect(transportMenu(view)).toEqual([["On foot", ["On foot"]]]);
  fireEvent.mouseEnter(view.getByRole("button", { name: RIFLES.name }));
  act(() => vi.advanceTimersByTime(1000));
  expect(transportMenu(view)).toEqual([[tab("ifv_apc"), [APC.name]]]);
  // With no way on foot, the squad's card previews its one ride.
  expect(highlighted(view)).toEqual([APC.name]);
  const menu = within(view.getByRole("region", { name: "Transport" }));
  fireEvent.click(menu.getByRole("button", { name: APC.name }));
  expect(choose).toHaveBeenCalledExactlyOnceWith(RIFLES.id, APC.id);
});
