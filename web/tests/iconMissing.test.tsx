import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { Icon } from "../src/battle/present/icons";

afterEach(cleanup);

test("a missing icon draws nothing and reports its path once", () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  const view = render(
    <>
      <Icon path="weapons/undefined.svg" />
      <Icon path="weapons/undefined.svg" />
      <Icon path="weapons/rifle.svg" />
    </>,
  );
  expect(view.container.querySelectorAll(".ro-icon")).toHaveLength(1);
  expect(error.mock.calls).toEqual([["missing icon: assets/icons/weapons/undefined.svg"]]);
  error.mockRestore();
});
