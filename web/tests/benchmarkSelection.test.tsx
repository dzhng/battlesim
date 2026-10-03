// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import BenchmarkPage from "@apps/battle-lab/src/routes/benchmark";

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
});

test("the advertised city-contact address selects the city workload rather than the village", () => {
  window.history.replaceState(null, "", "/benchmark?preset=city-contact");
  render(<BenchmarkPage />);
  expect(screen.getByText(/city-contact v2/).textContent).toContain("city-contact v2");
  expect(screen.getByText(/local-contact stress/i).textContent).toContain(
    "complete generated world",
  );
  expect(screen.queryByText(/village-contact v1/)).toBeNull();
});

test("the normal benchmark retains its village control and unknown presets refuse", () => {
  window.history.replaceState(null, "", "/benchmark");
  const page = render(<BenchmarkPage />);
  expect(screen.getByText(/village-contact v1/).textContent).toContain("seed 20260925");
  page.unmount();
  window.history.replaceState(null, "", "/benchmark?preset=missing");
  render(<BenchmarkPage />);
  expect(screen.getByTestId("error").textContent).toBe("Unknown benchmark preset.");
  expect(screen.queryByRole("button", { name: /Short run/ })).toBeNull();
});
