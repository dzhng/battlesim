// @vitest-environment node
import { expect, test } from "vitest";
import { isChromium } from "../src/browserSupport";

const MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)";
const IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)";

test("Chrome and the browsers on its engine open the game", () => {
  for (const userAgent of [
    `${MAC} AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36`,
    `${MAC} AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36 Edg/142.0.0.0`,
    `${MAC} AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/148.0.0.0 Safari/537.36`,
  ])
    expect(isChromium(userAgent)).toBe(true);
});

test("Safari, Firefox and every iOS browser get the notice", () => {
  for (const userAgent of [
    `${MAC} AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15`,
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:145.0) Gecko/20100101 Firefox/145.0",
    `${IPHONE} AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/142.0.0.0 Mobile/15E148 Safari/604.1`,
    `${IPHONE} AppleWebKit/605.1.15 (KHTML, like Gecko) EdgiOS/142.0.0.0 Mobile/15E148 Safari/605.1.15`,
  ])
    expect(isChromium(userAgent)).toBe(false);
});
