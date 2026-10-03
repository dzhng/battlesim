import { readFile, writeFile } from "node:fs/promises";

export async function run(ctx) {
  const source = new URL("../../fixtures/map-presets.json", import.meta.url);
  const before = await readFile(source, "utf8");
  const page = await ctx.newPage({ viewport: { width: 1440, height: 1000 } });
  await page.goto(ctx.url);
  await page.locator(".mw-plan svg").waitFor({ timeout: 120000 });
  const shot = async (name) => writeFile(ctx.evidencePath(`${name}.png`), await page.screenshot());
  await shot("overview");
  const snapshot = await page.evaluate(async () =>
    (await fetch("/__map-workbench/snapshot")).json(),
  );
  const field = snapshot.fields.find(
    (field) =>
      field.document === "presets" &&
      field.path.join(".") === "districts.apartments.streets.block_depth_m",
  );
  if (!field) throw new Error("The apartment construction field is missing");
  const district = page.locator('.mw-plan [data-rule-group="districts.apartments"]').first();
  const point = await district.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    for (let y = 0.1; y < 1; y += 0.1)
      for (let x = 0.1; x < 1; x += 0.1) {
        const point = { x: bounds.x + x * bounds.width, y: bounds.y + y * bounds.height };
        if (document.elementFromPoint(point.x, point.y) === element) return point;
      }
    throw new Error("The district has no exposed clickable ground");
  });
  await page.mouse.click(point.x, point.y);
  const control = page.locator(`[id=${JSON.stringify(`field-${field.id}`)}]`);
  await control.waitFor();
  ctx.check(
    "map selection opens the shared district-kind controls",
    await page.getByText(/wherever this preset is used/).isVisible(),
  );
  const saved = await control.inputValue();
  await page.getByRole("button", { name: "Saved baseline · same seed" }).click();
  await page.waitForFunction(() =>
    document
      .querySelector('.mw-inspection button[aria-pressed="true"]')
      ?.textContent.includes("Saved baseline"),
  );
  await page.getByRole("button", { name: "Inspect selection" }).click();
  await page.waitForFunction(() => {
    const box = document
      .querySelector(".mw-plan svg")
      ?.getAttribute("viewBox")
      ?.split(" ")
      .map(Number);
    return box && box[2] < 6000;
  });
  const baselinePlan = await page.locator(".mw-plan").innerHTML();
  await shot("baseline-before-edit");
  await control.fill(String(Number(saved) + 10));
  await page.getByLabel("Search rules").fill(field.label);
  ctx.check(
    "search retains the map-selected edit",
    (await control.inputValue()) === String(Number(saved) + 10),
  );
  await page.waitForFunction(
    () =>
      /Current draft (admitted|refused)/.test(
        document.querySelector('[role="status"]')?.textContent ?? "",
      ),
    undefined,
    { timeout: 120000 },
  );
  await shot("baseline-after-edit");
  ctx.check(
    "saved comparison retains its exact plan while the draft changes",
    (await page.locator(".mw-plan").innerHTML()) === baselinePlan,
  );
  await page.getByRole("button", { name: "Draft plan", exact: true }).click();
  await shot("selected-edited");
  await control.fill("-");
  ctx.check(
    "unfinished input suspends generation and save",
    (await control.getAttribute("aria-invalid")) === "true" &&
      (await page.getByRole("button", { name: "Review save" }).isDisabled()),
  );
  await shot("unfinished-input");
  await page.getByRole("button", { name: "Undo edit" }).click();
  ctx.check(
    "Undo clears unfinished input and restores the saved value",
    (await control.inputValue()) === saved &&
      (await control.getAttribute("aria-invalid")) === "false",
  );
  await control.fill(String(Number(saved) + 10));
  await page.getByRole("button", { name: "Review save" }).click();
  await page.locator(".mw-review summary").first().waitFor({ timeout: 120000 });
  await page.locator(".mw-review summary").first().click();
  await page.locator(".mw-review").scrollIntoViewIfNeeded();
  await shot("save-review");
  ctx.check(
    "save preview names exact fixture replacements",
    (await page.locator(".mw-review").textContent()).includes("fixtures/map-presets.json"),
  );
  await page.getByRole("button", { name: "Close review" }).click();
  await page.getByLabel("Sample seed count").fill("2");
  await page.getByRole("button", { name: "Run seed sample" }).click();
  await page.waitForFunction(
    () =>
      document
        .querySelector(".mw-sample")
        ?.textContent.includes("2 / 2 requested outcomes · complete"),
    undefined,
    { timeout: 180000 },
  );
  await page.locator(".mw-sample").scrollIntoViewIfNeeded();
  await shot("sample-complete");
  ctx.check(
    "each requested seed retains an outcome",
    (await page.locator(".mw-sample tbody tr").count()) === 2,
  );
  // District IDs are local to each artifact, so reselect after regeneration.
  const currentDistrict = await page
    .locator('.mw-plan [data-rule-group="districts.apartments"]')
    .first()
    .getAttribute("data-feature-id");
  await page.getByLabel("Inspect feature").selectOption(currentDistrict);
  await page.getByRole("button", { name: "Inspect selection" }).click();
  await page.waitForFunction(() => {
    const box = document
      .querySelector(".mw-plan svg")
      ?.getAttribute("viewBox")
      ?.split(" ")
      .map(Number);
    return box && box[2] < 6000;
  });
  await page.locator(".mw-inspection").scrollIntoViewIfNeeded();
  await shot("district-detail");
  ctx.check(
    "a retained sampled preview remains inspectable",
    (await page.locator(".mw-plan svg").count()) === 1,
  );
  await page.getByRole("button", { name: "Measure openness" }).click();
  await page.locator(".mw-sight").waitFor({ timeout: 120000 });
  await page.locator(".mw-sight").scrollIntoViewIfNeeded();
  await shot("openness");
  await page.locator(".mw-measurements > details").evaluateAll((nodes) => {
    for (const node of nodes) node.open = true;
  });
  await page.locator(".mw-measurements details").evaluateAll((nodes) => {
    for (const node of nodes) node.open = true;
  });
  await page
    .locator(".mw-measurements")
    .evaluate((node) => node.scrollIntoView({ block: "start" }));
  await shot("geometry-expanded");
  await page
    .locator(".mw-measurements > details")
    .filter({ has: page.locator("summary", { hasText: /^Encounter$/ }) })
    .evaluate((node) => node.scrollIntoView({ block: "start" }));
  await shot("encounter-expanded");
  await page.locator(".mw-analysis").evaluate((node) => node.scrollIntoView({ block: "end" }));
  await shot("geometry-expanded-bottom");
  await ctx.writeEvidence("openness-small.txt", await page.locator(".mw-sight").textContent());
  // A comparison is cached until its map choice changes. Return to the same seed for this probe.
  await page.getByLabel("Map seed").fill("2");
  await page.waitForFunction(
    () =>
      document.querySelector('.mw-inspection button[aria-pressed="true"]')?.textContent ===
      "Draft plan",
  );
  await Promise.all([
    page.waitForResponse(
      (response) =>
        response.url().endsWith("/__map-workbench/generate") &&
        response.request().postDataJSON().choice.seed === "1" &&
        response.ok(),
    ),
    page.getByLabel("Map seed").fill("1"),
  ]);
  await page.waitForFunction(
    () => document.querySelector('[role="status"]')?.textContent === "Current draft admitted",
  );
  // Control only the baseline response; the displayed draft and regeneration stay native.
  await page.route("**/__map-workbench/generate", async (route) => {
    const request = route.request().postDataJSON();
    const depth = request.draft.documents.presets.districts.apartments.streets.block_depth_m;
    if (request.purpose === "preview" && depth === Number(saved))
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          status: "refused",
          stage: "generation",
          choice: request.choice,
          fingerprint: "controlled-baseline-refusal",
          artifactId: "00000000-0000-4000-8000-000000000001",
          diagnostics: [
            {
              code: "generation_failed",
              feature: "baseline",
              location: "$",
              message: "Controlled saved-seed refusal",
            },
          ],
        }),
      });
    else await route.continue();
  });
  await page.getByRole("button", { name: "Saved baseline · same seed" }).click();
  await page.getByText(/Saved baseline refused:/).waitFor();
  ctx.check(
    "a refused comparison offers no geometry-only actions",
    (await page.getByRole("button", { name: "Whole map" }).isDisabled()) &&
      (await page.getByRole("button", { name: "Measure openness" }).isDisabled()),
  );
  await page.locator(".mw-inspection").evaluate((node) => node.scrollIntoView({ block: "start" }));
  await shot("baseline-refused");
  await page.getByRole("button", { name: "Regenerate", exact: true }).click();
  await page.waitForFunction(
    () => document.querySelector('[role="status"]')?.textContent === "Current draft admitted",
  );
  await page.getByRole("button", { name: "Draft plan", exact: true }).click();
  await Promise.all([
    page.waitForResponse(
      (response) => response.url().endsWith("/__map-workbench/inspect") && response.ok(),
    ),
    page.getByRole("button", { name: "Whole map" }).click(),
  ]);
  ctx.check(
    "regeneration after a refused comparison preserves the admitted native artifact",
    (await page.locator(".mw-plan svg").count()) === 1,
  );
  await page.unroute("**/__map-workbench/generate");
  await page.getByLabel("Search rules").fill("");
  await page.getByLabel("Rule group").selectOption("limits");
  const limit = snapshot.fields.find(
    (field) =>
      field.document === "defaults" && field.path.join(".") === "limits.max_authored_parts",
  );
  const limitControl = page.locator(`[id=${JSON.stringify(`field-${limit.id}`)}]`);
  await limitControl.fill("0");
  await page.locator(".mw-refusal").waitFor({ timeout: 30000 });
  await page.locator(".mw-header").scrollIntoViewIfNeeded();
  await shot("validation-error");
  ctx.check(
    "invalid numeric policy identifies its source field",
    (await page.locator(".mw-refusal").textContent()).includes(
      "$.defaults.limits.max_authored_parts",
    ),
  );
  await limitControl.fill("1");
  await page.waitForFunction(
    () =>
      !document.querySelector(".mw-refusal")?.textContent.includes("Allowance must be positive") &&
      document.querySelector('[role="status"]')?.textContent === "Current draft refused",
    undefined,
    { timeout: 30000 },
  );
  await page.locator(".mw-refusal").scrollIntoViewIfNeeded();
  await shot("refused-draft");
  ctx.check(
    "refusal preserves the older admitted plan and its exact inputs",
    (await page.getByText(/OLDER RESULT/).count()) === 1 &&
      (await page.getByRole("button", { name: "Export refused inputs" }).count()) === 1,
  );
  await page.setViewportSize({ width: 600, height: 900 });
  await page.locator(".mw-review").count();
  await page.locator(".mw-controls").scrollIntoViewIfNeeded();
  await shot("narrow-controls");
  await page.locator(".mw-inspection").scrollIntoViewIfNeeded();
  await shot("narrow-plan");
  await page.locator(".mw-measurements details").evaluateAll((nodes) => {
    for (const node of nodes) node.open = true;
  });
  await page
    .locator(".mw-measurements")
    .evaluate((node) => node.scrollIntoView({ block: "start" }));
  await shot("narrow-geometry");
  await page
    .locator(".mw-measurements > details")
    .filter({ has: page.locator("summary", { hasText: /^Encounter$/ }) })
    .evaluate((node) => node.scrollIntoView({ block: "start" }));
  await shot("narrow-encounter");
  await page.locator(".mw-analysis").evaluate((node) => node.scrollIntoView({ block: "end" }));
  await shot("narrow-geometry-bottom");
  await page.getByRole("button", { name: "Review save" }).click();
  await page.locator(".mw-review summary").first().waitFor();
  await page.locator(".mw-review summary").first().click();
  await page.locator(".mw-review").evaluate((node) => node.scrollIntoView({ block: "start" }));
  await shot("narrow-review");
  const readableField = await page
    .locator(".mw-review-changes code")
    .filter({ hasText: "block_depth_m" })
    .evaluate((code) => {
      const leaf = code.textContent.split(".").at(-1);
      const walker = document.createTreeWalker(code, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const start = node.textContent.indexOf(leaf);
        if (start < 0) continue;
        const range = document.createRange();
        range.setStart(node, start);
        range.setEnd(node, start + leaf.length);
        return range.getClientRects().length === 1;
      }
      return false;
    });
  ctx.check("narrow save review keeps the field name on one line", readableField);
  await page
    .getByRole("button", { name: "Save reviewed defaults" })
    .evaluate((node) => node.scrollIntoView({ block: "end" }));
  await shot("narrow-review-bottom");
  const overflow = await page.evaluate(
    () => document.querySelector(".map-workbench").scrollWidth > innerWidth,
  );
  ctx.check("narrow tool remains within the viewport", !overflow);
  ctx.check(
    "drafts, samples and review do not publish fixtures",
    (await readFile(source, "utf8")) === before,
  );
  await page.close();
}
