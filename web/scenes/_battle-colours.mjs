import { writeFile } from "node:fs/promises";

// Role comparisons read the real components' computed paint, never palette
// literals. The result specimen is the gallery's finished battle: playing a
// live 1v1 to its normal score target would take half an hour.
export async function captureSeats(ctx, authority, pages, moment, evidence) {
  try {
    // Stop the process, including its battle worker, outside the product's
    // protocol. Drain both streams and insist the publications AND submitted
    // frames agree before photographing either seat.
    let ticks;
    for (let attempt = 0; attempt < 20; attempt++) {
      authority.hold();
      await pages[0].waitForTimeout(400);
      ticks = await Promise.all(
        pages.map((page) =>
          page.evaluate(() => ({
            observed: window.__lab.route.tick(),
            drawn: window.__lab.route.presented()?.tick,
          })),
        ),
      );
      if (ticks.every((t) => t.observed === ticks[0].observed && t.drawn === t.observed)) break;
      authority.resume();
      await pages[0].waitForTimeout(100);
    }
    const same = ticks.every((t) => t.observed === ticks[0].observed && t.drawn === t.observed);
    ctx.check(
      `${moment}: both seats photograph the same submitted battle tick`,
      same,
      JSON.stringify(ticks),
    );
    if (!same) throw new Error(`Could not align ${moment}: ${JSON.stringify(ticks)}`);
    const paints = [];
    for (const [i, page] of pages.entries()) {
      const side = i === 0 ? "blue" : "red";
      await page.evaluate((moment) => {
        const o = window.__lab.route.observation();
        const unit = o.own[0];
        if (!unit) return;
        window.__lab.route.select([unit.id]);
        const enemy = moment === "contact" ? o.identified[0] : null;
        const target = enemy
          ? unit.position.map((v, k) => (v + enemy.position[k]) / 2)
          : unit.position;
        const distance = enemy
          ? Math.max(220, Math.hypot(...unit.position.map((v, k) => v - enemy.position[k])) * 1.6)
          : 220;
        window.__lab.setCamera({ ...window.__lab.camera(), target, distance });
      }, moment);
      await page.keyboard.down("Space");
      await page.mouse.move(-5, -5);
      await page.waitForTimeout(250);
      paints.push(
        await page.evaluate(() => {
          const paint = (node) => (node ? getComputedStyle(node).color : null);
          const own = document.querySelector("[data-unit]");
          return {
            own: paint(own?.querySelector("span")),
            enemy: paint(document.querySelector("[data-enemy] span")),
            selected: own?.parentElement.querySelector(":scope > svg > path")
              ? getComputedStyle(own.parentElement.querySelector(":scope > svg > path")).stroke
              : null,
            score: paint(document.querySelector('[aria-label="Your victory score"]')),
            enemyScore: paint(document.querySelector('[aria-label="Enemy victory score"]')),
            objectives: window.__lab.route.observation().skirmish.objectives.map((o, i) => ({
              owner: o.owner,
              label: paint(document.querySelector(`[aria-label="Capture area ${i + 1}"]`)),
              flag: paint([...document.querySelector('[aria-label="Objectives"]').children][i]),
            })),
          };
        }),
      );
      await page.screenshot({ path: `${evidence}${side}-${moment}.png`, animations: "disabled" });
      await page.keyboard.up("Space");
    }
    if (moment !== "preparation")
      ctx.check(
        `${moment}: own callouts and selected leaders agree across seats`,
        paints[0].own !== null &&
          paints[0].own === paints[1].own &&
          paints[0].selected !== null &&
          paints[0].selected === paints[1].selected,
        JSON.stringify(paints),
      );
    if (moment !== "preparation")
      ctx.check(
        `${moment}: both seats use the same distinct own and enemy score paint`,
        paints[0].score !== null &&
          paints[0].score === paints[1].score &&
          paints[0].enemyScore === paints[1].enemyScore &&
          paints[0].score !== paints[0].enemyScore,
        JSON.stringify(paints),
      );
    if (moment === "held")
      ctx.check(
        "held: objective labels and score-strip flags share their side's role paint",
        paints.every((p, i) => {
          const side = i === 0 ? "blue" : "red";
          const held = p.objectives.filter((o) => o.owner !== null);
          return (
            held.some((o) => o.owner === side) &&
            held.some((o) => o.owner !== side) &&
            held.every(
              (o) => o.label === (o.owner === side ? p.score : p.enemyScore) && o.flag === o.label,
            )
          );
        }),
        JSON.stringify(paints),
      );
    if (moment === "contact")
      ctx.check(
        "contact: both commanders see enemy callouts in the same distinct enemy paint",
        paints[0].enemy !== null &&
          paints[0].enemy === paints[1].enemy &&
          paints.every((p) => p.enemy === p.enemyScore && p.enemy !== p.own),
        JSON.stringify(paints),
      );
    await ctx.writeEvidence(`${moment}-colours.json`, { ticks, paints });
  } finally {
    authority.resume();
  }
}

export async function resultColours(ctx, origin, evidence) {
  const pages = [await ctx.newPage(), await ctx.newPage(), await ctx.newPage()];
  const paints = [];
  try {
    for (const [i, page] of pages.entries()) {
      await page.goto(`${origin}/labs/ui?shot=report-1v1`);
      await page.getByRole("dialog", { name: "Match result" }).waitFor();
      await page.evaluate(
        async ({ side, root }) => {
          const { showResult } = await import("/@fs" + root + "web/scenes/_result-seat.mjs");
          await showResult(side === "spectator" ? null : `${side}-1`);
        },
        {
          side: ["blue", "red", "spectator"][i],
          root: new URL("../../", import.meta.url).pathname,
        },
      );
      await page.getByRole("dialog", { name: "Match result" }).waitFor();
      const side = ["blue", "red", "spectator"][i];
      paints.push(
        await page.getByRole("dialog", { name: "Match result" }).evaluate((dialog) => {
          const scores = dialog.querySelectorAll('[aria-label="Final score"] b');
          // A commander's own side comes first; a watcher's first is Blue.
          const groups = dialog.querySelectorAll('[role="group"]');
          return {
            own: getComputedStyle(scores[0]).color,
            enemy: getComputedStyle(scores[1]).color,
            border: getComputedStyle(dialog.firstElementChild).borderTopColor,
            ownHeading: getComputedStyle(groups[0].querySelector("th")).color,
            enemyHeading: getComputedStyle(groups[1].querySelector("th")).color,
            numbers: [...scores].map((score) => score.textContent),
            headings: [...groups].map((group) => group.querySelector("th").textContent),
            // Whether the viewer's own row is in the first scoreboard.
            ownFirst: groups[0].querySelector("[data-you]") !== null,
          };
        }),
      );
      await page.screenshot({ path: `${evidence}${side}-result.png` });
    }
    ctx.check(
      "result: either commander's own final score wears the same paint",
      paints[0].own === paints[1].own && paints[0].own !== paints[0].enemy,
      JSON.stringify(paints),
    );
    ctx.check(
      "result: either commander's enemy final score wears the same paint",
      paints[0].enemy === paints[1].enemy,
      JSON.stringify(paints),
    );
    // The same Red victory is a defeat for the first viewer and a victory
    // for the second; compare each border with its winning side's score.
    const rgb = (paint) =>
      paint
        .match(/[\d.]+/g)
        .slice(0, 3)
        .join(",");
    ctx.check(
      "result: the winning side colours the border relative to the commander",
      rgb(paints[0].border) === rgb(paints[0].enemy) &&
        rgb(paints[1].border) === rgb(paints[1].own) &&
        rgb(paints[0].border) !== rgb(paints[1].border),
      JSON.stringify(paints),
    );
    // One match seen from both seats: what is first for one is second for
    // the other, in the score and in the scoreboards alike.
    const flipped = (list) => [...list].reverse().join("|");
    ctx.check(
      "result: each commander's own score and own side come first, as in the battle's strip",
      paints[0].numbers[0] !== paints[0].numbers[1] &&
        paints[1].numbers.join("|") === flipped(paints[0].numbers) &&
        paints[0].ownFirst &&
        paints[1].ownFirst &&
        paints[2].numbers.join("|") === paints[0].numbers.join("|"),
      JSON.stringify(paints.map(({ numbers, ownFirst }) => ({ numbers, ownFirst }))),
    );
    // A commander's sides are headed by what they are to them, the same
    // words from either seat; a watcher's by their names.
    ctx.check(
      "result: both commanders read the same headings, and a watcher reads the sides' names",
      paints[0].headings.join("|") === paints[1].headings.join("|") &&
        paints[2].headings.join("|") !== paints[0].headings.join("|") &&
        paints[0].headings[0] !== paints[0].headings[1],
      JSON.stringify(paints.map(({ headings }) => headings)),
    );
    ctx.check(
      "result: scoreboard headings agree with each viewer's final scores",
      paints.every((p) => p.ownHeading === p.own && p.enemyHeading === p.enemy),
      JSON.stringify(paints),
    );
    ctx.check(
      "result: a spectator keeps the colours by side name",
      paints[2].own === paints[0].own &&
        paints[2].enemy === paints[0].enemy &&
        paints[2].border === paints[0].border,
      JSON.stringify(paints),
    );
    await writeFile(`${evidence}result-colours.json`, JSON.stringify(paints, null, 2));
  } finally {
    for (const page of pages) await page.close();
  }
}
