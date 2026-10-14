// The faction air matrix and the runtime models (slice 09). The panel lists,
// for each faction, the roster's short, medium and long air defence, its
// SEAD jets and its other fixed-wing aircraft, read from the cards as the
// catalog resolves them; every faction has all three AA tiers and a SEAD
// platform. The battle parks each of those types on the air field, and each
// is drawn; a contact sheet of them is left for review.
import { gpuWarnings, aim, lab, hideHud } from "./_lab.mjs";
import { encounter, openAir, shoot, drawnVehicles } from "./_air.mjs";
import { writeSheet } from "./_sheet.mjs";
import { layerOf } from "../../packages/scene-assets/src/units.ts";
import { unitType, weapons } from "./_units.mjs";
import { nodeCatalogSet } from "../src/battle/catalog/node.ts";
import { AIR_ROLES, airRole, FACTIONS } from "../../packages/scene-assets/src/units.ts";
const cards = (await nodeCatalogSet("test")).units.view.cards.filter((c) => !c.disabled_reason);
const matrix = Object.fromEntries(
  FACTIONS.map((f) => [
    f,
    Object.fromEntries(
      AIR_ROLES.map((r) => [
        r,
        cards
          .filter((c) => c.roster.factions.includes(f) && airRole(unitType(c.id)) === r)
          .map((c) => c.name),
      ]),
    ),
  ]),
);

export async function run(ctx) {
  await recordsPanel(ctx);
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const shown = await page.evaluate(() =>
    Object.fromEntries(
      [...document.querySelectorAll("[data-testid=air-matrix] [data-faction]")].map((s) => [
        s.getAttribute("data-faction"),
        Object.fromEntries(
          [...s.querySelectorAll("[data-role]")].map((r) => [
            r.getAttribute("data-role"),
            r.textContent.slice(r.textContent.indexOf(":") + 2),
          ]),
        ),
      ]),
    ),
  );
  for (const f of FACTIONS) {
    const want = Object.fromEntries(AIR_ROLES.map((r) => [r, matrix[f][r].join(", ") || "—"]));
    ctx.check(
      `${f}: the matrix lists the roster's air roles`,
      JSON.stringify(shown[f]) === JSON.stringify(want),
      JSON.stringify({ shown: shown[f], want }),
    );
    ctx.check(
      `${f}: short, medium and long air defence and a SEAD platform`,
      ["short", "medium", "long", "sead"].every((r) => matrix[f][r].length > 0),
      JSON.stringify(matrix[f]),
    );
  }

  // The line-up: one of each type in the matrix, every one drawn.
  const o = await lab(page, () => window.__lab.route.observation());
  const kinds = new Set(cards.filter((c) => airRole(unitType(c.id))).map((c) => c.id));
  ctx.check(
    "the battle parks each type of the matrix once",
    o.own.length === kinds.size && o.own.every((u) => kinds.has(u.kind)),
    `${o.own.length} units, ${kinds.size} types`,
  );
  await shoot(ctx, page, "lineup");
  // The crops show the models alone: no HUD, cards or panel.
  await hideHud(page);
  await page.addStyleTag({ content: "[data-testid=air-matrix] { display: none !important; }" });
  const cells = [];
  const undrawn = [];
  for (const u of o.own) {
    await aim(
      page,
      u.position,
      { distance: 32, pitch: 0.45, yaw: -Math.PI / 2 - 0.6 },
      { onGround: false },
    );
    const drawn = await drawnVehicles(page);
    if (
      !drawn.some(
        (v) => Math.hypot(v.position[0] - u.position[0], v.position[1] - u.position[1]) < 2,
      )
    )
      undrawn.push(u.kind);
    await page.evaluate(() => window.__lab.frame());
    const png = await page.screenshot({ clip: { x: 340, y: 200, width: 600, height: 400 } });
    cells.push({ png, caption: `${unitType(u.kind).name} (${u.kind})` });
  }
  ctx.check("every parked type is drawn", undrawn.length === 0, undrawn.join(", "));
  await writeSheet(ctx, "contact-sheet.png", {
    title: "air-roster: the roster's air and air-defence types on their runtime models",
    columns: 5,
    cellWidth: 300,
    cells,
  });
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}

async function recordsPanel(ctx) {
  const page = await openAir(ctx, "records");
  const kinds = [...new Set(encounter("contracts", "air-roster").units.map((u) => u.kind))];
  // The panel: one record per fielded type, its fields the catalog's.
  const records = await page.evaluate(() =>
    [...document.querySelectorAll("[data-testid=air-records] [data-kind]")].map((s) => ({
      kind: s.getAttribute("data-kind"),
      fields: Object.fromEntries(
        [...s.querySelectorAll(".lab-row > span[data-field]")].map((f) => [
          f.getAttribute("data-field"),
          f.textContent.slice(f.getAttribute("data-field").length + 1),
        ]),
      ),
      weapons: Object.fromEntries(
        [...s.querySelectorAll("[data-weapon]")].map((w) => [
          w.getAttribute("data-weapon"),
          w.querySelector("[data-field=targets]").textContent,
        ]),
      ),
    })),
  );
  ctx.check(
    "the panel prints one record per fielded type",
    JSON.stringify(records.map((r) => r.kind).sort()) === JSON.stringify([...kinds].sort()),
    JSON.stringify(records.map((r) => r.kind)),
  );
  for (const kind of kinds) {
    const t = unitType(kind);
    const rec = records.find((r) => r.kind === kind);
    const caps = t.capabilities;
    const radar = caps.air?.radar ?? caps.radar;
    const want = {
      layer: layerOf(t),
      fuel: caps.air ? String(caps.air.fuel_capacity_s) : "—",
      aa_tier: caps.air_defense?.tier ?? "—",
      radar: radar ? `${radar.emits ? "emits" : "silent"}/${radar.detection_m}` : "—",
      sead: caps.air?.sead ? "yes" : "no",
    };
    const targets = Object.fromEntries(
      t.mounts
        .flatMap((m) => m.weapons)
        .map((w) => [w, Object.keys(weapons[w].targets ?? { ground: {} }).join("+")]),
    );
    ctx.check(
      `${kind}: its record reads the catalog's layer, fuel, tier, radar and weapon layers`,
      !!rec &&
        Object.entries(want).every(([k, v]) => rec.fields[k] === v) &&
        JSON.stringify(rec.weapons) === JSON.stringify(targets),
      JSON.stringify({ rec, want, targets }),
    );
  }

  await page.close();
}
