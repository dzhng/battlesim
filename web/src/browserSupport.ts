/** Which browsers the game opens in: Chrome, or another browser on Chrome's
 *  engine (Edge, Brave, Opera, Arc), which draws WebGPU the same way. Safari,
 *  Firefox and every iOS browser (all WebKit underneath, whatever their name)
 *  get a notice instead of the game. */

/** What the check reads of `navigator`. */
export interface BrowserIdentity {
  userAgent: string;
  /** Client hints: Chromium's own brand list, in secure contexts only. */
  userAgentData?: { brands: readonly { brand: string }[] };
}

/** Whether `browser` runs on Chromium. */
export function isChromium(browser: BrowserIdentity): boolean {
  if (browser.userAgentData)
    return browser.userAgentData.brands.some((b) => b.brand === "Chromium");
  // No client hints (a page not served securely): Chromium's user agent names
  // it, and Safari's and Firefox's never do. iOS browsers are WebKit.
  return (
    /\b(?:Headless)?Chrom(?:e|ium)\/\d/.test(browser.userAgent) &&
    !/\b(?:CriOS|FxiOS|EdgiOS)\b/.test(browser.userAgent)
  );
}
