// @vitest-environment node
import { expect, test } from "vitest";
import type { Browser } from "playwright";
import { fileURLToPath } from "node:url";
import { launchBrowser } from "../browser.mjs";
import { browserTestServer } from "./support/browserServer";

test("browser setup supports module imports without booting the application", async () => {
  const server = await browserTestServer();
  let browser: Browser | undefined;
  try {
    browser = await launchBrowser({
      args: ["--disable-gpu", "--disable-software-rasterizer"],
    });
    const page = await browser.newPage();
    const entries: string[] = [];
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/src/main.tsx") entries.push(request.url());
    });
    await page.goto(`${server.url}__test_setup__`);
    const result = await page.evaluate(
      async (moduleUrl) => {
        const importModule = new Function("url", "return import(url)");
        const { createElement } = await importModule(moduleUrl);
        return { title: document.title, element: createElement("button", null, "Ready").type };
      },
      `/@fs/${fileURLToPath(new URL("./support/browserVisit.tsx", import.meta.url))}`,
    );
    expect(result).toEqual({ title: "Browser test setup", element: "button" });
    expect(entries).toEqual([]);
    expect(await page.locator("body").innerText()).toBe("");
  } finally {
    await browser?.close();
    await server.close();
  }
});
