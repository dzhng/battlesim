// @vitest-environment node
import { expect, test } from "vitest";
import { isChromium } from "../src/browserSupport";

const brands = (...names: string[]) => ({ brands: names.map((brand) => ({ brand })) });

test("Chrome and the browsers on its engine open the game", () => {
  for (const userAgentData of [
    brands("Google Chrome", "Chromium", "Not.A/Brand"),
    brands("Microsoft Edge", "Chromium", "Not.A/Brand"),
    brands("Chromium", "HeadlessChrome"),
  ])
    expect(isChromium({ userAgent: "Mozilla/5.0", userAgentData })).toBe(true);
  // Served insecurely, Chromium has no client hints: its user agent names it.
  expect(
    isChromium({
      userAgent:
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36",
    }),
  ).toBe(true);
});

test("Safari, Firefox and every iOS browser get the notice", () => {
  for (const userAgent of [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:145.0) Gecko/20100101 Firefox/145.0",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/142.0.0.0 Mobile/15E148 Safari/604.1",
  ])
    expect(isChromium({ userAgent })).toBe(false);
});
