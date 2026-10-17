import { render, screen, cleanup } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { RejectedOrder } from "../src/battle/present/rejectedOrder";

afterEach(cleanup);

test("an order dropped by the rate limit says nothing", () => {
  render(
    <RejectedOrder
      acks={[
        {
          seq: 31,
          label: "move",
          order: { kind: "move", units: [1], gesture: 31, goal: [50, 50], route: "shortest" },
          ack: { seq: 31, applied_tick: 1, refused: [], error: { reason: "rate_limited" } },
        },
      ]}
    />,
  );
  expect(screen.queryByRole("status")).toBeNull();
});

test("a unit refused on its own says why beside its card, not the cursor", () => {
  const box = { x0: 300, y0: 200, x1: 420, y1: 230 };
  render(
    <RejectedOrder
      acks={[]}
      refusals={[{ unit: 6, reason: "no_room_to_unload" }]}
      cardBox={(unit) => (unit === 6 ? box : null)}
    />,
  );
  const message = screen.getByRole("status");
  expect(message.textContent?.trim()).toBe("NO ROOM TO UNLOAD");
  expect(message.style.transform).toBe("translate(426px, 200px)");
});

test("every unit refused in one tick says why, each beside its own card", () => {
  const boxes: Record<number, { x0: number; y0: number; x1: number; y1: number }> = {
    4: { x0: 100, y0: 100, x1: 200, y1: 130 },
    6: { x0: 300, y0: 200, x1: 420, y1: 230 },
  };
  render(
    <RejectedOrder
      acks={[]}
      refusals={[
        { unit: 6, reason: "capacity_full" },
        { unit: 4, reason: "capacity_full" },
      ]}
      cardBox={(unit) => boxes[unit] ?? null}
    />,
  );
  const shown = screen
    .getAllByRole("status")
    .map((m) => [m.dataset.unit, m.textContent?.trim(), m.style.transform])
    .sort();
  expect(shown).toEqual([
    ["4", "TOO MANY TO FIT", "translate(206px, 100px)"],
    ["6", "TOO MANY TO FIT", "translate(426px, 200px)"],
  ]);
});

test("a unit's refusal and a whole order's refusal each say their own", () => {
  const view = render(
    <RejectedOrder
      acks={[]}
      refusals={[{ unit: 6, reason: "no_room_to_unload" }]}
      cardBox={() => ({ x0: 300, y0: 200, x1: 420, y1: 230 })}
    />,
  );
  view.rerender(
    <RejectedOrder
      acks={[
        {
          seq: 2,
          label: "move",
          order: { kind: "move", units: [1], gesture: 2, goal: [50, 50], route: "shortest" },
          ack: { seq: 2, applied_tick: 1, refused: [], error: { reason: "out_of_bounds" } },
        },
      ]}
      refusals={[{ unit: 6, reason: "no_room_to_unload" }]}
      cardBox={() => ({ x0: 300, y0: 200, x1: 420, y1: 230 })}
    />,
  );
  expect(
    screen
      .getAllByRole("status")
      .map((m) => m.textContent?.trim())
      .sort(),
  ).toEqual(["NO ROOM TO UNLOAD", "OUTSIDE THE MAP"]);
});

test("each new carriage refusal reads in player words", () => {
  for (const [reason, text] of [
    ["not_a_carrier", "NOT A CARRIER"],
    ["aboard", "ABOARD"],
  ]) {
    const view = render(
      <RejectedOrder
        acks={[]}
        refusals={[{ unit: 3, reason }]}
        cardBox={() => ({ x0: 0, y0: 0, x1: 10, y1: 10 })}
      />,
    );
    expect(screen.getByRole("status").textContent?.trim()).toBe(text);
    view.unmount();
  }
});
