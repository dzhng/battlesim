import { render, screen, cleanup } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { RejectedOrder } from "../src/battle/present/rejectedOrder";
import type { Order } from "../src/battle/sim/protocol";

afterEach(cleanup);

test("a partially refused group move tells the player some moves are unavailable", () => {
  const order: Order = {
    kind: "move",
    units: [1, 2],
    gesture: 1,
    goal: [50, 50],
    route: "shortest",
  };
  render(
    <RejectedOrder
      acks={[
        {
          seq: 1,
          label: "move",
          order,
          ack: {
            seq: 1,
            applied_tick: 0,
            error: null,
            placement: {
              gesture: 1,
              destinations: [
                { unit: 1, goal: [50, 50], facing: 0, placed: true },
                { unit: 2, goal: [60, 50], facing: 0, placed: false },
              ],
            },
          },
        },
      ]}
    />,
  );
  expect(screen.getByRole("status").textContent?.trim()).toBe("SOME MOVES NOT AVAILABLE");
});

test("a refusal starts beyond the complete approved cursor composite", () => {
  const view = render(<RejectedOrder acks={[]} />);
  window.dispatchEvent(new MouseEvent("pointermove", { clientX: 120, clientY: 85 }));
  view.rerender(
    <RejectedOrder
      acks={[
        {
          seq: 1,
          label: "move",
          order: { kind: "move", units: [1], gesture: 1, goal: [50, 50], route: "shortest" },
          ack: { seq: 1, applied_tick: 1, error: { reason: "no_valid_destination" } },
        },
      ]}
    />,
  );
  const callout = screen.getByRole("status");
  const x = Number(callout.style.transform.match(/translate\(([^p]+)px/)![1]);
  // The approved arrow+badge occupies 41 screen pixels to the right of its tip.
  expect(x).toBeGreaterThan(120 + 41);
  expect(callout.textContent).toContain("MOVE NOT AVAILABLE");
});
