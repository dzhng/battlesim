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
          ack: { seq: 31, applied_tick: 1, error: { reason: "rate_limited" } },
        },
      ]}
    />,
  );
  expect(screen.queryByRole("status")).toBeNull();
});
