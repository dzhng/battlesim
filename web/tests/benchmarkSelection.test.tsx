// @vitest-environment jsdom
import { cleanup, screen } from "@testing-library/react";
import { renderInRouter as render } from "./support/router";
import { afterEach, expect, test } from "vitest";
import BenchmarkPage from "@apps/battle-lab/src/routes/benchmark";
import { CITY_CONTACT } from "@web/battle/benchmark/presets";

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
});

test("the advertised city-contact address and the plain benchmark both select the city workload", () => {
  for (const address of ["/benchmark?preset=city-contact", "/benchmark"]) {
    window.history.replaceState(null, "", address);
    const page = render(<BenchmarkPage />);
    const name = `city-contact v${CITY_CONTACT.version}`;
    expect(screen.getByText(new RegExp(name)).textContent, address).toContain(name);
    expect(screen.getByText(/local-contact stress/i).textContent).toContain(
      "complete generated world",
    );
    page.unmount();
  }
});

test("an unknown preset refuses", () => {
  window.history.replaceState(null, "", "/benchmark?preset=missing");
  render(<BenchmarkPage />);
  expect(screen.getByTestId("error").textContent).toBe("Unknown benchmark preset.");
  expect(screen.queryByRole("button", { name: /Short run/ })).toBeNull();
});
