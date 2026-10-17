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
      refusals={[{ key: 1, tick: 40, unit: 6, reason: "no_room_to_unload" }]}
      cardBox={(unit) => (unit === 6 ? box : null)}
    />,
  );
  const message = screen.getByRole("status");
  expect(message.textContent?.trim()).toBe("NO ROOM TO UNLOAD");
  expect(message.style.transform).toBe("translate(426px, 200px)");
});

test("each new carriage refusal reads in player words", () => {
  for (const [reason, text] of [
    ["not_a_carrier", "NOT A CARRIER"],
    ["aboard", "ABOARD"],
  ]) {
    const view = render(
      <RejectedOrder
        acks={[]}
        refusals={[{ key: 2, tick: 1, unit: 3, reason }]}
        cardBox={() => ({ x0: 0, y0: 0, x1: 10, y1: 10 })}
      />,
    );
    expect(screen.getByRole("status").textContent?.trim()).toBe(text);
    view.unmount();
  }
});
