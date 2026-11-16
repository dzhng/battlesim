// Browser proofs of the commanded entry and screen-relative camera controls.
// Measure the submitted view through the page's own projector, not a second
// implementation of the controller's yaw-to-pan arithmetic.
export async function openingView(ctx, page, side, evidence) {
  const proof = await page.evaluate(() => {
    const { start, size } = window.__lab.route.prepared();
    const project = (point) =>
      window.__lab.projectToCss(...point, window.__lab.route.surfaceZ(...point));
    const entry = project(start.at);
    const centre = project([size[0] / 2, size[1] / 2]);
    return { camera: window.__lab.camera(), start, size, entry, centre };
  });
  ctx.check(
    `${side}: own entry lies below the map centre in the opening view`,
    !!proof.entry && !!proof.centre && proof.entry[1] > proof.centre[1],
    JSON.stringify(proof),
  );
  await page.screenshot({ path: `${evidence}${side}-opening.png` });
  await ctx.writeEvidence(`${side}-opening.json`, proof);
  // A close view of the same road exposes the painted lane digit, without
  // changing the yaw. Restore the player's real opening before continuing.
  if (side === "red") {
    await page.evaluate(async ({ camera, start }) => {
      window.__lab.setCamera({ ...camera, target: [...start.at, 0], distance: 160 });
      await window.__lab.frame();
    }, proof);
    await page.screenshot({ path: `${evidence}red-lanes.png` });
  }
  await restore(page, proof.camera);
}

async function restore(page, camera) {
  // A pointer left on an edge would start another pan during the next sample.
  await page.mouse.move(100, 360);
  await page.evaluate(async (camera) => {
    window.__lab.setCamera(camera);
    await window.__lab.frame();
  }, camera);
}

export async function screenControls(ctx, page, side) {
  const opening = await page.evaluate(() => window.__lab.camera());
  const readings = [];
  for (const [name, keys, edge, expected] of [
    ["up", ["w", "ArrowUp"], [100, 2], [0, -1]],
    ["down", ["s", "ArrowDown"], [100, 718], [0, 1]],
    ["left", ["a", "ArrowLeft"], [2, 360], [-1, 0]],
    ["right", ["d", "ArrowRight"], [1278, 360], [1, 0]],
  ]) {
    for (const control of [...keys, "edge"]) {
      await restore(page, opening);
      const before = await page.evaluate(() => window.__lab.camera());
      if (control === "edge") await page.mouse.move(...edge);
      else await page.keyboard.down(control);
      let reading;
      try {
        // The old centre must slide opposite the requested camera movement.
        // Waiting on pixels rather than elapsed time survives a slow GPU.
        reading = await page
          .waitForFunction(
            ({ target, expected }) => {
              const at = window.__lab.projectToCss(...target);
              if (!at) return false;
              const delta = [at[0] - innerWidth / 2, at[1] - innerHeight / 2];
              return Math.hypot(...delta) > 8 ? { at, delta, expected } : false;
            },
            { target: before.target, expected },
            { timeout: 10_000 },
          )
          .then((value) => value.jsonValue());
      } finally {
        if (control !== "edge") await page.keyboard.up(control);
        await page.mouse.move(100, 360);
      }
      const { delta } = reading;
      const along = delta[0] * expected[0] + delta[1] * expected[1];
      const across = delta[0] * expected[1] - delta[1] * expected[0];
      ctx.check(
        `${side}: ${control === "edge" ? name + " screen edge" : control} pans ${name} on screen`,
        along < -8 && Math.abs(across) < Math.abs(along) * 0.1,
        JSON.stringify(reading),
      );
      readings.push({ name, control, ...reading });
    }
  }
  await restore(page, opening);
  await ctx.writeEvidence(`${side}-camera-controls.json`, readings);
}

export async function screenUpOrder(ctx, page, side, move, evidence) {
  const before = await page.evaluate(async () => {
    const unit = window.__lab.route.observation().own[0];
    const camera = window.__lab.camera();
    window.__lab.setCamera({ ...camera, target: unit.position, distance: 220 });
    await window.__lab.frame();
    // Pick a ground point directly above the unit through the submitted view.
    const at = window.__lab.projectToCss(...unit.position);
    const ray = window.__lab.rayAt(at[0], at[1] - 100);
    const t = -ray.origin[2] / ray.dir[2];
    const goal = [ray.origin[0] + t * ray.dir[0], ray.origin[1] + t * ray.dir[1]];
    const { start, size } = window.__lab.route.prepared();
    return { id: unit.id, position: unit.position, goal, start: start.at, size };
  });
  await move(page, before.goal);
  const proof = await page.evaluate(({ id, position, start, size }) => {
    const unit = window.__lab.route.observation().own.find((unit) => unit.id === id);
    if (!unit) return null;
    const project = (point) => window.__lab.projectToCss(...point);
    const was = project(position);
    const now = project(unit.position);
    const fromEntry = [size[0] / 2 - start[0], size[1] / 2 - start[1]];
    const delta = [unit.position[0] - position[0], unit.position[1] - position[1]];
    return {
      was,
      now,
      position: unit.position,
      inward: delta[0] * fromEntry[0] + delta[1] * fromEntry[1],
    };
  }, before);
  ctx.check(
    `${side}: an own unit ordered straight up screen advances away from its entry edge`,
    !!proof && !!proof.was && !!proof.now && proof.now[1] < proof.was[1] && proof.inward > 0,
    JSON.stringify({ before, proof }),
  );
  await page.screenshot({ path: `${evidence}${side}-mid.png` });
  await ctx.writeEvidence(`${side}-up-screen-order.json`, { before, proof });
}
